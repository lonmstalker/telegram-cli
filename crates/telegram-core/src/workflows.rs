//! Curated stateful workflows поверх общего TDJSON call.

use std::error::Error;
use std::fmt;
use std::time::Instant;

use serde_json::{Value, json};

use crate::raw_api::{RawApiError, RawPolicy, td_call};
use crate::registry::TdObject;
use crate::runtime::CoreRuntime;

pub enum ChatTarget<'value> {
    Id(i64),
    PublicUsername(&'value str),
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

fn resolve_with(
    target: ChatTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<ChatResolution, ChatWorkflowError> {
    let (method, request) = match target {
        ChatTarget::Id(chat_id) => ("getChat", json!({"@type":"getChat","chat_id":chat_id})),
        ChatTarget::PublicUsername(username) => (
            "searchPublicChat",
            json!({"@type":"searchPublicChat","username":username}),
        ),
        ChatTarget::InviteLink(invite_link) => (
            "checkChatInviteLink",
            json!({"@type":"checkChatInviteLink","invite_link":invite_link}),
        ),
    };
    let raw = checked_call(method, request, &mut call)?;
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
    if response.as_value()["@type"] == "error" {
        return Err(ChatWorkflowError::Tdlib {
            method,
            code: response.as_value()["code"].as_i64(),
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
    Tdlib {
        method: &'static str,
        code: Option<i64>,
    },
    InvalidResult {
        method: &'static str,
        field: &'static str,
    },
}

impl fmt::Display for ChatWorkflowError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call(error) => write!(formatter, "chat workflow call failed: {error}"),
            Self::Tdlib { method, code } => {
                write!(formatter, "TDLib `{method}` failed with code {code:?}")
            }
            Self::InvalidResult { method, field } => {
                write!(formatter, "TDLib `{method}` result has invalid `{field}`")
            }
        }
    }
}

impl Error for ChatWorkflowError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Call(error) => Some(error),
            Self::Tdlib { .. } | Self::InvalidResult { .. } => None,
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
            ["getChat", "searchPublicChat", "checkChatInviteLink"]
        );
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
}
