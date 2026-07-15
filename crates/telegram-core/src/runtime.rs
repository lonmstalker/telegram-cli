//! Startup handshake и ordered runtime driver поверх TDJSON transport.

use std::fmt;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

use serde_json::{Value, json};

use crate::reducer::{AppliedUpdate, ReducerError, StateReducer};
use crate::transport::{TdJsonBackend, TdJsonEvent, TdJsonTransport, TransportError};

const PINNED_MANIFEST: &str = include_str!("../../../vendor/tdlib/manifest.json");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdentity {
    version: String,
    commit: String,
}

impl RuntimeIdentity {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn commit(&self) -> &str {
        &self.commit
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoreRuntimeEvent {
    State(AppliedUpdate),
    UnmatchedResponse { extra: Value, response: Value },
}

pub struct CoreRuntime {
    transport: TdJsonTransport,
    events: Receiver<TdJsonEvent>,
    reducer: StateReducer,
    identity: RuntimeIdentity,
}

impl CoreRuntime {
    pub fn start<B: TdJsonBackend>(backend: B, deadline: Instant) -> Result<Self, RuntimeError> {
        let (transport, events) =
            TdJsonTransport::start(backend).map_err(RuntimeError::Transport)?;
        Self::initialize(transport, events, deadline)
    }

    pub fn initialize(
        transport: TdJsonTransport,
        events: Receiver<TdJsonEvent>,
        deadline: Instant,
    ) -> Result<Self, RuntimeError> {
        let expected = pinned_identity()?;
        disable_internal_logging(&transport, deadline)?;
        verify_option(&transport, "version", &expected.version, deadline)?;
        verify_option(&transport, "commit_hash", &expected.commit, deadline)?;

        let (snapshot, boundary) =
            startup_call(&transport, json!({"@type":"getCurrentState"}), deadline)?;
        let snapshot_updates = snapshot
            .as_object()
            .filter(|object| object.get("@type") == Some(&Value::String("updates".to_owned())))
            .and_then(|object| object.get("updates"))
            .and_then(Value::as_array)
            .ok_or(RuntimeError::InvalidStartupResponse("getCurrentState"))?;

        discard_through_boundary(&events, boundary, deadline)?;
        let mut reducer = StateReducer::default();
        for update in snapshot_updates {
            reducer.apply(update).map_err(RuntimeError::Reducer)?;
        }
        Ok(Self {
            transport,
            events,
            reducer,
            identity: expected,
        })
    }

    pub fn identity(&self) -> &RuntimeIdentity {
        &self.identity
    }

    pub fn state(&self) -> &StateReducer {
        &self.reducer
    }

    pub fn transport(&self) -> &TdJsonTransport {
        &self.transport
    }

    pub fn next_event_until(
        &mut self,
        deadline: Instant,
    ) -> Result<CoreRuntimeEvent, RuntimeError> {
        loop {
            let remaining = remaining(deadline)?;
            match self.events.recv_timeout(remaining) {
                Ok(TdJsonEvent::Update(update)) => {
                    return self
                        .reducer
                        .apply(&update)
                        .map(CoreRuntimeEvent::State)
                        .map_err(RuntimeError::Reducer);
                }
                Ok(TdJsonEvent::ResponseBoundary { .. }) => {}
                Ok(TdJsonEvent::UnmatchedResponse { extra, response }) => {
                    return Ok(CoreRuntimeEvent::UnmatchedResponse { extra, response });
                }
                Ok(TdJsonEvent::Fatal(error)) => return Err(RuntimeError::Transport(error)),
                Err(RecvTimeoutError::Timeout) => return Err(RuntimeError::DeadlineExceeded),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(RuntimeError::EventStreamClosed);
                }
            }
        }
    }

    pub fn shutdown(self) -> Result<(), RuntimeError> {
        self.transport.shutdown().map_err(RuntimeError::Transport)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    InvalidPinnedManifest,
    RuntimeMismatch {
        option: &'static str,
        expected: String,
        actual: String,
    },
    InvalidStartupResponse(&'static str),
    UnexpectedStartupEvent,
    DeadlineExceeded,
    EventStreamClosed,
    Transport(TransportError),
    Reducer(ReducerError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPinnedManifest => formatter.write_str("pinned TDLib manifest is invalid"),
            Self::RuntimeMismatch {
                option,
                expected,
                actual,
            } => write!(
                formatter,
                "TDLib runtime {option} mismatch: expected {expected}, got {actual}"
            ),
            Self::InvalidStartupResponse(method) => {
                write!(formatter, "invalid TDLib startup response for {method}")
            }
            Self::UnexpectedStartupEvent => {
                formatter.write_str("unexpected unmatched response during TDLib startup")
            }
            Self::DeadlineExceeded => formatter.write_str("TDLib runtime deadline exceeded"),
            Self::EventStreamClosed => formatter.write_str("TDLib event stream closed"),
            Self::Transport(error) => write!(formatter, "TDJSON transport error: {error}"),
            Self::Reducer(error) => write!(formatter, "TDLib reducer error: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

fn pinned_identity() -> Result<RuntimeIdentity, RuntimeError> {
    let manifest: Value =
        serde_json::from_str(PINNED_MANIFEST).map_err(|_| RuntimeError::InvalidPinnedManifest)?;
    let upstream = manifest
        .get("upstream")
        .and_then(Value::as_object)
        .ok_or(RuntimeError::InvalidPinnedManifest)?;
    let version = upstream
        .get("version")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(RuntimeError::InvalidPinnedManifest)?;
    let commit = upstream
        .get("commit")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or(RuntimeError::InvalidPinnedManifest)?;
    Ok(RuntimeIdentity {
        version: version.to_owned(),
        commit: commit.to_owned(),
    })
}

fn verify_option(
    transport: &TdJsonTransport,
    name: &'static str,
    expected: &str,
    deadline: Instant,
) -> Result<(), RuntimeError> {
    let (response, _) = startup_call(
        transport,
        json!({"@type":"getOption", "name":name}),
        deadline,
    )?;
    let actual = response
        .as_object()
        .filter(|object| {
            object.get("@type") == Some(&Value::String("optionValueString".to_owned()))
        })
        .and_then(|object| object.get("value"))
        .and_then(Value::as_str)
        .ok_or(RuntimeError::InvalidStartupResponse("getOption"))?;
    if actual != expected {
        return Err(RuntimeError::RuntimeMismatch {
            option: name,
            expected: expected.to_owned(),
            actual: actual.to_owned(),
        });
    }
    Ok(())
}

fn disable_internal_logging(
    transport: &TdJsonTransport,
    deadline: Instant,
) -> Result<(), RuntimeError> {
    let (response, _) = startup_call(
        transport,
        json!({"@type":"setLogStream", "log_stream":{"@type":"logStreamEmpty"}}),
        deadline,
    )?;
    if response.get("@type").and_then(Value::as_str) == Some("ok") {
        Ok(())
    } else {
        Err(RuntimeError::InvalidStartupResponse("setLogStream"))
    }
}

fn startup_call(
    transport: &TdJsonTransport,
    request: Value,
    deadline: Instant,
) -> Result<(Value, u64), RuntimeError> {
    remaining(deadline)?;
    let pending = transport
        .request(request)
        .map_err(RuntimeError::Transport)?;
    let boundary = pending.correlation_id();
    let response = pending.wait_until(deadline).map_err(map_transport_error)?;
    Ok((response, boundary))
}

fn discard_through_boundary(
    events: &Receiver<TdJsonEvent>,
    boundary: u64,
    deadline: Instant,
) -> Result<(), RuntimeError> {
    loop {
        match events.recv_timeout(remaining(deadline)?) {
            Ok(TdJsonEvent::ResponseBoundary { correlation_id }) if correlation_id == boundary => {
                return Ok(());
            }
            Ok(TdJsonEvent::Update(_) | TdJsonEvent::ResponseBoundary { .. }) => {}
            Ok(TdJsonEvent::UnmatchedResponse { .. }) => {
                return Err(RuntimeError::UnexpectedStartupEvent);
            }
            Ok(TdJsonEvent::Fatal(error)) => return Err(RuntimeError::Transport(error)),
            Err(RecvTimeoutError::Timeout) => return Err(RuntimeError::DeadlineExceeded),
            Err(RecvTimeoutError::Disconnected) => return Err(RuntimeError::EventStreamClosed),
        }
    }
}

fn remaining(deadline: Instant) -> Result<std::time::Duration, RuntimeError> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or(RuntimeError::DeadlineExceeded)
}

fn map_transport_error(error: TransportError) -> RuntimeError {
    match error {
        TransportError::ResponseTimeout => RuntimeError::DeadlineExceeded,
        other => RuntimeError::Transport(other),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::transport::BackendError;

    #[derive(Clone)]
    struct StartupState {
        inner: Arc<Mutex<StartupInner>>,
    }

    struct StartupInner {
        incoming: VecDeque<String>,
        sent_types: Vec<String>,
        version: String,
    }

    struct StartupBackend(StartupState);

    impl StartupBackend {
        fn new(version: String) -> (Self, StartupState) {
            let state = StartupState {
                inner: Arc::new(Mutex::new(StartupInner {
                    incoming: VecDeque::new(),
                    sent_types: Vec::new(),
                    version,
                })),
            };
            (Self(state.clone()), state)
        }
    }

    impl TdJsonBackend for StartupBackend {
        fn send(&mut self, request: &str) -> Result<(), BackendError> {
            let request: Value = serde_json::from_str(request).unwrap();
            let mut inner = self.0.inner.lock().unwrap();
            inner
                .sent_types
                .push(request["@type"].as_str().unwrap().to_owned());
            let extra = request["@extra"].clone();
            match request["@type"].as_str().unwrap() {
                "setLogStream" => {
                    assert_eq!(request["log_stream"]["@type"], "logStreamEmpty");
                    inner
                        .incoming
                        .push_back(json!({"@type":"ok","@extra":extra}).to_string());
                }
                "getOption" if request["name"] == "version" => {
                    inner.incoming.push_back(
                        json!({"@type":"updateNewChat","chat":{"@type":"chat","id":"99"}})
                            .to_string(),
                    );
                    let version = inner.version.clone();
                    inner.incoming.push_back(
                        json!({"@type":"optionValueString","value":version,"@extra":extra})
                            .to_string(),
                    );
                }
                "getOption" => {
                    let commit = pinned_identity().unwrap().commit;
                    inner.incoming.push_back(
                        json!({"@type":"optionValueString","value":commit,"@extra":extra})
                            .to_string(),
                    );
                }
                "getCurrentState" => {
                    inner.incoming.push_back(
                        json!({
                            "@type":"updates",
                            "updates":[
                                {"@type":"updateAuthorizationState","authorization_state":{"@type":"authorizationStateReady"}},
                                {"@type":"updateUser","user":{"@type":"user","id":"7","status":{"@type":"userStatusOffline"}}}
                            ],
                            "@extra":extra
                        })
                        .to_string(),
                    );
                    inner.incoming.push_back(
                        json!({"@type":"updateUserStatus","user_id":"7","status":{"@type":"userStatusOnline","expires":10}})
                            .to_string(),
                    );
                }
                _ => unreachable!(),
            }
            Ok(())
        }

        fn receive(&mut self, timeout: Duration) -> Result<Option<String>, BackendError> {
            let value = self.0.inner.lock().unwrap().incoming.pop_front();
            if value.is_none() {
                thread::sleep(timeout.min(Duration::from_millis(1)));
            }
            Ok(value)
        }
    }

    #[test]
    fn startup_handshake_discards_pre_snapshot_updates_and_applies_later_events() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version.clone());
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        assert_eq!(runtime.identity(), &identity);
        assert!(runtime.state().chat(99).is_none());
        assert_eq!(
            runtime.state().user(7).unwrap().value["status"]["@type"],
            "userStatusOffline"
        );
        assert!(matches!(
            runtime
                .next_event_until(Instant::now() + Duration::from_secs(1))
                .unwrap(),
            CoreRuntimeEvent::State(_)
        ));
        assert_eq!(
            runtime.state().user(7).unwrap().value["status"]["@type"],
            "userStatusOnline"
        );
        assert_eq!(
            state.inner.lock().unwrap().sent_types,
            ["setLogStream", "getOption", "getOption", "getCurrentState"]
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn runtime_mismatch_stops_before_snapshot() {
        let (backend, state) = StartupBackend::new("wrong-version".to_owned());
        assert!(matches!(
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)),
            Err(RuntimeError::RuntimeMismatch {
                option: "version",
                ..
            })
        ));
        assert_eq!(
            state.inner.lock().unwrap().sent_types,
            ["setLogStream", "getOption"]
        );
    }
}
