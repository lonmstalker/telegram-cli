//! Startup handshake и ordered runtime driver поверх TDJSON transport.

use std::fmt;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Instant;

use serde_json::{Value, json};

use crate::reducer::{AppliedUpdate, ReducerError, StateReducer, UpdateGap};
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

    pub fn mark_update_gap(&mut self) -> UpdateGap {
        self.reducer.mark_update_gap()
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

    pub(crate) fn apply_through_boundary(
        &mut self,
        boundary: u64,
        deadline: Instant,
    ) -> Result<usize, RuntimeError> {
        let mut applied = 0;
        loop {
            match self.events.recv_timeout(remaining(deadline)?) {
                Ok(TdJsonEvent::Update(update)) => {
                    self.reducer.apply(&update).map_err(RuntimeError::Reducer)?;
                    applied += 1;
                }
                Ok(TdJsonEvent::ResponseBoundary { correlation_id })
                    if correlation_id == boundary =>
                {
                    return Ok(applied);
                }
                Ok(TdJsonEvent::ResponseBoundary { .. }) => {}
                Ok(TdJsonEvent::UnmatchedResponse { .. }) => {
                    return Err(RuntimeError::UnexpectedRuntimeEvent);
                }
                Ok(TdJsonEvent::Fatal(error)) => return Err(RuntimeError::Transport(error)),
                Err(RecvTimeoutError::Timeout) => return Err(RuntimeError::DeadlineExceeded),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(RuntimeError::EventStreamClosed);
                }
            }
        }
    }

    pub(crate) fn replace_state_from_snapshot(
        &mut self,
        updates: &[Value],
    ) -> Result<(), RuntimeError> {
        self.reducer
            .replace_from_snapshot(updates)
            .map_err(RuntimeError::Reducer)
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
    UnexpectedRuntimeEvent,
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
            Self::UnexpectedRuntimeEvent => {
                formatter.write_str("unexpected unmatched response during TDLib workflow")
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
        load_calls: usize,
        emit_leave_update: bool,
        accept_pending_membership: bool,
        emit_resolution_update: bool,
        migrated_basic_group: bool,
        malformed_chat_list_entry: bool,
    }

    struct StartupBackend(StartupState);

    impl StartupBackend {
        fn new(version: String) -> (Self, StartupState) {
            let state = StartupState {
                inner: Arc::new(Mutex::new(StartupInner {
                    incoming: VecDeque::new(),
                    sent_types: Vec::new(),
                    version,
                    load_calls: 0,
                    emit_leave_update: true,
                    accept_pending_membership: false,
                    emit_resolution_update: false,
                    migrated_basic_group: false,
                    malformed_chat_list_entry: false,
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
                "getMe" => inner
                    .incoming
                    .push_back(json!({"@type":"user","id":"7","@extra":extra}).to_string()),
                "getChatStatistics" => inner.incoming.push_back(
                    json!({"@type":"error","code":400,"message":"not available","@extra":extra})
                        .to_string(),
                ),
                "loadChats" => {
                    inner.load_calls += 1;
                    if inner.load_calls <= 2 {
                        let chat_id = i64::try_from(inner.load_calls + 1).unwrap();
                        let order = i64::try_from(inner.load_calls * 10).unwrap();
                        let list = request["chat_list"].clone();
                        let chat_type = if inner.load_calls == 1 && inner.migrated_basic_group {
                            json!({
                                "@type":"chatTypeBasicGroup",
                                "basic_group_id":chat_id
                            })
                        } else if inner.load_calls == 1 && inner.malformed_chat_list_entry {
                            json!({
                                "@type":"chatTypeSupergroup",
                                "supergroup_id":chat_id
                            })
                        } else if inner.load_calls == 1 {
                            json!({
                                "@type":"chatTypeSupergroup",
                                "supergroup_id":chat_id,
                                "is_channel":true
                            })
                        } else {
                            json!({"@type":"chatTypePrivate","user_id":chat_id})
                        };
                        inner.incoming.push_back(
                            json!({
                                "@type":"updateNewChat",
                                "chat":{
                                    "@type":"chat",
                                    "id":chat_id,
                                    "title":format!("Chat {chat_id}"),
                                    "type":chat_type,
                                    "last_message":{
                                        "@type":"message",
                                        "content":"PRIVATE_MESSAGE_CANARY"
                                    },
                                    "positions":[],
                                    "chat_lists":[list.clone()]
                                }
                            })
                            .to_string(),
                        );
                        if inner.load_calls == 1 && inner.migrated_basic_group {
                            inner.incoming.push_back(
                                json!({
                                    "@type":"updateBasicGroup",
                                    "basic_group":{
                                        "@type":"basicGroup",
                                        "id":chat_id,
                                        "status":{"@type":"chatMemberStatusLeft"},
                                        "upgraded_to_supergroup_id":9
                                    }
                                })
                                .to_string(),
                            );
                        } else if inner.load_calls == 1 {
                            inner.incoming.push_back(
                                json!({
                                    "@type":"updateSupergroup",
                                    "supergroup":{
                                        "@type":"supergroup",
                                        "id":chat_id,
                                        "status":{"@type":"chatMemberStatusMember"}
                                    }
                                })
                                .to_string(),
                            );
                        }
                        inner.incoming.push_back(
                            json!({
                                "@type":"updateChatPosition",
                                "chat_id":chat_id,
                                "position":{
                                    "@type":"chatPosition",
                                    "list":list,
                                    "order":order,
                                    "is_pinned":false,
                                    "source":null
                                }
                            })
                            .to_string(),
                        );
                        inner
                            .incoming
                            .push_back(json!({"@type":"ok","@extra":extra}).to_string());
                    } else {
                        inner.incoming.push_back(
                            json!({"@type":"error","code":404,"message":"Not Found","@extra":extra})
                                .to_string(),
                        );
                    }
                }
                "searchPublicChat" => {
                    let failing = request["username"] == "fail";
                    let chat_id = if failing { 51 } else { 50 };
                    let supergroup_id = if failing { 61 } else { 60 };
                    let mut response = json!({
                        "@type":"chat",
                        "id":chat_id,
                        "title":"Public channel",
                        "type":{
                            "@type":"chatTypeSupergroup",
                            "supergroup_id":supergroup_id,
                            "is_channel":true
                        },
                        "last_message":{
                            "@type":"message",
                            "content":"PRIVATE_MESSAGE_CANARY"
                        },
                        "positions":[],
                        "chat_lists":[]
                    });
                    response
                        .as_object_mut()
                        .unwrap()
                        .insert("@extra".to_owned(), extra);
                    if inner.emit_resolution_update {
                        inner.incoming.push_back(
                            json!({
                                "@type":"updateSupergroup",
                                "supergroup":{
                                    "@type":"supergroup",
                                    "id":supergroup_id,
                                    "has_location":false,
                                    "usernames":{"active_usernames":["boundary_name"]}
                                }
                            })
                            .to_string(),
                        );
                    }
                    inner.incoming.push_back(response.to_string());
                }
                "checkChatInviteLink" => {
                    let is_public = request["invite_link"]
                        .as_str()
                        .is_some_and(|link| link.contains("public"));
                    inner.incoming.push_back(
                        json!({
                            "@type":"chatInviteLinkInfo",
                            "chat_id":if is_public { 50 } else { 0 },
                            "accessible_for":0,
                            "type":{"@type":"inviteLinkChatTypeChannel"},
                            "title":if is_public { "Public channel" } else { "Private channel" },
                            "description":"PRIVATE_DESCRIPTION_CANARY",
                            "member_user_ids":[7,8],
                            "creates_join_request":!is_public,
                            "is_public":is_public,
                            "@extra":extra
                        })
                        .to_string(),
                    );
                }
                "leaveChat" => {
                    let chat_id = request["chat_id"].as_i64().unwrap();
                    if inner.emit_leave_update {
                        inner.incoming.push_back(
                            json!({
                                "@type":"updateSupergroup",
                                "supergroup":{
                                    "@type":"supergroup",
                                    "id":chat_id,
                                    "status":{"@type":"chatMemberStatusLeft"}
                                }
                            })
                            .to_string(),
                        );
                    }
                    inner
                        .incoming
                        .push_back(json!({"@type":"ok","@extra":extra}).to_string());
                }
                "joinChatByInviteLink" => inner.incoming.push_back(
                    json!({"@type":"chatJoinResultRequestSent","@extra":extra}).to_string(),
                ),
                "getChat" => {
                    let chat_type = if inner.migrated_basic_group {
                        json!({"@type":"chatTypeBasicGroup","basic_group_id":2})
                    } else {
                        json!({
                            "@type":"chatTypeSupergroup",
                            "supergroup_id":2,
                            "is_channel":true
                        })
                    };
                    inner.incoming.push_back(
                        json!({
                            "@type":"chat",
                            "id":2,
                            "title":"Channel",
                            "type":chat_type,
                            "@extra":extra
                        })
                        .to_string(),
                    );
                }
                "getBasicGroup" => inner.incoming.push_back(
                    json!({
                        "@type":"basicGroup",
                        "id":2,
                        "upgraded_to_supergroup_id":9,
                        "@extra":extra
                    })
                    .to_string(),
                ),
                "getSupergroup" => {
                    let status = if inner.accept_pending_membership {
                        "chatMemberStatusMember"
                    } else {
                        "chatMemberStatusLeft"
                    };
                    if inner.accept_pending_membership {
                        inner.incoming.push_back(
                            json!({
                                "@type":"updateSupergroup",
                                "supergroup":{
                                    "@type":"supergroup",
                                    "id":2,
                                    "status":{"@type":status,"member_until_date":0}
                                }
                            })
                            .to_string(),
                        );
                    }
                    inner.incoming.push_back(
                        json!({
                            "@type":"supergroup",
                            "id":2,
                            "status":{"@type":status,"member_until_date":0},
                            "@extra":extra
                        })
                        .to_string(),
                    );
                }
                "openChat" | "closeChat" => inner
                    .incoming
                    .push_back(json!({"@type":"ok","@extra":extra}).to_string()),
                "getSupergroupFullInfo" if request["supergroup_id"] == 61 => {
                    inner.incoming.push_back(
                        json!({"@type":"error","code":400,"message":"unavailable","@extra":extra})
                            .to_string(),
                    );
                }
                "getSupergroupFullInfo" => inner.incoming.push_back(
                    json!({
                        "@type":"supergroupFullInfo",
                        "description":"PRIVATE_FULL_INFO_CANARY",
                        "invite_link":{"invite_link":"https://t.me/+PRIVATE_CANARY"},
                        "@extra":extra
                    })
                    .to_string(),
                ),
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
        let version = crate::raw_api::version(&runtime);
        assert_eq!(version.tdlib_version, identity.version());
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![crate::registry::RiskClass::Read],
        );
        let response = crate::raw_api::td_call(
            &runtime,
            &policy,
            json!({"@type":"getChatStatistics"}),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(response.as_value()["@type"], "error");
        let sent_after_allowed = state.inner.lock().unwrap().sent_types.len();
        assert!(matches!(
            crate::raw_api::td_call(
                &runtime,
                &policy,
                json!({"@type":"testSquareInt","x":2}),
                Instant::now() + Duration::from_secs(1),
            ),
            Err(crate::raw_api::RawApiError::Policy(
                crate::raw_api::PolicyError::DefaultDeny
            ))
        ));
        assert_eq!(
            state.inner.lock().unwrap().sent_types.len(),
            sent_after_allowed
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

    #[test]
    fn chat_list_repeats_loads_until_404_and_orders_position_updates() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![crate::registry::RiskClass::Read],
        );
        let snapshot = crate::workflows::load_chat_list(
            &mut runtime,
            &policy,
            crate::reducer::ChatList::Main,
            100,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        assert_eq!(snapshot.load_calls, 3);
        assert_eq!(snapshot.entries.len(), snapshot.positions.len());
        assert_eq!(
            snapshot
                .entries
                .iter()
                .map(|entry| entry.kind)
                .collect::<Vec<_>>(),
            [
                crate::workflows::ChatListEntryKind::Private,
                crate::workflows::ChatListEntryKind::Channel,
            ]
        );
        let serialized_entries = serde_json::to_string(&snapshot.entries).unwrap();
        assert!(!serialized_entries.contains("PRIVATE_MESSAGE_CANARY"));
        assert!(!serialized_entries.contains("last_message"));
        assert_eq!(
            snapshot
                .positions
                .iter()
                .map(|position| (position.order, position.chat_id))
                .collect::<Vec<_>>(),
            [(20, 3), (10, 2)]
        );
        assert_eq!(state.inner.lock().unwrap().load_calls, 3);
        runtime.shutdown().unwrap();
    }

    #[test]
    fn leave_chat_waits_for_left_update_and_is_idempotent() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                crate::registry::RiskClass::Read,
                crate::registry::RiskClass::ReversibleMutation,
            ],
        );
        crate::workflows::load_chat_list(
            &mut runtime,
            &policy,
            crate::reducer::ChatList::Main,
            100,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        let left = crate::workflows::leave_chat(
            &mut runtime,
            &policy,
            2,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(
            left.outcome,
            crate::workflows::LeaveChatOutcome::VerifiedLeft
        );
        assert!(left.complete);
        assert!(left.sequence.is_some());

        let already_left = crate::workflows::leave_chat(
            &mut runtime,
            &policy,
            2,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(
            already_left.outcome,
            crate::workflows::LeaveChatOutcome::AlreadyNotMember
        );
        assert!(already_left.complete);
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .sent_types
                .iter()
                .filter(|method| method.as_str() == "leaveChat")
                .count(),
            1
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn leave_chat_timeout_is_uncertain_without_retry() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                crate::registry::RiskClass::Read,
                crate::registry::RiskClass::ReversibleMutation,
            ],
        );
        crate::workflows::load_chat_list(
            &mut runtime,
            &policy,
            crate::reducer::ChatList::Main,
            100,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        state.inner.lock().unwrap().emit_leave_update = false;

        let receipt = crate::workflows::leave_chat(
            &mut runtime,
            &policy,
            2,
            Instant::now() + Duration::from_millis(5),
        )
        .unwrap();
        assert_eq!(
            receipt.outcome,
            crate::workflows::LeaveChatOutcome::Uncertain
        );
        assert!(!receipt.complete);
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .sent_types
                .iter()
                .filter(|method| method.as_str() == "leaveChat")
                .count(),
            1
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn pending_membership_accepts_late_member_update_without_rejoining() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                crate::registry::RiskClass::Read,
                crate::registry::RiskClass::ReversibleMutation,
            ],
        );
        crate::workflows::load_chat_list(
            &mut runtime,
            &policy,
            crate::reducer::ChatList::Main,
            100,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        crate::workflows::leave_chat(
            &mut runtime,
            &policy,
            2,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        let pending = crate::workflows::ensure_membership(
            &runtime,
            &policy,
            crate::workflows::MembershipTarget::InviteLink("https://t.me/+pending"),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(
            pending.state,
            crate::workflows::MembershipState::RequestPending
        );
        assert!(pending.submission_complete);
        assert!(!pending.membership_complete);

        state.inner.lock().unwrap().accept_pending_membership = true;
        let status = crate::workflows::membership_status(
            &mut runtime,
            &policy,
            crate::workflows::MembershipTarget::ChatId(2),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(
            status.state,
            crate::workflows::MembershipStatusState::Member
        );
        assert!(status.complete);
        assert_eq!(
            runtime.state().supergroup(2).unwrap().value["status"]["@type"],
            "chatMemberStatusMember"
        );
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .sent_types
                .iter()
                .filter(|method| method.as_str() == "joinChatByInviteLink")
                .count(),
            1
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn resolve_applies_pre_response_supergroup_update() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![crate::registry::RiskClass::Read],
        );
        state.inner.lock().unwrap().emit_resolution_update = true;

        let resolution = crate::workflows::resolve(
            &mut runtime,
            &policy,
            crate::workflows::ChatTarget::PublicUsername("public_name"),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        assert_eq!(
            resolution.chat.active_usernames,
            ["public_name", "boundary_name"]
        );
        assert_eq!(
            runtime.state().supergroup(60).unwrap().value["usernames"]["active_usernames"],
            json!(["boundary_name"])
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn leave_chat_reports_basic_group_migration_without_dispatch() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                crate::registry::RiskClass::Read,
                crate::registry::RiskClass::ReversibleMutation,
            ],
        );
        state.inner.lock().unwrap().migrated_basic_group = true;
        crate::workflows::load_chat_list(
            &mut runtime,
            &policy,
            crate::reducer::ChatList::Main,
            100,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        assert!(matches!(
            crate::workflows::leave_chat(
                &mut runtime,
                &policy,
                2,
                Instant::now() + Duration::from_secs(1),
            ),
            Ok(crate::workflows::LeaveChatReceipt {
                outcome: crate::workflows::LeaveChatOutcome::MigrationRequired { supergroup_id: 9 },
                complete: false,
                ..
            })
        ));
        assert!(
            !state
                .inner
                .lock()
                .unwrap()
                .sent_types
                .iter()
                .any(|method| method == "leaveChat")
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn membership_status_reports_basic_group_migration_without_guessing_membership() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![crate::registry::RiskClass::Read],
        );
        state.inner.lock().unwrap().migrated_basic_group = true;

        let status = crate::workflows::membership_status(
            &mut runtime,
            &policy,
            crate::workflows::MembershipTarget::ChatId(2),
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        assert_eq!(
            status.state,
            crate::workflows::MembershipStatusState::Migrated { supergroup_id: 9 }
        );
        assert!(!status.complete);
        assert!(
            state
                .inner
                .lock()
                .unwrap()
                .sent_types
                .ends_with(&["getChat".to_owned(), "getBasicGroup".to_owned()])
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn chat_list_keeps_snapshot_when_one_supergroup_lacks_is_channel() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![crate::registry::RiskClass::Read],
        );
        state.inner.lock().unwrap().malformed_chat_list_entry = true;

        let snapshot = crate::workflows::load_chat_list(
            &mut runtime,
            &policy,
            crate::reducer::ChatList::Main,
            100,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        assert!(
            snapshot
                .entries
                .iter()
                .any(|entry| entry.kind == crate::workflows::ChatListEntryKind::Unknown)
        );
        assert_eq!(snapshot.entries.len(), 2);
        runtime.shutdown().unwrap();
    }

    #[test]
    fn chat_inspection_uses_direct_chat_response_and_pairs_open_with_close() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                crate::registry::RiskClass::Read,
                crate::registry::RiskClass::Presence,
            ],
        );
        let inspection = crate::workflows::inspect_chat(
            &mut runtime,
            &policy,
            crate::workflows::ChatTarget::PublicLink("https://t.me/public_name"),
            true,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();

        assert!(inspection.complete());
        assert_eq!(
            inspection.full_info_kind,
            crate::workflows::ChatFullInfoKind::Supergroup
        );
        assert_eq!(
            inspection.resolution.chat.visibility,
            crate::workflows::ChatVisibility::Public
        );
        let serialized = serde_json::to_string(&inspection).unwrap();
        assert!(!serialized.contains("PRIVATE_MESSAGE_CANARY"));
        assert!(!serialized.contains("PRIVATE_FULL_INFO_CANARY"));
        assert!(!serialized.contains("PRIVATE_CANARY"));
        assert!(!serialized.contains("last_message"));
        assert!(!serialized.contains("invite_link"));
        assert!(state.inner.lock().unwrap().sent_types.ends_with(&[
            "searchPublicChat".to_owned(),
            "openChat".to_owned(),
            "getSupergroupFullInfo".to_owned(),
            "closeChat".to_owned(),
        ]));
        runtime.shutdown().unwrap();
    }

    #[test]
    fn chat_inspection_closes_open_chat_when_full_info_fails() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let mut runtime =
            CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                crate::registry::RiskClass::Read,
                crate::registry::RiskClass::Presence,
            ],
        );
        assert!(matches!(
            crate::workflows::inspect_chat(
                &mut runtime,
                &policy,
                crate::workflows::ChatTarget::PublicUsername("fail"),
                true,
                Instant::now() + Duration::from_secs(1),
            ),
            Err(crate::workflows::ChatWorkflowError::Tdlib {
                method: "getSupergroupFullInfo",
                code: Some(400)
            })
        ));
        assert_eq!(
            state
                .inner
                .lock()
                .unwrap()
                .sent_types
                .last()
                .map(String::as_str),
            Some("closeChat")
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn invite_preview_classifies_visibility_without_joining_or_opening() {
        let identity = pinned_identity().unwrap();
        let (backend, state) = StartupBackend::new(identity.version);
        let runtime = CoreRuntime::start(backend, Instant::now() + Duration::from_secs(1)).unwrap();
        let policy = crate::raw_api::RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![crate::registry::RiskClass::Read],
        );
        let public = crate::workflows::preview_invite_link(
            &runtime,
            &policy,
            "https://t.me/+public",
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        let non_public = crate::workflows::preview_invite_link(
            &runtime,
            &policy,
            "https://t.me/+private",
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
        assert_eq!(public.visibility, crate::workflows::ChatVisibility::Public);
        assert_eq!(public.access, crate::workflows::InviteAccess::Accessible);
        assert!(public.complete);
        assert_eq!(
            non_public.visibility,
            crate::workflows::ChatVisibility::NonPublic
        );
        assert_eq!(
            non_public.access,
            crate::workflows::InviteAccess::PreviewOnly
        );
        assert!(non_public.creates_join_request);
        let sent_types = state.inner.lock().unwrap().sent_types.clone();
        assert_eq!(
            sent_types
                .iter()
                .filter(|method| method.as_str() == "checkChatInviteLink")
                .count(),
            2
        );
        assert!(sent_types.iter().all(|method| !matches!(
            method.as_str(),
            "joinChat" | "joinChatByInviteLink" | "openChat" | "closeChat"
        )));
        runtime.shutdown().unwrap();
    }
}
