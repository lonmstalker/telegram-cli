//! Однопоточный TDJSON transport и `@extra` correlation.

use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::Value;

const RECEIVE_POLL: Duration = Duration::from_millis(25);

/// Минимальная граница над C TDJSON API. Экземпляр целиком переезжает в
/// единственный receive thread и больше нигде не вызывается.
pub trait TdJsonBackend: Send + 'static {
    fn send(&mut self, request: &str) -> Result<(), BackendError>;
    fn receive(&mut self, timeout: Duration) -> Result<Option<String>, BackendError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendError {
    message: String,
}

impl BackendError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for BackendError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransportError {
    RequestMustBeObject,
    ReservedExtra,
    CorrelationExhausted,
    TransportStopped,
    ResponseTimeout,
    Backend(String),
    InvalidTdJsonResponse,
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequestMustBeObject => formatter.write_str("TDJSON request must be an object"),
            Self::ReservedExtra => {
                formatter.write_str("TDJSON request must not contain reserved @extra")
            }
            Self::CorrelationExhausted => {
                formatter.write_str("TDJSON correlation identifier space is exhausted")
            }
            Self::TransportStopped => formatter.write_str("TDJSON transport is stopped"),
            Self::ResponseTimeout => formatter.write_str("TDJSON response deadline exceeded"),
            Self::Backend(message) => write!(formatter, "TDJSON backend failed: {message}"),
            Self::InvalidTdJsonResponse => {
                formatter.write_str("TDJSON backend returned invalid JSON")
            }
        }
    }
}

impl std::error::Error for TransportError {}

/// Непривязанные к pending request значения из единственного receive loop.
#[derive(Debug, Clone, PartialEq)]
pub enum TdJsonEvent {
    Update(Value),
    ResponseBoundary { correlation_id: u64 },
    UnmatchedResponse { extra: Value, response: Value },
    Fatal(TransportError),
}

pub struct PendingResponse {
    receiver: Receiver<Result<Value, TransportError>>,
    commands: Sender<Command>,
    correlation_id: u64,
    finished: bool,
}

impl PendingResponse {
    pub fn correlation_id(&self) -> u64 {
        self.correlation_id
    }

    pub fn wait_timeout(self, timeout: Duration) -> Result<Value, TransportError> {
        let Some(deadline) = Instant::now().checked_add(timeout) else {
            return Err(TransportError::ResponseTimeout);
        };
        self.wait_until(deadline)
    }

    pub fn wait_until(mut self, deadline: Instant) -> Result<Value, TransportError> {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return Err(TransportError::ResponseTimeout);
        };
        match self.receiver.recv_timeout(remaining) {
            Ok(result) => {
                self.finished = true;
                result
            }
            Err(RecvTimeoutError::Timeout) => Err(TransportError::ResponseTimeout),
            Err(RecvTimeoutError::Disconnected) => {
                self.finished = true;
                Err(TransportError::TransportStopped)
            }
        }
    }

    pub fn cancel(mut self) -> Result<(), TransportError> {
        let (acknowledgement, acknowledged) = mpsc::channel();
        self.commands
            .send(Command::Cancel {
                extra: self.correlation_id,
                acknowledgement: Some(acknowledgement),
            })
            .map_err(|_| TransportError::TransportStopped)?;
        acknowledged
            .recv()
            .map_err(|_| TransportError::TransportStopped)?;
        self.finished = true;
        Ok(())
    }
}

impl Drop for PendingResponse {
    fn drop(&mut self) {
        if !self.finished {
            let _ = self.commands.send(Command::Cancel {
                extra: self.correlation_id,
                acknowledgement: None,
            });
        }
    }
}

enum Command {
    Request {
        extra: u64,
        json: String,
        response: Sender<Result<Value, TransportError>>,
    },
    Cancel {
        extra: u64,
        acknowledgement: Option<Sender<()>>,
    },
    Shutdown,
}

/// Thread-safe request handle. Сам transport не `Clone`; для параллельных
/// callers его можно разделять через `Arc`, сохраняя один backend/receive loop.
pub struct TdJsonTransport {
    commands: Sender<Command>,
    next_extra: AtomicU64,
    thread: Option<JoinHandle<()>>,
}

impl TdJsonTransport {
    pub fn start<B: TdJsonBackend>(
        backend: B,
    ) -> Result<(Self, Receiver<TdJsonEvent>), TransportError> {
        let (command_tx, command_rx) = mpsc::channel();
        let (event_tx, event_rx) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("telegram-tdjson-receive".into())
            .spawn(move || receive_loop(backend, command_rx, event_tx))
            .map_err(|error| TransportError::Backend(error.to_string()))?;
        Ok((
            Self {
                commands: command_tx,
                next_extra: AtomicU64::new(1),
                thread: Some(thread),
            },
            event_rx,
        ))
    }

    pub fn request(&self, mut request: Value) -> Result<PendingResponse, TransportError> {
        let object = request
            .as_object_mut()
            .ok_or(TransportError::RequestMustBeObject)?;
        if object.contains_key("@extra") {
            return Err(TransportError::ReservedExtra);
        }
        let extra = self.reserve_extra()?;
        object.insert("@extra".into(), Value::from(extra));
        let json =
            serde_json::to_string(&request).map_err(|_| TransportError::RequestMustBeObject)?;
        let (response_tx, response_rx) = mpsc::channel();
        self.commands
            .send(Command::Request {
                extra,
                json,
                response: response_tx,
            })
            .map_err(|_| TransportError::TransportStopped)?;
        Ok(PendingResponse {
            receiver: response_rx,
            commands: self.commands.clone(),
            correlation_id: extra,
            finished: false,
        })
    }

    pub fn call(&self, request: Value, timeout: Duration) -> Result<Value, TransportError> {
        self.request(request)?.wait_timeout(timeout)
    }

    pub fn call_until(&self, request: Value, deadline: Instant) -> Result<Value, TransportError> {
        self.request(request)?.wait_until(deadline)
    }

    pub fn shutdown(mut self) -> Result<(), TransportError> {
        self.stop_and_join()
    }

    fn reserve_extra(&self) -> Result<u64, TransportError> {
        self.next_extra
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                (current != u64::MAX).then_some(current + 1)
            })
            .map_err(|_| TransportError::CorrelationExhausted)
    }

    fn stop_and_join(&mut self) -> Result<(), TransportError> {
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        let _ = self.commands.send(Command::Shutdown);
        thread
            .join()
            .map_err(|_| TransportError::Backend("receive thread panicked".into()))
    }
}

impl Drop for TdJsonTransport {
    fn drop(&mut self) {
        let _ = self.stop_and_join();
    }
}

fn receive_loop<B: TdJsonBackend>(
    mut backend: B,
    commands: Receiver<Command>,
    events: Sender<TdJsonEvent>,
) {
    let mut pending = HashMap::<u64, Sender<Result<Value, TransportError>>>::new();
    loop {
        let mut shutdown = false;
        loop {
            match commands.try_recv() {
                Ok(Command::Request {
                    extra,
                    json,
                    response,
                }) => {
                    pending.insert(extra, response);
                    if let Err(error) = backend.send(&json) {
                        if let Some(response) = pending.remove(&extra) {
                            let _ = response.send(Err(TransportError::Backend(error.to_string())));
                        }
                    }
                }
                Ok(Command::Cancel {
                    extra,
                    acknowledgement,
                }) => {
                    pending.remove(&extra);
                    if let Some(acknowledgement) = acknowledgement {
                        let _ = acknowledgement.send(());
                    }
                }
                Ok(Command::Shutdown) | Err(TryRecvError::Disconnected) => {
                    shutdown = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        if shutdown {
            fail_pending(&mut pending, TransportError::TransportStopped);
            return;
        }

        let raw = match backend.receive(RECEIVE_POLL) {
            Ok(Some(raw)) => raw,
            Ok(None) => continue,
            Err(error) => {
                let failure = TransportError::Backend(error.to_string());
                fail_pending(&mut pending, failure.clone());
                let _ = events.send(TdJsonEvent::Fatal(failure));
                return;
            }
        };
        let mut value: Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(_) => {
                let failure = TransportError::InvalidTdJsonResponse;
                fail_pending(&mut pending, failure.clone());
                let _ = events.send(TdJsonEvent::Fatal(failure));
                return;
            }
        };
        let extra = value
            .as_object_mut()
            .and_then(|object| object.remove("@extra"));
        match extra {
            None => {
                let _ = events.send(TdJsonEvent::Update(value));
            }
            Some(extra) => {
                let response = extra
                    .as_u64()
                    .and_then(|id| pending.remove(&id).map(|response| (id, response)));
                match response {
                    Some((correlation_id, response)) => {
                        let _ = events.send(TdJsonEvent::ResponseBoundary { correlation_id });
                        let _ = response.send(Ok(value));
                    }
                    None => {
                        let _ = events.send(TdJsonEvent::UnmatchedResponse {
                            extra,
                            response: value,
                        });
                    }
                }
            }
        }
    }
}

fn fail_pending(
    pending: &mut HashMap<u64, Sender<Result<Value, TransportError>>>,
    error: TransportError,
) {
    for (_, response) in pending.drain() {
        let _ = response.send(Err(error.clone()));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashSet, VecDeque};
    use std::sync::{Arc, Barrier, Mutex};

    use serde_json::json;

    use super::*;

    #[derive(Clone, Default)]
    struct ScriptedState {
        inner: Arc<Mutex<ScriptedInner>>,
    }

    #[derive(Default)]
    struct ScriptedInner {
        sent: Vec<Value>,
        incoming: VecDeque<String>,
        receive_threads: HashSet<thread::ThreadId>,
        reverse_pairs: bool,
        invalid_after_send: bool,
    }

    struct ScriptedBackend {
        state: ScriptedState,
    }

    impl ScriptedBackend {
        fn new(state: ScriptedState) -> Self {
            Self { state }
        }
    }

    impl TdJsonBackend for ScriptedBackend {
        fn send(&mut self, request: &str) -> Result<(), BackendError> {
            let request: Value = serde_json::from_str(request).unwrap();
            let mut inner = self.state.inner.lock().unwrap();
            inner.sent.push(request.clone());
            if inner.reverse_pairs && inner.sent.len() % 2 == 0 {
                let pair = &inner.sent[inner.sent.len() - 2..];
                let responses = pair
                    .iter()
                    .rev()
                    .map(|request| {
                        json!({
                            "@type": "result",
                            "sequence": request["sequence"],
                            "@extra": request["@extra"]
                        })
                        .to_string()
                    })
                    .collect::<Vec<_>>();
                inner.incoming.extend(responses);
            }
            if inner.invalid_after_send {
                inner.incoming.push_back("not json".into());
            }
            Ok(())
        }

        fn receive(&mut self, timeout: Duration) -> Result<Option<String>, BackendError> {
            let value = {
                let mut inner = self.state.inner.lock().unwrap();
                inner.receive_threads.insert(thread::current().id());
                inner.incoming.pop_front()
            };
            if value.is_none() {
                thread::sleep(timeout.min(Duration::from_millis(1)));
            }
            Ok(value)
        }
    }

    #[test]
    fn reversed_parallel_responses_are_correlated_on_one_receive_thread() {
        let state = ScriptedState::default();
        state.inner.lock().unwrap().reverse_pairs = true;
        let (transport, _events) =
            TdJsonTransport::start(ScriptedBackend::new(state.clone())).unwrap();
        let transport = Arc::new(transport);
        let barrier = Arc::new(Barrier::new(3));
        let callers = [1_u64, 2]
            .into_iter()
            .map(|sequence| {
                let transport = Arc::clone(&transport);
                let barrier = Arc::clone(&barrier);
                thread::spawn(move || {
                    barrier.wait();
                    transport
                        .call(
                            json!({"@type": "test", "sequence": sequence}),
                            Duration::from_secs(1),
                        )
                        .unwrap()
                })
            })
            .collect::<Vec<_>>();
        barrier.wait();
        let mut sequences = callers
            .into_iter()
            .map(|caller| caller.join().unwrap()["sequence"].as_u64().unwrap())
            .collect::<Vec<_>>();
        sequences.sort_unstable();
        assert_eq!(sequences, vec![1, 2]);
        assert_eq!(state.inner.lock().unwrap().receive_threads.len(), 1);
        drop(transport);
    }

    #[test]
    fn updates_keep_receive_order_and_unmatched_response_is_explicit() {
        let state = ScriptedState::default();
        state.inner.lock().unwrap().incoming.extend([
            json!({"@type": "updateOne", "value": 1}).to_string(),
            json!({"@type": "updateTwo", "value": 2}).to_string(),
            json!({"@type": "ok", "@extra": "foreign"}).to_string(),
        ]);
        let (transport, events) = TdJsonTransport::start(ScriptedBackend::new(state)).unwrap();
        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).unwrap(),
            TdJsonEvent::Update(json!({"@type": "updateOne", "value": 1}))
        );
        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).unwrap(),
            TdJsonEvent::Update(json!({"@type": "updateTwo", "value": 2}))
        );
        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).unwrap(),
            TdJsonEvent::UnmatchedResponse {
                extra: json!("foreign"),
                response: json!({"@type": "ok"}),
            }
        );
        transport.shutdown().unwrap();
    }

    #[test]
    fn transport_owns_extra_and_fails_closed_on_invalid_backend_json() {
        let state = ScriptedState::default();
        state.inner.lock().unwrap().invalid_after_send = true;
        let (transport, _events) =
            TdJsonTransport::start(ScriptedBackend::new(state.clone())).unwrap();
        assert!(matches!(
            transport.request(json!(["not", "an", "object"])),
            Err(TransportError::RequestMustBeObject)
        ));
        assert!(matches!(
            transport.request(json!({"@type": "getMe", "@extra": 7})),
            Err(TransportError::ReservedExtra)
        ));
        let pending = transport.request(json!({"@type": "getMe"})).unwrap();
        assert_eq!(
            pending.wait_timeout(Duration::from_secs(1)),
            Err(TransportError::InvalidTdJsonResponse)
        );
    }

    #[test]
    fn deadline_and_explicit_cancellation_remove_pending_response() {
        let state = ScriptedState::default();
        let (transport, events) =
            TdJsonTransport::start(ScriptedBackend::new(state.clone())).unwrap();
        assert_eq!(
            transport.call_until(json!({"@type":"slow"}), Instant::now()),
            Err(TransportError::ResponseTimeout)
        );
        let pending = transport.request(json!({"@type":"cancelled"})).unwrap();
        let extra = pending.correlation_id();
        pending.cancel().unwrap();
        state
            .inner
            .lock()
            .unwrap()
            .incoming
            .push_back(json!({"@type":"ok","@extra":extra}).to_string());
        assert_eq!(
            events.recv_timeout(Duration::from_secs(1)).unwrap(),
            TdJsonEvent::UnmatchedResponse {
                extra: Value::from(extra),
                response: json!({"@type":"ok"}),
            }
        );
        transport.shutdown().unwrap();
    }
}
