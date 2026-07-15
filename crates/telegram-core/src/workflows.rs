//! Curated stateful workflows поверх общего TDJSON call.

use std::error::Error;
use std::fmt;
use std::time::Instant;

use serde_json::{Value, json};

use crate::raw_api::{RawApiError, RawPolicy, td_call, td_call_with_boundary};
use crate::reducer::{ChatList, ChatListPosition, ReducerError};
use crate::registry::TdObject;
use crate::runtime::{CoreRuntime, RuntimeError};

#[derive(Clone, Copy)]
pub enum ChatTarget<'value> {
    Id(i64),
    PublicUsername(&'value str),
    PublicLink(&'value str),
    InviteLink(&'value str),
}

pub enum MembershipTarget<'value> {
    ChatId(i64),
    InviteLink(&'value str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResolutionState {
    Chat { chat_id: i64 },
    InvitePreview { chat_id: Option<i64> },
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatResolution {
    pub state: ResolutionState,
    pub raw: TdObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipState {
    Member { chat_id: i64 },
    RequestPending,
    ApprovalRequired { bot_user_id: i64, query_id: i64 },
    Declined,
    Unknown,
}

impl MembershipState {
    pub fn complete(self) -> bool {
        matches!(self, Self::Member { .. } | Self::Declined)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MembershipResult {
    pub state: MembershipState,
    pub raw: TdObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatListTerminal {
    AllChatsLoaded,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChatListSnapshot {
    pub positions: Vec<ChatListPosition>,
    pub load_calls: usize,
    pub terminal: ChatListTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChatInspectionStatus {
    Complete,
    MembershipRequired,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChatInspection {
    pub status: ChatInspectionStatus,
    pub resolution: ChatResolution,
    pub cached_chat: Option<Value>,
    pub full_info: Option<TdObject>,
    pub used_open_lease: bool,
}

impl ChatInspection {
    pub fn complete(&self) -> bool {
        self.status == ChatInspectionStatus::Complete
    }
}

pub fn resolve(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    target: ChatTarget<'_>,
    deadline: Instant,
) -> Result<ChatResolution, ChatWorkflowError> {
    resolve_with(target, |request| {
        td_call(runtime, policy, request, deadline)
    })
}

pub fn ensure_membership(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    target: MembershipTarget<'_>,
    deadline: Instant,
) -> Result<MembershipResult, ChatWorkflowError> {
    ensure_membership_with(target, |request| {
        td_call(runtime, policy, request, deadline)
    })
}

pub fn load_chat_list(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    list: ChatList,
    limit: i32,
    deadline: Instant,
) -> Result<ChatListSnapshot, ChatWorkflowError> {
    if limit <= 0 {
        return Err(ChatWorkflowError::InvalidLimit);
    }
    let load_calls = load_until_terminal(list.tdjson(), limit, |request| {
        let (response, boundary) = td_call_with_boundary(runtime, policy, request, deadline)
            .map_err(ChatWorkflowError::Call)?;
        runtime
            .apply_through_boundary(boundary, deadline)
            .map_err(ChatWorkflowError::Runtime)?;
        Ok(response)
    })?;
    Ok(ChatListSnapshot {
        positions: runtime
            .state()
            .chat_list_positions(list)
            .map_err(ChatWorkflowError::Reducer)?,
        load_calls,
        terminal: ChatListTerminal::AllChatsLoaded,
    })
}

pub fn inspect_chat(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    target: ChatTarget<'_>,
    open: bool,
    deadline: Instant,
) -> Result<ChatInspection, ChatWorkflowError> {
    let (method, request) = resolution_request(target)?;
    let (raw, boundary) = td_call_with_boundary(runtime, policy, request, deadline)
        .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let resolution = resolution_from_raw(method, checked_response(method, raw)?)?;

    let chat_id = match resolution.state {
        ResolutionState::Chat { chat_id } => chat_id,
        ResolutionState::InvitePreview {
            chat_id: Some(chat_id),
        } if runtime.state().chat(chat_id).is_some() => chat_id,
        ResolutionState::InvitePreview { .. } => {
            return Ok(ChatInspection {
                status: ChatInspectionStatus::MembershipRequired,
                resolution,
                cached_chat: None,
                full_info: None,
                used_open_lease: false,
            });
        }
        ResolutionState::Unknown => {
            return Ok(ChatInspection {
                status: ChatInspectionStatus::Unknown,
                resolution,
                cached_chat: None,
                full_info: None,
                used_open_lease: false,
            });
        }
    };
    let cached_chat = wait_for_chat(runtime, chat_id, deadline)?;
    let full_info = load_full_info(runtime, policy, &cached_chat, open, deadline)?;
    Ok(ChatInspection {
        status: ChatInspectionStatus::Complete,
        resolution,
        cached_chat: Some(cached_chat),
        full_info: Some(full_info),
        used_open_lease: open,
    })
}

fn load_until_terminal(
    list: Value,
    limit: i32,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<usize, ChatWorkflowError> {
    let mut load_calls = 0;
    loop {
        let response = call(json!({"@type":"loadChats","chat_list":list,"limit":limit}))?;
        load_calls += 1;
        match response.as_value()["@type"].as_str() {
            Some("ok") => {}
            Some("error") => {
                let code = required_i64(response.as_value(), "code", "loadChats")?;
                if code == 404 {
                    return Ok(load_calls);
                }
                return Err(ChatWorkflowError::Tdlib {
                    method: "loadChats",
                    code: Some(code),
                });
            }
            _ => {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "loadChats",
                });
            }
        }
    }
}

fn resolve_with(
    target: ChatTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<ChatResolution, ChatWorkflowError> {
    let (method, request) = resolution_request(target)?;
    let raw = checked_call(method, request, &mut call)?;
    resolution_from_raw(method, raw)
}

fn resolution_request(target: ChatTarget<'_>) -> Result<(&'static str, Value), ChatWorkflowError> {
    Ok(match target {
        ChatTarget::Id(chat_id) => ("getChat", json!({"@type":"getChat","chat_id":chat_id})),
        ChatTarget::PublicUsername(username) => (
            "searchPublicChat",
            json!({"@type":"searchPublicChat","username":username_value(username)?}),
        ),
        ChatTarget::PublicLink(link) => (
            "searchPublicChat",
            json!({"@type":"searchPublicChat","username":public_link_username(link)?}),
        ),
        ChatTarget::InviteLink(invite_link) => (
            "checkChatInviteLink",
            json!({"@type":"checkChatInviteLink","invite_link":invite_link}),
        ),
    })
}

fn resolution_from_raw(
    method: &'static str,
    raw: TdObject,
) -> Result<ChatResolution, ChatWorkflowError> {
    let state = match raw.as_value()["@type"].as_str() {
        Some("chat") => ResolutionState::Chat {
            chat_id: required_i64(raw.as_value(), "id", method)?,
        },
        Some("chatInviteLinkInfo") => ResolutionState::InvitePreview {
            chat_id: optional_nonzero_i64(raw.as_value(), "chat_id", method)?,
        },
        _ => ResolutionState::Unknown,
    };
    Ok(ChatResolution { state, raw })
}

fn username_value(username: &str) -> Result<&str, ChatWorkflowError> {
    let username = username.strip_prefix('@').unwrap_or(username);
    if username.is_empty() || username.contains('/') {
        return Err(ChatWorkflowError::InvalidTarget);
    }
    Ok(username)
}

fn public_link_username(link: &str) -> Result<&str, ChatWorkflowError> {
    const PREFIXES: [&str; 4] = [
        "https://t.me/",
        "http://t.me/",
        "https://telegram.me/",
        "http://telegram.me/",
    ];
    let path = PREFIXES
        .iter()
        .find_map(|prefix| link.strip_prefix(prefix))
        .ok_or(ChatWorkflowError::InvalidTarget)?;
    let username = path.split(['?', '#']).next().unwrap_or_default();
    if username.starts_with('+') || username.starts_with("joinchat/") {
        return Err(ChatWorkflowError::InvalidTarget);
    }
    username_value(username)
}

fn wait_for_chat(
    runtime: &mut CoreRuntime,
    chat_id: i64,
    deadline: Instant,
) -> Result<Value, ChatWorkflowError> {
    loop {
        if let Some(chat) = runtime.state().chat(chat_id) {
            return Ok(chat.value.clone());
        }
        match runtime
            .next_event_until(deadline)
            .map_err(ChatWorkflowError::Runtime)?
        {
            crate::runtime::CoreRuntimeEvent::State(_) => {}
            crate::runtime::CoreRuntimeEvent::UnmatchedResponse { .. } => {
                return Err(ChatWorkflowError::Runtime(
                    RuntimeError::UnexpectedRuntimeEvent,
                ));
            }
        }
    }
}

fn load_full_info(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat: &Value,
    open: bool,
    deadline: Instant,
) -> Result<TdObject, ChatWorkflowError> {
    let (method, request) = full_info_request(chat)?;
    if !open {
        return invoke(runtime, policy, method, request, deadline);
    }

    let chat_id = required_i64(chat, "id", "chat")?;
    let lease = OpenChatLease::acquire(runtime, policy, chat_id, deadline)?;
    let result = invoke(runtime, policy, method, request, deadline);
    let cleanup = lease.close();
    cleanup?;
    result
}

fn full_info_request(chat: &Value) -> Result<(&'static str, Value), ChatWorkflowError> {
    let chat_type = chat["type"]
        .as_object()
        .ok_or(ChatWorkflowError::InvalidResult {
            method: "chat",
            field: "type",
        })?;
    Ok(match chat_type.get("@type").and_then(Value::as_str) {
        Some("chatTypePrivate" | "chatTypeSecret") => (
            "getUserFullInfo",
            json!({
                "@type":"getUserFullInfo",
                "user_id":required_i64(&chat["type"], "user_id", "chat")?
            }),
        ),
        Some("chatTypeBasicGroup") => (
            "getBasicGroupFullInfo",
            json!({
                "@type":"getBasicGroupFullInfo",
                "basic_group_id":required_i64(&chat["type"], "basic_group_id", "chat")?
            }),
        ),
        Some("chatTypeSupergroup") => (
            "getSupergroupFullInfo",
            json!({
                "@type":"getSupergroupFullInfo",
                "supergroup_id":required_i64(&chat["type"], "supergroup_id", "chat")?
            }),
        ),
        _ => {
            return Err(ChatWorkflowError::InvalidResult {
                method: "chat",
                field: "type.@type",
            });
        }
    })
}

struct OpenChatLease<'runtime> {
    runtime: &'runtime CoreRuntime,
    policy: &'runtime RawPolicy,
    chat_id: i64,
    deadline: Instant,
    active: bool,
}

impl<'runtime> OpenChatLease<'runtime> {
    fn acquire(
        runtime: &'runtime CoreRuntime,
        policy: &'runtime RawPolicy,
        chat_id: i64,
        deadline: Instant,
    ) -> Result<Self, ChatWorkflowError> {
        expect_ok(
            invoke(
                runtime,
                policy,
                "openChat",
                json!({"@type":"openChat","chat_id":chat_id}),
                deadline,
            )?,
            "openChat",
        )?;
        Ok(Self {
            runtime,
            policy,
            chat_id,
            deadline,
            active: true,
        })
    }

    fn close(mut self) -> Result<(), ChatWorkflowError> {
        self.active = false;
        close_chat(self.runtime, self.policy, self.chat_id, self.deadline)
    }
}

impl Drop for OpenChatLease<'_> {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            let _ = close_chat(self.runtime, self.policy, self.chat_id, self.deadline);
        }
    }
}

fn close_chat(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    deadline: Instant,
) -> Result<(), ChatWorkflowError> {
    expect_ok(
        invoke(
            runtime,
            policy,
            "closeChat",
            json!({"@type":"closeChat","chat_id":chat_id}),
            deadline,
        )?,
        "closeChat",
    )
}

fn invoke(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    method: &'static str,
    request: Value,
    deadline: Instant,
) -> Result<TdObject, ChatWorkflowError> {
    let response = td_call(runtime, policy, request, deadline).map_err(ChatWorkflowError::Call)?;
    checked_response(method, response)
}

fn expect_ok(response: TdObject, method: &'static str) -> Result<(), ChatWorkflowError> {
    if response.as_value()["@type"] == "ok" {
        Ok(())
    } else {
        Err(ChatWorkflowError::UnexpectedResult { method })
    }
}

fn ensure_membership_with(
    target: MembershipTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<MembershipResult, ChatWorkflowError> {
    let (method, request) = match target {
        MembershipTarget::ChatId(chat_id) => {
            ("joinChat", json!({"@type":"joinChat","chat_id":chat_id}))
        }
        MembershipTarget::InviteLink(invite_link) => (
            "joinChatByInviteLink",
            json!({"@type":"joinChatByInviteLink","invite_link":invite_link}),
        ),
    };
    let raw = checked_call(method, request, &mut call)?;
    let state = match raw.as_value()["@type"].as_str() {
        Some("chatJoinResultSuccess") => MembershipState::Member {
            chat_id: required_i64(raw.as_value(), "chat_id", method)?,
        },
        Some("chatJoinResultRequestSent") => MembershipState::RequestPending,
        Some("chatJoinResultGuardBotApprovalRequired") => MembershipState::ApprovalRequired {
            bot_user_id: required_i64(raw.as_value(), "bot_user_id", method)?,
            query_id: required_i64(raw.as_value(), "query_id", method)?,
        },
        Some("chatJoinResultDeclined") => MembershipState::Declined,
        _ => MembershipState::Unknown,
    };
    Ok(MembershipResult { state, raw })
}

fn checked_call(
    method: &'static str,
    request: Value,
    call: &mut impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<TdObject, ChatWorkflowError> {
    let response = call(request).map_err(ChatWorkflowError::Call)?;
    checked_response(method, response)
}

fn checked_response(
    method: &'static str,
    response: TdObject,
) -> Result<TdObject, ChatWorkflowError> {
    if response.as_value()["@type"] == "error" {
        return Err(ChatWorkflowError::Tdlib {
            method,
            code: response.as_value()["code"]
                .as_i64()
                .or_else(|| response.as_value()["code"].as_str()?.parse().ok()),
        });
    }
    Ok(response)
}

fn required_i64(
    value: &Value,
    field: &'static str,
    method: &'static str,
) -> Result<i64, ChatWorkflowError> {
    value[field]
        .as_i64()
        .or_else(|| value[field].as_str()?.parse().ok())
        .ok_or(ChatWorkflowError::InvalidResult { method, field })
}

fn optional_nonzero_i64(
    value: &Value,
    field: &'static str,
    method: &'static str,
) -> Result<Option<i64>, ChatWorkflowError> {
    required_i64(value, field, method).map(|value| (value != 0).then_some(value))
}

#[derive(Debug)]
pub enum ChatWorkflowError {
    Call(RawApiError),
    Runtime(RuntimeError),
    Reducer(ReducerError),
    Tdlib {
        method: &'static str,
        code: Option<i64>,
    },
    InvalidResult {
        method: &'static str,
        field: &'static str,
    },
    UnexpectedResult {
        method: &'static str,
    },
    InvalidTarget,
    InvalidLimit,
}

impl fmt::Display for ChatWorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call(error) => write!(formatter, "chat workflow call failed: {error}"),
            Self::Runtime(error) => write!(formatter, "chat workflow runtime failed: {error}"),
            Self::Reducer(error) => write!(formatter, "chat list cache failed: {error}"),
            Self::Tdlib { method, code } => {
                write!(formatter, "TDLib `{method}` failed with code {code:?}")
            }
            Self::InvalidResult { method, field } => {
                write!(formatter, "TDLib `{method}` result has invalid `{field}`")
            }
            Self::UnexpectedResult { method } => {
                write!(formatter, "TDLib `{method}` returned an unexpected result")
            }
            Self::InvalidTarget => formatter.write_str("chat target is invalid or ambiguous"),
            Self::InvalidLimit => formatter.write_str("chat list load limit must be positive"),
        }
    }
}

impl Error for ChatWorkflowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Call(error) => Some(error),
            Self::Runtime(error) => Some(error),
            Self::Reducer(error) => Some(error),
            Self::Tdlib { .. }
            | Self::InvalidResult { .. }
            | Self::UnexpectedResult { .. }
            | Self::InvalidTarget
            | Self::InvalidLimit => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{CapabilityDisposition, RiskClass, capability};

    fn object(value: Value) -> Result<TdObject, RawApiError> {
        Ok(TdObject::from_value(value).unwrap())
    }

    #[test]
    fn resolve_never_dispatches_membership_methods() {
        let mut methods = Vec::new();
        for target in [
            ChatTarget::Id(7),
            ChatTarget::PublicUsername("public_name"),
            ChatTarget::PublicLink("https://t.me/public_name?single"),
            ChatTarget::InviteLink("private-link"),
        ] {
            resolve_with(target, |request| {
                let method = request["@type"].as_str().unwrap().to_owned();
                methods.push(method.clone());
                match method.as_str() {
                    "getChat" | "searchPublicChat" => object(json!({"@type":"chat","id":7})),
                    "checkChatInviteLink" => {
                        object(json!({"@type":"chatInviteLinkInfo","chat_id":7}))
                    }
                    _ => unreachable!(),
                }
            })
            .unwrap();
        }
        assert_eq!(
            methods,
            [
                "getChat",
                "searchPublicChat",
                "searchPublicChat",
                "checkChatInviteLink"
            ]
        );
        assert!(matches!(
            resolution_request(ChatTarget::PublicLink("https://t.me/+invite")),
            Err(ChatWorkflowError::InvalidTarget)
        ));
    }

    #[test]
    fn membership_is_explicit_and_preserves_pending_outcomes() {
        let member = ensure_membership_with(MembershipTarget::ChatId(7), |request| {
            assert_eq!(request["@type"], "joinChat");
            object(json!({"@type":"chatJoinResultSuccess","chat_id":7}))
        })
        .unwrap();
        assert_eq!(member.state, MembershipState::Member { chat_id: 7 });
        assert!(member.state.complete());

        let pending =
            ensure_membership_with(MembershipTarget::InviteLink("private-link"), |request| {
                assert_eq!(request["@type"], "joinChatByInviteLink");
                object(json!({"@type":"chatJoinResultRequestSent"}))
            })
            .unwrap();
        assert_eq!(pending.state, MembershipState::RequestPending);
        assert!(!pending.state.complete());

        let approval = ensure_membership_with(MembershipTarget::ChatId(7), |_| {
            object(json!({
                "@type":"chatJoinResultGuardBotApprovalRequired",
                "bot_user_id":8,
                "query_id":"9007199254740993"
            }))
        })
        .unwrap();
        assert_eq!(
            approval.state,
            MembershipState::ApprovalRequired {
                bot_user_id: 8,
                query_id: 9_007_199_254_740_993
            }
        );
        assert!(!approval.state.complete());
    }

    #[test]
    fn tdlib_error_does_not_become_not_found_or_membership_proof() {
        let error = resolve_with(ChatTarget::PublicUsername("missing"), |_| {
            object(json!({"@type":"error","code":400,"message":"not found"}))
        })
        .unwrap_err();
        assert!(matches!(
            error,
            ChatWorkflowError::Tdlib {
                method: "searchPublicChat",
                code: Some(400)
            }
        ));
    }

    #[test]
    fn policy_data_separates_read_resolution_from_membership_mutation() {
        assert!(matches!(
            capability("searchPublicChat").unwrap().disposition,
            CapabilityDisposition::Reviewed {
                risk: RiskClass::Read,
                ..
            }
        ));
        assert!(matches!(
            capability("joinChat").unwrap().disposition,
            CapabilityDisposition::Reviewed {
                risk: RiskClass::ReversibleMutation,
                ..
            }
        ));
    }

    #[test]
    fn chat_type_selects_exact_full_info_method() {
        let cases = [
            (
                json!({"@type":"chatTypePrivate","user_id":1}),
                "getUserFullInfo",
            ),
            (
                json!({"@type":"chatTypeSecret","user_id":1}),
                "getUserFullInfo",
            ),
            (
                json!({"@type":"chatTypeBasicGroup","basic_group_id":2}),
                "getBasicGroupFullInfo",
            ),
            (
                json!({"@type":"chatTypeSupergroup","supergroup_id":3}),
                "getSupergroupFullInfo",
            ),
        ];
        for (chat_type, expected) in cases {
            assert_eq!(
                full_info_request(&json!({"@type":"chat","id":7,"type":chat_type}))
                    .unwrap()
                    .0,
                expected
            );
        }
    }
}
