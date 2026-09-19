//! Однопоточный TDJSON transport и `@extra` correlation.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::Value;

const COMMAND_CAPACITY: usize = 1024;
const EVENT_CAPACITY: usize = 1024;
const EVENT_BYTES: usize = 16 * 1024 * 1024;

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
    Overloaded,
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
            Self::Overloaded => formatter.write_str("TDJSON request queue is full"),
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

/// Ordered bounded events. Overflow disconnects the stream, forcing runtime failure and a daemon restart.
pub struct EventReceiver {
    receiver: Receiver<(TdJsonEvent, usize)>,
    bytes: Arc<AtomicUsize>,
}

impl EventReceiver {
    pub fn recv_timeout(&self, timeout: Duration) -> Result<TdJsonEvent, RecvTimeoutError> {
        let (event, bytes) = self.receiver.recv_timeout(timeout)?;
        self.bytes.fetch_sub(bytes, Ordering::Relaxed);
        Ok(event)
    }
}

struct EventSender {
    sender: SyncSender<(TdJsonEvent, usize)>,
    bytes: Arc<AtomicUsize>,
}

fn event_channel() -> (EventSender, EventReceiver) {
    let (sender, receiver) = mpsc::sync_channel(EVENT_CAPACITY);
    let bytes = Arc::new(AtomicUsize::new(0));
    (
        EventSender {
            sender,
            bytes: bytes.clone(),
        },
        EventReceiver { receiver, bytes },
    )
}

impl EventSender {
    fn send(&self, event: TdJsonEvent) -> Result<(), ()> {
        let bytes = match &event {
            TdJsonEvent::Update(value) => value.to_string().len(),
            TdJsonEvent::UnmatchedResponse { extra, response } => {
                extra.to_string().len() + response.to_string().len()
            }
            _ => 64,
        };
        self.bytes
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |used| {
                used.checked_add(bytes)
                    .filter(|total| *total <= EVENT_BYTES)
            })
            .map_err(|_| ())?;
        if self.sender.try_send((event, bytes)).is_err() {
            self.bytes.fetch_sub(bytes, Ordering::Relaxed);
            return Err(());
        }
        Ok(())
    }
}

pub struct PendingResponse {
    receiver: Receiver<Result<Value, TransportError>>,
    commands: SyncSender<Command>,
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
            .try_send(Command::Cancel {
                extra: self.correlation_id,
                acknowledgement: Some(acknowledgement),
            })
            .map_err(command_queue_error)?;
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
            let _ = self.commands.try_send(Command::Cancel {
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
    commands: SyncSender<Command>,
    next_extra: AtomicU64,
    thread: Option<JoinHandle<()>>,
}

impl TdJsonTransport {
    pub fn start<B: TdJsonBackend>(backend: B) -> Result<(Self, EventReceiver), TransportError> {
        let (command_tx, command_rx) = mpsc::sync_channel(COMMAND_CAPACITY);
        let (event_tx, event_rx) = event_channel();
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
            .try_send(Command::Request {
                extra,
                json,
                response: response_tx,
            })
            .map_err(command_queue_error)?;
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

fn command_queue_error<T>(error: mpsc::TrySendError<T>) -> TransportError {
    match error {
        mpsc::TrySendError::Full(_) => TransportError::Overloaded,
        mpsc::TrySendError::Disconnected(_) => TransportError::TransportStopped,
    }
}

fn receive_loop<B: TdJsonBackend>(
    mut backend: B,
    commands: Receiver<Command>,
    events: EventSender,
) {
    let mut pending = HashMap::<u64, Sender<Result<Value, TransportError>>>::new();
    loop {
        let mut shutdown = false;
        for _ in 0..COMMAND_CAPACITY {
            match commands.try_recv() {
                Ok(Command::Request {
                    extra,
                    json,
                    response,
                }) => {
                    if pending.len() >= COMMAND_CAPACITY {
                        let _ = response.send(Err(TransportError::Overloaded));
                        continue;
                    }
                    pending.insert(extra, response);
                    if let Err(error) = backend.send(&json)
                        && let Some(response) = pending.remove(&extra)
                    {
                        let _ = response.send(Err(TransportError::Backend(error.to_string())));
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
        if raw.len() > EVENT_BYTES {
            fail_pending(&mut pending, TransportError::TransportStopped);
            return;
        }
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
                if events.send(TdJsonEvent::Update(value)).is_err() {
                    return;
                }
            }
            Some(extra) => {
                let response = extra
                    .as_u64()
                    .and_then(|id| pending.remove(&id).map(|response| (id, response)));
                match response {
                    Some((correlation_id, response)) => {
                        if events
                            .send(TdJsonEvent::ResponseBoundary { correlation_id })
                            .is_err()
                        {
                            return;
                        }
                        if let Err(undelivered) = response.send(Ok(value))
                            && let Ok(response) = undelivered.0
                            && events
                                .send(TdJsonEvent::UnmatchedResponse {
                                    extra: Value::from(correlation_id),
                                    response,
                                })
                                .is_err()
                        {
                            return;
                        }
                    }
                    None => {
                        if events
                            .send(TdJsonEvent::UnmatchedResponse {
                                extra,
                                response: value,
                            })
                            .is_err()
                        {
                            return;
                        }
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

    #[test]
    fn command_queue_overload_is_distinct_from_shutdown() {
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.try_send(()).unwrap();
        assert_eq!(
            command_queue_error(sender.try_send(()).unwrap_err()),
            TransportError::Overloaded
        );
        drop(receiver);
        assert_eq!(
            command_queue_error(sender.try_send(()).unwrap_err()),
            TransportError::TransportStopped
        );
    }

    #[test]
    fn event_queue_bounds_count_and_bytes_without_blocking() {
        let (sender, receiver) = event_channel();
        for _ in 0..EVENT_CAPACITY {
            sender
                .send(TdJsonEvent::Update(json!({"@type":"updateNewMessage"})))
                .unwrap();
        }
        assert!(sender.send(TdJsonEvent::Update(json!({}))).is_err());
        receiver.recv_timeout(Duration::ZERO).unwrap();
        assert!(sender.send(TdJsonEvent::Update(json!({}))).is_ok());
        let (sender, receiver) = event_channel();
        assert!(
            sender
                .send(TdJsonEvent::Update(Value::String("x".repeat(EVENT_BYTES))))
                .is_err()
        );
        assert_eq!(receiver.bytes.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn overflow_stops_transport_instead_of_hanging_a_pending_call() {
        let state = ScriptedState::default();
        state
            .inner
            .lock()
            .unwrap()
            .incoming
            .extend((0..EVENT_CAPACITY + 1).map(|_| r#"{"@type":"updateNewMessage"}"#.to_owned()));
        let (transport, _events) = TdJsonTransport::start(ScriptedBackend::new(state)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !transport.thread.as_ref().unwrap().is_finished() && Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(transport.thread.as_ref().unwrap().is_finished());
        assert!(matches!(
            transport.call(json!({"@type":"getMe"}), Duration::from_millis(50)),
            Err(TransportError::TransportStopped)
        ));
    }

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
            if inner.reverse_pairs && inner.sent.len().is_multiple_of(2) {
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

    #[test]
    fn timed_out_dispatch_emits_late_response_without_second_send() {
        let state = ScriptedState::default();
        let (transport, events) =
            TdJsonTransport::start(ScriptedBackend::new(state.clone())).unwrap();
        let pending = transport.request(json!({"@type": "authAction"})).unwrap();
        let extra = pending.correlation_id();
        assert_eq!(
            pending.wait_until(Instant::now()),
            Err(TransportError::ResponseTimeout)
        );

        let deadline = Instant::now() + Duration::from_secs(1);
        while state.inner.lock().unwrap().sent.is_empty() && Instant::now() < deadline {
            thread::yield_now();
        }
        assert_eq!(state.inner.lock().unwrap().sent.len(), 1);
        state
            .inner
            .lock()
            .unwrap()
            .incoming
            .push_back(json!({"@type": "error", "code": 500, "@extra": extra}).to_string());

        let late = loop {
            let event = events.recv_timeout(Duration::from_secs(1)).unwrap();
            if matches!(event, TdJsonEvent::UnmatchedResponse { .. }) {
                break event;
            }
        };
        assert_eq!(
            late,
            TdJsonEvent::UnmatchedResponse {
                extra: Value::from(extra),
                response: json!({"@type": "error", "code": 500}),
            }
        );
        assert_eq!(state.inner.lock().unwrap().sent.len(), 1);
        transport.shutdown().unwrap();
    }
}
