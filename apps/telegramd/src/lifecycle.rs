//! Bounded daemon lifecycle поверх одного daemon-owned `CoreRuntime`.

use std::fmt;
use std::io;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use telegram_core::NativeTdJson;
use telegram_core::authorization::{AuthorizationError, ChallengeId};
use telegram_core::database_key::DatabaseKey;
use telegram_core::reducer::CachedUpdateKind;
use telegram_core::registry::AccountKind;
use telegram_core::runtime::{CoreRuntime, CoreRuntimeEvent, RuntimeError};
use telegram_core::transport::TransportError;

use crate::authorization::{AuthorizationCoordinator, AuthorizationObservation};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthorizationReadiness {
    Ready,
    InteractiveRequired,
    ExternalShutdown,
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

    pub fn authorization_lost(&mut self) -> Result<(), LifecycleError> {
        match self.state {
            DaemonState::Ready => {
                self.state = DaemonState::Starting;
                self.idle_since = None;
                Ok(())
            }
            DaemonState::Starting => Ok(()),
            DaemonState::Stopped | DaemonState::Draining | DaemonState::Closed => {
                Err(LifecycleError::InvalidTransition)
            }
        }
    }

    pub fn authorization_verified(&mut self, now: Instant) -> Result<(), LifecycleError> {
        match self.state {
            DaemonState::Starting => self.ready(now),
            DaemonState::Ready => Ok(()),
            DaemonState::Stopped | DaemonState::Draining | DaemonState::Closed => {
                Err(LifecycleError::InvalidTransition)
            }
        }
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
    authorization: &mut AuthorizationCoordinator,
) -> Result<AuthorizationReadiness, LifecycleError> {
    let deadline = deadline_after(AUTHORIZATION_TIMEOUT)?;
    let mut step = initial_authorization_observation(runtime, authorization)?;

    loop {
        match step {
            AuthorizationObservation::ParametersRequired { generation } => {
                submit_tdlib_parameters(
                    runtime,
                    config,
                    database_key,
                    authorization,
                    generation,
                    deadline,
                )?;
                step = wait_for_authorization(runtime, authorization, deadline)?;
            }
            AuthorizationObservation::ReadyObserved => {
                let account =
                    verify_ready_identity(runtime, ownership, config.expected_user_id(), deadline)?;
                authorization
                    .mark_identity_verified(account)
                    .map_err(LifecycleError::Authorization)?;
                return Ok(AuthorizationReadiness::Ready);
            }
            AuthorizationObservation::InteractiveRequired => {
                return Ok(AuthorizationReadiness::InteractiveRequired);
            }
            AuthorizationObservation::LoggingOut
            | AuthorizationObservation::Closing
            | AuthorizationObservation::Closed => {
                return Ok(AuthorizationReadiness::ExternalShutdown);
            }
        }
    }
}

fn initial_authorization_observation(
    runtime: &CoreRuntime,
    authorization: &mut AuthorizationCoordinator,
) -> Result<AuthorizationObservation, LifecycleError> {
    let initial = runtime
        .state()
        .authorization()
        .ok_or(LifecycleError::MissingAuthorizationState)?;
    authorization
        .observe(&initial.value, Instant::now())
        .map_err(LifecycleError::Authorization)
}

fn submit_tdlib_parameters(
    runtime: &mut CoreRuntime,
    config: &DaemonConfig,
    database_key: &DatabaseKey,
    authorization: &mut AuthorizationCoordinator,
    generation: ChallengeId,
    deadline: Instant,
) -> Result<(), LifecycleError> {
    let request = authorization
        .submit_parameters(generation, config.tdlib_parameters(), database_key)
        .map_err(LifecycleError::Authorization)?;
    let response = runtime
        .transport()
        .call_until(request.into_value(), deadline)
        .map_err(LifecycleError::Transport)?;
    if response.get("@type").and_then(Value::as_str) == Some("ok") {
        return Ok(());
    }
    let code = response
        .get("code")
        .and_then(Value::as_i64)
        .and_then(|code| i32::try_from(code).ok())
        .unwrap_or_default();
    authorization
        .parameters_failed(generation, code)
        .map_err(LifecycleError::Authorization)?;
    Err(LifecycleError::TdlibParametersRejected)
}

pub fn serve_until_authorized(
    runtime: &mut CoreRuntime,
    socket: &DaemonSocket,
    server: &mut LeaseServer,
    ownership: &ProfileDatabaseLock,
    expected_user_id: Option<i64>,
) -> Result<AuthorizationReadiness, LifecycleError> {
    configure_socket(socket)?;
    loop {
        let now = Instant::now();
        server
            .poll(socket.listener(), runtime, now)
            .map_err(LifecycleError::Server)?;
        let event_deadline = now.checked_add(READY_POLL).unwrap_or(now);
        match poll_authorization_event(runtime, server, event_deadline)? {
            AuthorizationEvent::Ready => {
                verify_server_identity(runtime, server, ownership, expected_user_id)?;
                return Ok(AuthorizationReadiness::Ready);
            }
            AuthorizationEvent::Lost => ensure_authorization_active(runtime)?,
            AuthorizationEvent::ExternalShutdown => {
                return Ok(AuthorizationReadiness::ExternalShutdown);
            }
            AuthorizationEvent::None => {}
        }
    }
}

fn configure_socket(socket: &DaemonSocket) -> Result<(), LifecycleError> {
    socket
        .listener()
        .set_nonblocking(true)
        .map_err(|error| LifecycleError::SocketMode(error.kind()))
}

#[derive(Clone, Copy)]
enum AuthorizationEvent {
    None,
    Ready,
    Lost,
    ExternalShutdown,
}

fn poll_authorization_event(
    runtime: &mut CoreRuntime,
    server: &mut LeaseServer,
    deadline: Instant,
) -> Result<AuthorizationEvent, LifecycleError> {
    match runtime.next_event_until(deadline) {
        Ok(CoreRuntimeEvent::State(applied)) => {
            server.record_event(applied);
            if applied.kind != CachedUpdateKind::Authorization {
                return Ok(AuthorizationEvent::None);
            }
            let ready = server
                .observe_authorization(runtime, Instant::now())
                .map_err(LifecycleError::Server)?;
            Ok(if ready {
                AuthorizationEvent::Ready
            } else if external_shutdown_state(authorization_type(runtime)) {
                AuthorizationEvent::ExternalShutdown
            } else {
                AuthorizationEvent::Lost
            })
        }
        Ok(CoreRuntimeEvent::UnmatchedResponse { extra, response }) => {
            if let Some(correlation_id) = extra.as_u64() {
                server
                    .reconcile_authorization_response(correlation_id, &response)
                    .map_err(LifecycleError::Server)?;
            }
            Ok(AuthorizationEvent::None)
        }
        Err(RuntimeError::DeadlineExceeded) => Ok(AuthorizationEvent::None),
        Err(error) => Err(LifecycleError::Runtime(error)),
    }
}

fn verify_server_identity(
    runtime: &mut CoreRuntime,
    server: &mut LeaseServer,
    ownership: &ProfileDatabaseLock,
    expected_user_id: Option<i64>,
) -> Result<(), LifecycleError> {
    let deadline = deadline_after(AUTHORIZATION_TIMEOUT)?;
    let account = verify_ready_identity(runtime, ownership, expected_user_id, deadline)?;
    server
        .mark_identity_verified(account)
        .map_err(LifecycleError::Server)
}

fn ensure_authorization_active(runtime: &CoreRuntime) -> Result<(), LifecycleError> {
    if external_shutdown_state(authorization_type(runtime)) {
        Err(LifecycleError::UnexpectedAuthorizationState)
    } else {
        Ok(())
    }
}

fn external_shutdown_state(state: Option<&str>) -> bool {
    matches!(
        state,
        Some(
            "authorizationStateLoggingOut"
                | "authorizationStateClosing"
                | "authorizationStateClosed"
        )
    )
}

fn verify_ready_identity(
    runtime: &mut CoreRuntime,
    ownership: &ProfileDatabaseLock,
    expected_user_id: Option<i64>,
    deadline: Instant,
) -> Result<AccountKind, LifecycleError> {
    let response = runtime
        .transport()
        .call_until(json!({"@type":"getMe"}), deadline)
        .map_err(LifecycleError::Transport)?;
    let (actual_user_id, account) =
        identity::account_from_get_me(&response).map_err(LifecycleError::Identity)?;
    identity::verify_or_bind(
        ownership.canonical_database_directory(),
        actual_user_id,
        expected_user_id,
    )
    .map_err(LifecycleError::Identity)?;
    Ok(account)
}

fn wait_for_authorization(
    runtime: &mut CoreRuntime,
    authorization: &mut AuthorizationCoordinator,
    deadline: Instant,
) -> Result<AuthorizationObservation, LifecycleError> {
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
                return authorization
                    .observe(&state.value, Instant::now())
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
    ownership: &ProfileDatabaseLock,
    expected_user_id: Option<i64>,
) -> Result<(), LifecycleError> {
    configure_socket(&socket)?;

    loop {
        let now = Instant::now();
        server
            .poll(socket.listener(), &mut runtime, now)
            .map_err(LifecycleError::Server)?;
        let event_deadline = now.checked_add(READY_POLL).unwrap_or(now);
        let event = poll_authorization_event(&mut runtime, &mut server, event_deadline)?;
        if matches!(event, AuthorizationEvent::ExternalShutdown) {
            eprintln!("telegramd: external authorization shutdown; waiting for close");
            drop(socket);
            finish_external_shutdown(runtime)?;
            lifecycle.begin_draining()?;
            return lifecycle.closed();
        }
        update_authorization_lifecycle(
            event,
            &mut runtime,
            &mut server,
            lifecycle,
            ownership,
            expected_user_id,
        )?;

        let has_activity = server.active_leases() != 0;
        if lifecycle.idle_elapsed(Instant::now(), has_activity) {
            lifecycle.begin_draining()?;
            eprintln!("telegramd: Draining");
            break;
        }
    }

    drop(socket);
    graceful_close(runtime)?;
    lifecycle.closed()
}

fn update_authorization_lifecycle(
    event: AuthorizationEvent,
    runtime: &mut CoreRuntime,
    server: &mut LeaseServer,
    lifecycle: &mut Lifecycle,
    ownership: &ProfileDatabaseLock,
    expected_user_id: Option<i64>,
) -> Result<(), LifecycleError> {
    match event {
        AuthorizationEvent::Ready => {
            verify_server_identity(runtime, server, ownership, expected_user_id)?;
            lifecycle.authorization_verified(Instant::now())
        }
        AuthorizationEvent::Lost => {
            lifecycle.authorization_lost()?;
            ensure_authorization_active(runtime)
        }
        AuthorizationEvent::None | AuthorizationEvent::ExternalShutdown => Ok(()),
    }
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

pub fn finish_external_shutdown(mut runtime: CoreRuntime) -> Result<(), LifecycleError> {
    let deadline = deadline_after(CLOSE_TIMEOUT)?;
    while authorization_type(&runtime) != Some("authorizationStateClosed") {
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => break,
            Err(error) => return Err(LifecycleError::Runtime(error)),
        }
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
    use std::collections::VecDeque;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{SystemTime, UNIX_EPOCH};

    use telegram_core::idempotency::IdempotencyJournal;
    use telegram_core::transport::{BackendError, TdJsonBackend};
    use telegram_protocol::RiskScope;

    use crate::lease::LeaseManager;
    use crate::ownership::ProfileDatabaseLock;
    use crate::scheduler::{AccountScheduler, serial_daemon_budgets};
    use crate::socket::DaemonSocket;
    use crate::telemetry::{AuditLog, Telemetry};

    use super::*;

    #[derive(Clone)]
    struct ExternalLogoutBackendState(Arc<Mutex<ExternalLogoutBackendInner>>);

    struct ExternalLogoutBackendInner {
        incoming: VecDeque<String>,
        sent_types: Vec<String>,
    }

    struct ExternalLogoutBackend(ExternalLogoutBackendState);

    impl ExternalLogoutBackend {
        fn new() -> (Self, ExternalLogoutBackendState) {
            let state =
                ExternalLogoutBackendState(Arc::new(Mutex::new(ExternalLogoutBackendInner {
                    incoming: VecDeque::new(),
                    sent_types: Vec::new(),
                })));
            (Self(state.clone()), state)
        }
    }

    impl TdJsonBackend for ExternalLogoutBackend {
        fn send(&mut self, request: &str) -> Result<(), BackendError> {
            let request: Value = serde_json::from_str(request)
                .map_err(|error| BackendError::new(error.to_string()))?;
            let request_type = request
                .get("@type")
                .and_then(Value::as_str)
                .ok_or_else(|| BackendError::new("missing request type"))?;
            let extra = request
                .get("@extra")
                .cloned()
                .ok_or_else(|| BackendError::new("missing request correlation"))?;
            let mut inner = self.0.0.lock().unwrap();
            inner.sent_types.push(request_type.to_owned());
            match request_type {
                "setLogStream" => inner
                    .incoming
                    .push_back(json!({"@type":"ok","@extra":extra}).to_string()),
                "getOption" => {
                    let manifest: Value =
                        serde_json::from_str(include_str!("../../../vendor/tdlib/manifest.json"))
                            .map_err(|error| BackendError::new(error.to_string()))?;
                    let option = request.get("name").and_then(Value::as_str);
                    let value = match option {
                        Some("version") => manifest["upstream"]["version"].clone(),
                        Some("commit_hash") => manifest["upstream"]["commit"].clone(),
                        _ => return Err(BackendError::new("unexpected runtime option")),
                    };
                    inner.incoming.push_back(
                        json!({"@type":"optionValueString","value":value,"@extra":extra})
                            .to_string(),
                    );
                }
                "getCurrentState" => {
                    inner.incoming.push_back(
                        json!({
                            "@type":"updates",
                            "updates":[{
                                "@type":"updateAuthorizationState",
                                "authorization_state":{"@type":"authorizationStateReady"}
                            }],
                            "@extra":extra
                        })
                        .to_string(),
                    );
                    inner.incoming.extend([
                        json!({
                            "@type":"updateAuthorizationState",
                            "authorization_state":{"@type":"authorizationStateLoggingOut"}
                        })
                        .to_string(),
                        json!({
                            "@type":"updateAuthorizationState",
                            "authorization_state":{"@type":"authorizationStateClosed"}
                        })
                        .to_string(),
                    ]);
                }
                _ => return Err(BackendError::new("unexpected TDLib request")),
            }
            Ok(())
        }

        fn receive(&mut self, timeout: Duration) -> Result<Option<String>, BackendError> {
            let value = self.0.0.lock().unwrap().incoming.pop_front();
            if value.is_none() {
                thread::sleep(timeout.min(Duration::from_millis(1)));
            }
            Ok(value)
        }
    }

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

    #[test]
    fn auth_loss_suspends_idle_shutdown_until_identity_is_verified_again() {
        let start = Instant::now();
        let mut lifecycle = Lifecycle::new(Duration::from_millis(10));
        lifecycle.start().unwrap();
        lifecycle.ready(start).unwrap();
        lifecycle.authorization_lost().unwrap();
        assert_eq!(lifecycle.state, DaemonState::Starting);
        assert!(!lifecycle.idle_elapsed(start + Duration::from_secs(1), false));

        let verified_at = start + Duration::from_secs(2);
        lifecycle.authorization_verified(verified_at).unwrap();
        assert_eq!(lifecycle.state, DaemonState::Ready);
        assert!(!lifecycle.idle_elapsed(verified_at, false));
        assert!(lifecycle.idle_elapsed(verified_at + Duration::from_millis(10), false));
    }

    #[test]
    fn external_logout_reaches_closed_without_sending_close() {
        let (backend, backend_state) = ExternalLogoutBackend::new();
        let runtime = CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let root = std::env::temp_dir().join(format!(
            "telegramd-lifecycle-external-logout-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let ownership = ProfileDatabaseLock::acquire("external-logout".to_owned(), &root).unwrap();
        let socket = DaemonSocket::bind(&ownership).unwrap();
        let telemetry = Telemetry::default();
        let scheduler =
            AccountScheduler::with_telemetry(serial_daemon_budgets(), telemetry.clone()).unwrap();
        let server = LeaseServer::new(
            LeaseManager::with_telemetry([RiskScope::Read], telemetry.clone()),
            scheduler,
            telemetry,
            IdempotencyJournal::open(root.join("idempotency.jsonl")).unwrap(),
            AuditLog::open(root.join("audit.jsonl")).unwrap(),
            AuthorizationCoordinator::with_epoch(1),
        );
        let mut lifecycle = Lifecycle::new(Duration::from_secs(60));
        lifecycle.start().unwrap();
        lifecycle.ready(Instant::now()).unwrap();

        serve_until_idle(runtime, socket, server, &mut lifecycle, &ownership, None).unwrap();

        assert_eq!(lifecycle.state, DaemonState::Closed);
        assert!(
            !backend_state
                .0
                .lock()
                .unwrap()
                .sent_types
                .contains(&"close".to_owned())
        );
        drop(ownership);
        fs::remove_dir_all(root).unwrap();
    }
}
