//! Bounded daemon lifecycle поверх одного daemon-owned `CoreRuntime`.

use std::fmt;
use std::io;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use telegram_core::NativeTdJson;
use telegram_core::authorization::{AuthorizationError, AuthorizationMachine, AuthorizationStep};
use telegram_core::database_key::DatabaseKey;
use telegram_core::reducer::CachedUpdateKind;
use telegram_core::runtime::{CoreRuntime, CoreRuntimeEvent, RuntimeError};
use telegram_core::transport::TransportError;

use crate::config::DaemonConfig;
use crate::identity::{self, IdentityError};
use crate::ownership::ProfileDatabaseLock;
use crate::server::{LeaseServer, ServerError};
use crate::socket::DaemonSocket;

const AUTHORIZATION_TIMEOUT: Duration = Duration::from_secs(30);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(30);
const READY_POLL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonState {
    Stopped,
    Starting,
    Ready,
    Draining,
    Closed,
}

pub struct Lifecycle {
    state: DaemonState,
    idle_timeout: Duration,
    idle_since: Option<Instant>,
}

impl Lifecycle {
    pub fn new(idle_timeout: Duration) -> Self {
        Self {
            state: DaemonState::Stopped,
            idle_timeout,
            idle_since: None,
        }
    }

    pub fn start(&mut self) -> Result<(), LifecycleError> {
        self.transition(DaemonState::Stopped, DaemonState::Starting)
    }

    pub fn ready(&mut self, now: Instant) -> Result<(), LifecycleError> {
        self.transition(DaemonState::Starting, DaemonState::Ready)?;
        self.idle_since = Some(now);
        Ok(())
    }

    pub fn idle_elapsed(&mut self, now: Instant, has_activity: bool) -> bool {
        if self.state != DaemonState::Ready {
            return false;
        }
        if has_activity {
            self.idle_since = None;
            return false;
        }
        let idle_since = self.idle_since.get_or_insert(now);
        now.saturating_duration_since(*idle_since) >= self.idle_timeout
    }

    pub fn begin_draining(&mut self) -> Result<(), LifecycleError> {
        self.transition(DaemonState::Ready, DaemonState::Draining)
    }

    pub fn closed(&mut self) -> Result<(), LifecycleError> {
        self.transition(DaemonState::Draining, DaemonState::Closed)
    }

    fn transition(
        &mut self,
        expected: DaemonState,
        next: DaemonState,
    ) -> Result<(), LifecycleError> {
        if self.state != expected {
            return Err(LifecycleError::InvalidTransition);
        }
        self.state = next;
        Ok(())
    }
}

pub fn start_runtime(backend: NativeTdJson) -> Result<CoreRuntime, LifecycleError> {
    let deadline = deadline_after(AUTHORIZATION_TIMEOUT)?;
    CoreRuntime::start(backend, deadline).map_err(LifecycleError::Runtime)
}

pub fn reach_ready(
    runtime: &mut CoreRuntime,
    config: &DaemonConfig,
    database_key: &DatabaseKey,
    ownership: &ProfileDatabaseLock,
) -> Result<(), LifecycleError> {
    let deadline = deadline_after(AUTHORIZATION_TIMEOUT)?;
    let mut machine = AuthorizationMachine::default();
    let initial = runtime
        .state()
        .authorization()
        .ok_or(LifecycleError::MissingAuthorizationState)?;
    let mut step = machine
        .observe_state(&initial.value)
        .map_err(LifecycleError::Authorization)?;

    loop {
        match step {
            AuthorizationStep::ParametersRequired { generation } => {
                let request = machine
                    .submit_parameters(generation, config.tdlib_parameters(), database_key)
                    .map_err(LifecycleError::Authorization)?;
                let response = runtime
                    .transport()
                    .call_until(request.into_value(), deadline)
                    .map_err(LifecycleError::Transport)?;
                if response.get("@type").and_then(Value::as_str) != Some("ok") {
                    let code = response
                        .get("code")
                        .and_then(Value::as_i64)
                        .and_then(|code| i32::try_from(code).ok())
                        .unwrap_or_default();
                    machine
                        .parameters_failed(generation, code)
                        .map_err(LifecycleError::Authorization)?;
                    return Err(LifecycleError::TdlibParametersRejected);
                }
                step = wait_for_authorization(runtime, &mut machine, deadline)?;
            }
            AuthorizationStep::Ready => {
                let response = runtime
                    .transport()
                    .call_until(json!({"@type":"getMe"}), deadline)
                    .map_err(LifecycleError::Transport)?;
                let actual_user_id =
                    identity::user_id_from_get_me(&response).map_err(LifecycleError::Identity)?;
                identity::verify_or_bind(
                    ownership.canonical_database_directory(),
                    actual_user_id,
                    config.expected_user_id(),
                )
                .map_err(LifecycleError::Identity)?;
                return Ok(());
            }
            AuthorizationStep::Challenge(_) => {
                return Err(LifecycleError::InteractiveAuthorizationRequired);
            }
            AuthorizationStep::LoggingOut
            | AuthorizationStep::Closing
            | AuthorizationStep::Closed => {
                return Err(LifecycleError::UnexpectedAuthorizationState);
            }
        }
    }
}

fn wait_for_authorization(
    runtime: &mut CoreRuntime,
    machine: &mut AuthorizationMachine,
    deadline: Instant,
) -> Result<AuthorizationStep, LifecycleError> {
    loop {
        match runtime
            .next_event_until(deadline)
            .map_err(LifecycleError::Runtime)?
        {
            CoreRuntimeEvent::State(applied) if applied.kind == CachedUpdateKind::Authorization => {
                let state = runtime
                    .state()
                    .authorization()
                    .ok_or(LifecycleError::MissingAuthorizationState)?;
                return machine
                    .observe_state(&state.value)
                    .map_err(LifecycleError::Authorization);
            }
            CoreRuntimeEvent::State(_) | CoreRuntimeEvent::UnmatchedResponse { .. } => {}
        }
    }
}

pub fn serve_until_idle(
    mut runtime: CoreRuntime,
    socket: DaemonSocket,
    mut server: LeaseServer,
    lifecycle: &mut Lifecycle,
) -> Result<(), LifecycleError> {
    socket
        .listener()
        .set_nonblocking(true)
        .map_err(|error| LifecycleError::SocketMode(error.kind()))?;

    loop {
        let now = Instant::now();
        server
            .poll(socket.listener(), now)
            .map_err(LifecycleError::Server)?;
        // P2 has no workflow dispatcher; P4 adds its real activity source here.
        let has_activity = server.active_leases() != 0;
        if lifecycle.idle_elapsed(now, has_activity) {
            lifecycle.begin_draining()?;
            eprintln!("telegramd: Draining");
            break;
        }

        let event_deadline = now.checked_add(READY_POLL).unwrap_or(now);
        match runtime.next_event_until(event_deadline) {
            Ok(CoreRuntimeEvent::State(_) | CoreRuntimeEvent::UnmatchedResponse { .. }) => {}
            Err(RuntimeError::DeadlineExceeded) => {}
            Err(error) => return Err(LifecycleError::Runtime(error)),
        }
    }

    drop(socket);
    graceful_close(runtime)?;
    lifecycle.closed()
}

fn graceful_close(mut runtime: CoreRuntime) -> Result<(), LifecycleError> {
    let deadline = deadline_after(CLOSE_TIMEOUT)?;
    let response = runtime
        .transport()
        .call_until(json!({"@type":"close"}), deadline)
        .map_err(LifecycleError::Transport)?;
    if response.get("@type").and_then(Value::as_str) != Some("ok") {
        return Err(LifecycleError::CloseRejected);
    }

    while authorization_type(&runtime) != Some("authorizationStateClosed") {
        runtime
            .next_event_until(deadline)
            .map_err(LifecycleError::Runtime)?;
    }
    runtime.shutdown().map_err(LifecycleError::Runtime)
}

fn authorization_type(runtime: &CoreRuntime) -> Option<&str> {
    runtime
        .state()
        .authorization()?
        .value
        .get("@type")?
        .as_str()
}

fn deadline_after(timeout: Duration) -> Result<Instant, LifecycleError> {
    Instant::now()
        .checked_add(timeout)
        .ok_or(LifecycleError::DeadlineOverflow)
}

#[derive(Debug)]
pub enum LifecycleError {
    InvalidTransition,
    DeadlineOverflow,
    MissingAuthorizationState,
    Authorization(AuthorizationError),
    Transport(TransportError),
    Runtime(RuntimeError),
    Identity(IdentityError),
    Server(ServerError),
    SocketMode(io::ErrorKind),
    TdlibParametersRejected,
    InteractiveAuthorizationRequired,
    UnexpectedAuthorizationState,
    CloseRejected,
}

impl fmt::Display for LifecycleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition => formatter.write_str("invalid daemon lifecycle transition"),
            Self::DeadlineOverflow => formatter.write_str("daemon lifecycle deadline overflow"),
            Self::MissingAuthorizationState => {
                formatter.write_str("TDLib authorization state is missing")
            }
            Self::Authorization(error) => write!(formatter, "authorization failed: {error}"),
            Self::Transport(error) => write!(formatter, "TDLib request failed: {error}"),
            Self::Runtime(error) => write!(formatter, "TDLib runtime failed: {error}"),
            Self::Identity(error) => write!(formatter, "{error}"),
            Self::Server(error) => write!(formatter, "{error}"),
            Self::SocketMode(kind) => {
                write!(formatter, "can't configure profile socket: {kind:?}")
            }
            Self::TdlibParametersRejected => {
                formatter.write_str("TDLib parameters or database key were rejected")
            }
            Self::InteractiveAuthorizationRequired => {
                formatter.write_str("protected interactive authorization is required")
            }
            Self::UnexpectedAuthorizationState => {
                formatter.write_str("TDLib entered an unexpected authorization state")
            }
            Self::CloseRejected => formatter.write_str("TDLib rejected graceful close"),
        }
    }
}

impl std::error::Error for LifecycleError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_requires_zero_leases_and_workflows_before_draining() {
        let start = Instant::now();
        let mut lifecycle = Lifecycle::new(Duration::from_millis(10));
        lifecycle.start().unwrap();
        lifecycle.ready(start).unwrap();
        assert!(!lifecycle.idle_elapsed(start + Duration::from_millis(20), true));
        assert!(!lifecycle.idle_elapsed(start + Duration::from_millis(21), false));
        assert!(lifecycle.idle_elapsed(start + Duration::from_millis(31), false));

        lifecycle.begin_draining().unwrap();
        lifecycle.closed().unwrap();
        assert_eq!(lifecycle.state, DaemonState::Closed);
    }
}
