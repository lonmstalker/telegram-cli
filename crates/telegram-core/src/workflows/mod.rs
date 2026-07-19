//! Curated stateful workflows поверх общего TDJSON call.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::time::{Instant, SystemTime};

use serde::Serialize;
use serde_json::{Map, Value, json};

use crate::approval::PlanPreview;
use crate::authorization::SensitiveString;
use crate::raw_api::{RawApiError, RawPolicy, td_call, td_call_with_boundary};
use crate::reducer::{ChatList, ChatListPosition, MessageSendKey, MessageSendState, ReducerError};
use crate::registry::{RetryClass, RiskClass, TdObject, ValidatedRequest};
use crate::runtime::{CoreRuntime, RuntimeError};
use crate::transport::TransportError;

mod chat;
pub use chat::*;
mod forum;
pub use forum::*;
mod user;
pub use user::*;
mod message;
pub use message::*;
mod members;
pub use members::*;
mod proxy;
pub use proxy::*;
mod files;
use files::require_uploaded_file;
pub use files::*;
mod sticker;
pub use sticker::*;
mod story;
pub use story::*;
mod call;
pub use call::*;
mod session;
pub use session::*;
mod business;
pub use business::*;
mod stars;
pub use stars::*;
mod bot;
use bot::bot_start_receipt;
pub use bot::*;
mod webapp;
pub use webapp::*;

fn username_value(username: &str) -> Result<&str, ChatWorkflowError> {
    let username = username.strip_prefix('@').unwrap_or(username);
    if username.is_empty() || username.contains('/') {
        return Err(ChatWorkflowError::InvalidTarget);
    }
    Ok(username)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    ServerSnapshot,
    OrderedUpdate,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ResyncReceipt {
    pub gap_after_sequence: Option<u64>,
    pub snapshot_updates: usize,
    pub sequence: Option<u64>,
    pub source: TerminalSource,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalSource {
    Response,
    OrderedUpdate,
}

pub fn resync_after_gap(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    deadline: Instant,
) -> Result<ResyncReceipt, ChatWorkflowError> {
    let gap = runtime
        .state()
        .gap()
        .ok_or(ChatWorkflowError::NoResyncRequired)?;
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({"@type":"getCurrentState"}),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("getCurrentState", response)?;
    if response.as_value()["@type"] != "updates" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getCurrentState",
        });
    }
    let updates =
        response.as_value()["updates"]
            .as_array()
            .ok_or(ChatWorkflowError::InvalidResult {
                method: "getCurrentState",
                field: "updates",
            })?;
    runtime
        .replace_state_from_snapshot(updates)
        .map_err(ChatWorkflowError::Runtime)?;
    Ok(ResyncReceipt {
        gap_after_sequence: gap.after_sequence.map(|sequence| sequence.get()),
        snapshot_updates: updates.len(),
        sequence: runtime
            .state()
            .last_sequence()
            .map(|sequence| sequence.get()),
        source: TerminalSource::Response,
        complete: true,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

fn response_timed_out(error: &ChatWorkflowError) -> bool {
    matches!(
        error,
        ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))
    )
}

fn last_sequence(runtime: &CoreRuntime) -> u64 {
    runtime
        .state()
        .last_sequence()
        .map(|sequence| sequence.get())
        .unwrap_or(0)
}

fn require_resynced(runtime: &CoreRuntime) -> Result<(), ChatWorkflowError> {
    match runtime.state().gap() {
        Some(gap) => Err(ChatWorkflowError::ResyncRequired {
            gap_after_sequence: gap.after_sequence.map(|sequence| sequence.get()),
        }),
        None => Ok(()),
    }
}

fn wait_message_send(
    runtime: &mut CoreRuntime,
    key: MessageSendKey,
    deadline: Instant,
) -> Result<BotStartReceipt, ChatWorkflowError> {
    loop {
        let outcome = runtime
            .state()
            .message_send(key)
            .map(|state| state.state.clone());
        match outcome {
            Some(MessageSendState::Succeeded { message }) => {
                return Ok(bot_start_receipt(
                    key.old_message_id,
                    BotStartOutcome::Succeeded { message },
                    true,
                ));
            }
            Some(MessageSendState::Failed { message, error }) => {
                return Ok(bot_start_receipt(
                    key.old_message_id,
                    BotStartOutcome::Failed { message, error },
                    true,
                ));
            }
            Some(MessageSendState::Acknowledged) | None => {}
        }
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => {
                return Ok(bot_start_receipt(
                    key.old_message_id,
                    BotStartOutcome::Uncertain,
                    false,
                ));
            }
            Err(error) => return Err(ChatWorkflowError::Runtime(error)),
        }
    }
}

fn call_and_apply(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    method: &'static str,
    request: Value,
    deadline: Instant,
) -> Result<TdObject, ChatWorkflowError> {
    let (response, boundary) = td_call_with_boundary(runtime, policy, request, deadline)
        .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    checked_response(method, response)
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

#[derive(Clone, Copy)]
struct ConnectionObservation {
    sequence: u64,
    ready: bool,
}

fn invoke_ordered(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    method: &'static str,
    request: Value,
    deadline: Instant,
) -> Result<(TdObject, Option<ConnectionObservation>), ChatWorkflowError> {
    let (response, boundary) = td_call_with_boundary(runtime, policy, request, deadline)
        .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response(method, response)?;
    let connection = runtime
        .state()
        .connection()
        .map(|connection| ConnectionObservation {
            sequence: connection.sequence.get(),
            ready: connection.value["@type"] == "connectionStateReady",
        });
    Ok((response, connection))
}

fn expect_ok(response: TdObject, method: &'static str) -> Result<(), ChatWorkflowError> {
    if response.as_value()["@type"] == "ok" {
        Ok(())
    } else {
        Err(ChatWorkflowError::UnexpectedResult { method })
    }
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

fn required_i32(
    value: &Value,
    field: &'static str,
    method: &'static str,
) -> Result<i32, ChatWorkflowError> {
    i32::try_from(required_i64(value, field, method)?)
        .map_err(|_| ChatWorkflowError::InvalidResult { method, field })
}

fn required_string<'value>(
    value: &'value Value,
    field: &'static str,
    method: &'static str,
) -> Result<&'value str, ChatWorkflowError> {
    value[field]
        .as_str()
        .ok_or(ChatWorkflowError::InvalidResult { method, field })
}

fn required_bool(
    value: &Value,
    field: &'static str,
    method: &'static str,
) -> Result<bool, ChatWorkflowError> {
    value[field]
        .as_bool()
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
    PrerequisiteMissing {
        prerequisite: &'static str,
    },
    CapabilityDenied {
        capability: &'static str,
    },
    InvalidTarget,
    InvalidLimit,
    InvalidPageOptions,
    InvalidFileTransfer,
    InvalidStickerSetMutation,
    InvalidStoryMutation,
    InvalidGroupCall,
    InvalidNotificationSettings,
    InvalidSessionTarget,
    InvalidBusinessInput,
    InvalidPaymentInput,
    InvalidProxyTarget,
    InvalidProfileInput,
    InvalidChatConfiguration,
    InvalidBotInteraction,
    PlanStale,
    ResyncRequired {
        gap_after_sequence: Option<u64>,
    },
    NoResyncRequired,
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
            Self::PrerequisiteMissing { prerequisite } => {
                write!(formatter, "required cached `{prerequisite}` is missing")
            }
            Self::CapabilityDenied { capability } => {
                write!(formatter, "TDLib capability `{capability}` is unavailable")
            }
            Self::InvalidTarget => formatter.write_str("chat target is invalid or ambiguous"),
            Self::InvalidLimit => formatter.write_str("chat list load limit must be positive"),
            Self::InvalidPageOptions => {
                formatter.write_str("requested count and page limit must be within bounds")
            }
            Self::InvalidFileTransfer => {
                formatter.write_str("file transfer input is outside TDLib bounds")
            }
            Self::InvalidStickerSetMutation => {
                formatter.write_str("custom emoji set mutation is outside TDLib bounds")
            }
            Self::InvalidStoryMutation => {
                formatter.write_str("story mutation is outside TDLib bounds")
            }
            Self::InvalidGroupCall => formatter.write_str("group call identifier is invalid"),
            Self::InvalidNotificationSettings => {
                formatter.write_str("notification settings are outside TDLib bounds")
            }
            Self::InvalidSessionTarget => formatter.write_str("session target is invalid"),
            Self::InvalidBusinessInput => {
                formatter.write_str("business connection or message input is invalid")
            }
            Self::InvalidPaymentInput => {
                formatter.write_str("payment input is outside the approved Stars-only boundary")
            }
            Self::InvalidProxyTarget => formatter.write_str("proxy target is invalid or missing"),
            Self::InvalidProfileInput => {
                formatter.write_str("profile input is outside TDLib bounds")
            }
            Self::InvalidChatConfiguration => {
                formatter.write_str("chat configuration is outside TDLib bounds")
            }
            Self::InvalidBotInteraction => {
                formatter.write_str("bot interaction is outside the recorded reply")
            }
            Self::PlanStale => formatter.write_str("chat configuration plan is stale"),
            Self::ResyncRequired { gap_after_sequence } => write!(
                formatter,
                "update state is gapped after sequence {gap_after_sequence:?}; resync is required"
            ),
            Self::NoResyncRequired => formatter.write_str("update state isn't gapped"),
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
            | Self::PrerequisiteMissing { .. }
            | Self::CapabilityDenied { .. }
            | Self::InvalidTarget
            | Self::InvalidLimit
            | Self::InvalidPageOptions
            | Self::InvalidFileTransfer
            | Self::InvalidStickerSetMutation
            | Self::InvalidStoryMutation
            | Self::InvalidGroupCall
            | Self::InvalidNotificationSettings
            | Self::InvalidSessionTarget
            | Self::InvalidBusinessInput
            | Self::InvalidPaymentInput
            | Self::InvalidProxyTarget
            | Self::InvalidProfileInput
            | Self::InvalidChatConfiguration
            | Self::InvalidBotInteraction
            | Self::PlanStale
            | Self::ResyncRequired { .. }
            | Self::NoResyncRequired => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::business::{business_connection_with, send_business_text_with};
    use super::call::leave_group_call_with;
    use super::chat::{ensure_membership_with, full_info_request};
    use super::files::{FileDirection, cancel_download_with, file_size_verified};
    use super::forum::{forum_topics_with, set_forum_topic_closed_with};
    use super::members::{chat_statistics_with, resource_statistics_with, supergroup_members_with};
    use super::message::{
        chat_history_with, message_page, redact_message_content, search_chat_messages_with,
    };
    use super::proxy::{proxy_snapshot, set_proxy_enabled_with};
    use super::session::{
        active_sessions_with, set_notification_settings_with, terminate_session_with,
    };
    use super::stars::{apply_star_invoice_payment_with, plan_star_invoice_payment_with};
    use super::sticker::custom_emoji_set_with;
    use super::story::story_mutation_with;
    use super::*;
    use crate::registry::RiskClass;
    use crate::transport::{BackendError, TdJsonBackend};
    use ed25519_dalek::{Signer, SigningKey};

    fn object(value: Value) -> Result<TdObject, RawApiError> {
        Ok(TdObject::from_value(value).unwrap())
    }

    fn workflow_object(value: Value) -> Result<TdObject, ChatWorkflowError> {
        Ok(TdObject::from_value(value).unwrap())
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

    #[test]
    fn short_history_page_continues_until_requested_count() {
        let mut cursors = Vec::new();
        let mut page = 0;
        let result = chat_history_with(
            HistoryQuery {
                chat_id: 7,
                only_local: false,
                mark_read: false,
                page: PageOptions {
                    count: 3,
                    min_date: None,
                    page_limit: 100,
                },
            },
            |request| {
                cursors.push(request["from_message_id"].as_i64().unwrap());
                page += 1;
                workflow_object(if page == 1 {
                    json!({"@type":"messages","total_count":9,"messages":[
                        {"@type":"message","id":5,"date":50},
                        {"@type":"message","id":4,"date":40}
                    ]})
                } else {
                    json!({"@type":"messages","total_count":9,"messages":[
                        {"@type":"message","id":4,"date":40},
                        {"@type":"message","id":3,"date":30}
                    ]})
                })
            },
        )
        .unwrap();

        assert_eq!(cursors, [0, 4]);
        assert_eq!(result.pages, 2);
        assert_eq!(result.boundary, PageBoundary::Count);
        assert!(result.complete);
        assert_eq!(
            result
                .messages
                .iter()
                .map(|message| message["id"].as_i64().unwrap())
                .collect::<Vec<_>>(),
            [5, 4, 3]
        );
    }

    #[test]
    fn search_uses_returned_cursor_and_repeated_cursor_is_partial() {
        let mut cursors = Vec::new();
        let result = search_chat_messages_with(
            ChatSearchQuery {
                chat_id: 7,
                query: "query",
                mark_read: false,
                page: PageOptions {
                    count: 10,
                    min_date: None,
                    page_limit: 100,
                },
            },
            |request| {
                let cursor = request["from_message_id"].as_i64().unwrap();
                cursors.push(cursor);
                workflow_object(if cursor == 0 {
                    json!({"@type":"foundChatMessages","total_count":9,"messages":[
                        {"@type":"message","id":9,"date":90}
                    ],"next_from_message_id":7})
                } else {
                    json!({"@type":"foundChatMessages","total_count":9,"messages":[
                        {"@type":"message","id":7,"date":70}
                    ],"next_from_message_id":7})
                })
            },
        )
        .unwrap();

        assert_eq!(cursors, [0, 7]);
        assert_eq!(result.boundary, PageBoundary::NoProgress);
        assert!(!result.complete);
    }

    #[test]
    fn search_date_and_exhausted_cursors_are_complete_boundaries() {
        let options = PageOptions {
            count: 10,
            min_date: Some(20),
            page_limit: 100,
        };
        let dated = search_chat_messages_with(
            ChatSearchQuery {
                chat_id: 7,
                query: "query",
                mark_read: false,
                page: options,
            },
            |_| {
                workflow_object(json!({
                    "@type":"foundChatMessages",
                    "total_count":2,
                    "messages":[
                        {"@type":"message","id":2,"date":20},
                        {"@type":"message","id":1,"date":19}
                    ],
                    "next_from_message_id":1
                }))
            },
        )
        .unwrap();
        assert_eq!(dated.boundary, PageBoundary::Date);
        assert!(dated.complete);
        assert_eq!(dated.messages.len(), 1);

        let exhausted = search_chat_messages_with(
            ChatSearchQuery {
                chat_id: 7,
                query: "query",
                mark_read: false,
                page: PageOptions {
                    min_date: None,
                    ..options
                },
            },
            |_| {
                workflow_object(json!({
                    "@type":"foundChatMessages",
                    "total_count":0,
                    "messages":[],
                    "next_from_message_id":0
                }))
            },
        )
        .unwrap();
        assert_eq!(exhausted.boundary, PageBoundary::Exhausted);
        assert!(exhausted.complete);
    }

    #[test]
    fn members_continue_after_short_page_and_stop_without_progress() {
        let mut offsets = Vec::new();
        let complete = supergroup_members_with(
            MembersQuery {
                supergroup_id: 7,
                count: 3,
                page_limit: 200,
            },
            true,
            11,
            |request| {
                let offset = request["offset"].as_i64().unwrap();
                offsets.push(offset);
                workflow_object(if offset == 0 {
                    json!({"@type":"chatMembers","total_count":3,"members":[
                        {"@type":"chatMember","member_id":{"@type":"messageSenderUser","user_id":1}}
                    ]})
                } else {
                    json!({"@type":"chatMembers","total_count":3,"members":[
                        {"@type":"chatMember","member_id":{"@type":"messageSenderUser","user_id":2}},
                        {"@type":"chatMember","member_id":{"@type":"messageSenderUser","user_id":3}}
                    ]})
                })
            },
        )
        .unwrap();
        assert_eq!(offsets, [0, 1]);
        assert_eq!(complete.boundary, PageBoundary::Count);
        assert_eq!(complete.members.len(), 3);
        assert_eq!(complete.capability_sequence, 11);
        assert!(complete.complete);

        let partial = supergroup_members_with(
            MembersQuery {
                supergroup_id: 7,
                count: 3,
                page_limit: 200,
            },
            true,
            12,
            |_| workflow_object(json!({"@type":"chatMembers","total_count":3,"members":[]})),
        )
        .unwrap();
        assert_eq!(partial.boundary, PageBoundary::NoProgress);
        assert!(!partial.complete);
    }

    #[test]
    fn forum_topics_follow_returned_cursor_after_short_page() {
        let mut cursors = Vec::new();
        let result = forum_topics_with(
            ForumTopicsQuery {
                chat_id: 7,
                query: "",
                count: 2,
                page_limit: 100,
            },
            |request| {
                let cursor = request["offset_forum_topic_id"].as_i64().unwrap();
                cursors.push(cursor);
                workflow_object(if cursor == 0 {
                    json!({"@type":"forumTopics","total_count":2,"topics":[
                        {"@type":"forumTopic","info":{"@type":"forumTopicInfo","forum_topic_id":1}}
                    ],"next_offset_date":20,"next_offset_message_id":30,"next_offset_forum_topic_id":1})
                } else {
                    json!({"@type":"forumTopics","total_count":2,"topics":[
                        {"@type":"forumTopic","info":{"@type":"forumTopicInfo","forum_topic_id":2}}
                    ],"next_offset_date":0,"next_offset_message_id":0,"next_offset_forum_topic_id":0})
                })
            },
        )
        .unwrap();

        assert_eq!(cursors, [0, 1]);
        assert_eq!(result.boundary, PageBoundary::Count);
        assert_eq!(result.topics.len(), 2);
        assert!(result.complete);
    }

    #[test]
    fn repeated_forum_topic_cursor_is_partial() {
        let result = forum_topics_with(
            ForumTopicsQuery {
                chat_id: 7,
                query: "",
                count: 2,
                page_limit: 100,
            },
            |_| {
                workflow_object(json!({
                    "@type":"forumTopics","total_count":2,"topics":[],
                    "next_offset_date":20,"next_offset_message_id":30,"next_offset_forum_topic_id":1
                }))
            },
        )
        .unwrap();

        assert_eq!(result.pages, 2);
        assert_eq!(result.boundary, PageBoundary::NoProgress);
        assert!(!result.complete);
    }

    #[test]
    fn topic_close_is_desired_state_and_reconciles_timeout() {
        let mut calls = Vec::new();
        let receipt = set_forum_topic_closed_with(7, 3, true, |method, _| {
            calls.push(method);
            workflow_object(json!({
                "@type":"forumTopic","info":{"@type":"forumTopicInfo","is_closed":true}
            }))
        })
        .unwrap();
        assert_eq!(calls, ["getForumTopic"]);
        assert_eq!(receipt.outcome, TopicMutationOutcome::AlreadyApplied);

        let mut calls = 0;
        let reconciled = set_forum_topic_closed_with(7, 3, true, |method, _| {
            calls += 1;
            match (method, calls) {
                ("getForumTopic", 1) => workflow_object(json!({
                    "@type":"forumTopic","info":{"@type":"forumTopicInfo","is_closed":false}
                })),
                ("toggleForumTopicIsClosed", _) => Err(ChatWorkflowError::Call(
                    RawApiError::Transport(TransportError::ResponseTimeout),
                )),
                ("getForumTopic", _) => workflow_object(json!({
                    "@type":"forumTopic","info":{"@type":"forumTopicInfo","is_closed":true}
                })),
                _ => unreachable!(),
            }
        })
        .unwrap();
        assert_eq!(reconciled.outcome, TopicMutationOutcome::Verified);
        assert!(reconciled.complete);
    }

    #[test]
    fn protected_message_content_is_redacted() {
        let mut page = message_page(
            vec![json!({
                "@type":"message","id":1,
                "content":{"@type":"messageText","text":"PROTECTED_CONTENT_CANARY"}
            })],
            1,
            None,
            PageBoundary::Exhausted,
        );

        redact_message_content(true, &mut page);
        assert!(page.content_redacted);
        assert_eq!(page.messages[0]["content"]["@type"], "messageText");
        assert!(
            !serde_json::to_string(&page)
                .unwrap()
                .contains("PROTECTED_CONTENT_CANARY")
        );
    }

    #[test]
    fn cancel_download_probes_desired_state_after_timeout() {
        let mut reads = 0;
        let receipt = cancel_download_with(5, false, |method, _| match method {
            "getFile" => {
                reads += 1;
                workflow_object(json!({
                    "@type":"file",
                    "local":{"@type":"localFile","is_downloading_active":reads == 1}
                }))
            }
            "cancelDownloadFile" => Err(ChatWorkflowError::Call(RawApiError::Transport(
                TransportError::ResponseTimeout,
            ))),
            _ => unreachable!(),
        })
        .unwrap();

        assert_eq!(receipt.outcome, TransferCancellationOutcome::Verified);
        assert!(receipt.complete);
        assert!(
            file_size_verified(
                &json!({
                    "size":10,"expected_size":10,
                    "local":{"downloaded_size":9},"remote":{"uploaded_size":0}
                }),
                FileDirection::Download,
            )
            .is_err()
        );
    }

    #[test]
    fn custom_emoji_set_lifecycle_reconciles_and_proves_cleanup() {
        let set = |file_ids: &[i32]| {
            json!({
                "@type":"stickerSet",
                "id":77,
                "name":"codex_disposable",
                "is_owned":true,
                "sticker_type":{"@type":"stickerTypeCustomEmoji"},
                "stickers":file_ids.iter().map(|id| json!({
                    "@type":"sticker","sticker":{"@type":"file","id":id}
                })).collect::<Vec<_>>()
            })
        };
        let create = CustomEmojiSetAction::Create {
            user_id: 7,
            title: "Disposable",
            name: "codex_disposable",
            format: StickerFormat::Webp,
            sticker_file_id: 6,
            emojis: "🧪",
            needs_repainting: false,
        };
        let plan = plan_custom_emoji_set(create).unwrap();
        assert_eq!(plan.risk, RiskClass::Admin);
        assert_eq!(plan.retry, RetryClass::Reconcile);
        assert_eq!(plan.plan_hash.len(), 64);
        assert!(!serde_json::to_string(&plan).unwrap().contains("@type"));

        let created = custom_emoji_set_with(create, |method, _| {
            workflow_object(match method {
                "getFile" => json!({
                    "@type":"file","id":6,
                    "remote":{"@type":"remoteFile","is_uploading_completed":true}
                }),
                "checkStickerSetName" => json!({"@type":"checkStickerSetNameResultOk"}),
                "createNewStickerSet" | "getStickerSet" => set(&[6]),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(created.outcome, StickerSetMutationOutcome::Verified);
        assert_eq!(created.set_id, Some(77));
        assert!(created.complete);

        let add = CustomEmojiSetAction::Add {
            user_id: 7,
            set_id: 77,
            name: "codex_disposable",
            format: StickerFormat::Webp,
            sticker_file_id: 7,
            emojis: "✅",
        };
        let mut reads = 0;
        let added = custom_emoji_set_with(add, |method, _| match method {
            "getFile" => workflow_object(json!({
                "@type":"file","id":7,
                "remote":{"@type":"remoteFile","is_uploading_completed":true}
            })),
            "getStickerSet" => {
                reads += 1;
                workflow_object(if reads == 1 { set(&[6]) } else { set(&[6, 7]) })
            }
            "addStickerToSet" => Err(ChatWorkflowError::Call(RawApiError::Transport(
                TransportError::ResponseTimeout,
            ))),
            _ => unreachable!(),
        })
        .unwrap();
        assert_eq!(reads, 2);
        assert_eq!(added.outcome, StickerSetMutationOutcome::Verified);
        assert_eq!(added.sticker_count, 2);

        let delete = CustomEmojiSetAction::Delete {
            set_id: 77,
            name: "codex_disposable",
        };
        assert_eq!(
            plan_custom_emoji_set(delete).unwrap().risk,
            RiskClass::Destructive
        );
        let mut checks = 0;
        let deleted = custom_emoji_set_with(delete, |method, _| {
            workflow_object(match method {
                "checkStickerSetName" => {
                    checks += 1;
                    if checks == 1 {
                        json!({"@type":"checkStickerSetNameResultNameOccupied"})
                    } else {
                        json!({"@type":"checkStickerSetNameResultOk"})
                    }
                }
                "getStickerSet" => set(&[6, 7]),
                "deleteStickerSet" => json!({"@type":"ok"}),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(deleted.outcome, StickerSetMutationOutcome::Verified);
        assert!(deleted.cleanup_verified);
        assert!(deleted.complete);
    }

    #[test]
    fn story_and_group_call_lifecycles_reread_before_terminal_claims() {
        let story = |is_being_posted: bool| {
            json!({
                "@type":"story","id":9,"poster_chat_id":7,
                "is_being_posted":is_being_posted,"can_be_deleted":true
            })
        };
        let post = StoryAction::PostPhoto {
            chat_id: 7,
            photo_file_id: 6,
            caption: "test",
            privacy: StoryPrivacy::SelectedUsers(&[8]),
            active_period: 86_400,
            is_posted_to_chat_page: false,
            protect_content: true,
        };
        assert_eq!(plan_story_mutation(post).unwrap().risk, RiskClass::Admin);
        let posted = story_mutation_with(post, |method, _| {
            workflow_object(match method {
                "getFile" => json!({
                    "@type":"file","id":6,
                    "remote":{"@type":"remoteFile","is_uploading_completed":true}
                }),
                "postStory" => story(true),
                "getStory" => story(false),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(posted.story_id, Some(9));
        assert_eq!(posted.outcome, StoryMutationOutcome::Verified);

        let mut posts = 0;
        let uncertain = story_mutation_with(post, |method, _| match method {
            "getFile" => workflow_object(json!({
                "@type":"file","id":6,
                "remote":{"@type":"remoteFile","is_uploading_completed":true}
            })),
            "postStory" => {
                posts += 1;
                Err(ChatWorkflowError::Call(RawApiError::Transport(
                    TransportError::ResponseTimeout,
                )))
            }
            "getChatActiveStories" => workflow_object(json!({
                "@type":"chatActiveStories","chat_id":7,
                "stories":[{"@type":"storyInfo","story_id":9}]
            })),
            _ => unreachable!(),
        })
        .unwrap();
        assert_eq!(posts, 1);
        assert_eq!(uncertain.candidate_story_ids, [9]);
        assert!(!uncertain.complete);

        let delete = StoryAction::Delete {
            story_poster_chat_id: 7,
            story_id: 9,
        };
        assert_eq!(
            plan_story_mutation(delete).unwrap().risk,
            RiskClass::Destructive
        );
        let deleted = story_mutation_with(delete, |method, _| {
            workflow_object(match method {
                "getStory" => story(false),
                "deleteStory" => json!({"@type":"ok"}),
                "getChatActiveStories" => {
                    json!({"@type":"chatActiveStories","chat_id":7,"stories":[]})
                }
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert!(deleted.cleanup_verified);

        let mut reads = 0;
        let left = leave_group_call_with(4, |method, _| match method {
            "getGroupCall" => {
                reads += 1;
                workflow_object(json!({
                    "@type":"groupCall","id":4,"is_active":true,
                    "is_joined":reads == 1,"need_rejoin":false
                }))
            }
            "leaveGroupCall" => Err(ChatWorkflowError::Call(RawApiError::Transport(
                TransportError::ResponseTimeout,
            ))),
            _ => unreachable!(),
        })
        .unwrap();
        assert_eq!(left.outcome, Some(GroupCallLeaveOutcome::Verified));
        assert!(left.cleanup_verified);
    }

    #[test]
    fn account_settings_preserve_omissions_and_session_target_is_exact() {
        let settings = |mute_for| {
            json!({
                "@type":"scopeNotificationSettings","mute_for":mute_for,"sound_id":11,
                "show_preview":true,"use_default_mute_stories":true,
                "mute_stories":false,"story_sound_id":12,"show_story_poster":true,
                "disable_pinned_message_notifications":false,
                "disable_mention_notifications":true
            })
        };
        let mut reads = 0;
        let mut sent = Value::Null;
        let changed = set_notification_settings_with(
            NotificationScope::PrivateChats,
            NotificationSettingsPatch {
                mute_for: Some(60),
                ..NotificationSettingsPatch::default()
            },
            |method, request| {
                workflow_object(match method {
                    "getScopeNotificationSettings" => {
                        reads += 1;
                        settings(if reads == 1 { 0 } else { 60 })
                    }
                    "setScopeNotificationSettings" => {
                        sent = request["notification_settings"].clone();
                        json!({"@type":"ok"})
                    }
                    _ => unreachable!(),
                })
            },
        )
        .unwrap();
        assert_eq!(changed.changed_fields, ["mute_for"]);
        assert_eq!(sent["sound_id"], 11);
        assert_eq!(sent["disable_mention_notifications"], true);
        assert_eq!(changed.outcome, SettingsMutationOutcome::Verified);

        let sessions = |include_target| {
            let mut sessions = vec![json!({
                "@type":"session","id":1,"is_current":true,"is_password_pending":false,
                "is_unconfirmed":false,"is_official_application":true,"last_active_date":10,
                "ip_address":"PRIVATE_IP_CANARY","location":"PRIVATE_LOCATION_CANARY"
            })];
            if include_target {
                sessions.push(json!({
                    "@type":"session","id":2,"is_current":false,"is_password_pending":false,
                    "is_unconfirmed":true,"is_official_application":false,"last_active_date":9,
                    "device_model":"PRIVATE_DEVICE_CANARY"
                }));
            }
            json!({"@type":"sessions","sessions":sessions,"inactive_session_ttl_days":30})
        };
        let snapshot = active_sessions_with(|_, _| workflow_object(sessions(true))).unwrap();
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(snapshot.sensitive_metadata_redacted);
        assert!(!serialized.contains("PRIVATE_"));
        assert_eq!(
            plan_terminate_session(2).unwrap().risk,
            RiskClass::AuthSecurity
        );

        let mut lists = 0;
        let mut terminations = 0;
        let terminated = terminate_session_with(2, |method, _| match method {
            "getActiveSessions" => {
                lists += 1;
                workflow_object(sessions(lists == 1))
            }
            "terminateSession" => {
                terminations += 1;
                Err(ChatWorkflowError::Call(RawApiError::Transport(
                    TransportError::ResponseTimeout,
                )))
            }
            _ => unreachable!(),
        })
        .unwrap();
        assert_eq!(terminations, 1);
        assert_eq!(terminated.outcome, SessionTerminationOutcome::Verified);

        let current = terminate_session_with(1, |method, _| match method {
            "getActiveSessions" => workflow_object(sessions(true)),
            _ => unreachable!(),
        })
        .unwrap_err();
        assert!(matches!(
            current,
            ChatWorkflowError::CapabilityDenied {
                capability: "non_current_session"
            }
        ));
    }

    #[test]
    fn business_connections_are_explicit_and_timeout_never_retries_send() {
        let connection = |id: &str, enabled: bool| {
            json!({
                "@type":"businessConnection",
                "id":id,
                "is_enabled":enabled,
                "rights":{
                    "@type":"businessBotRights",
                    "can_reply":true,
                    "can_read_messages":true
                }
            })
        };
        let sent =
            send_business_text_with("first", 7, "CUSTOMER_TEXT_CANARY", |method, request| {
                assert_eq!(
                    request
                        .get("business_connection_id")
                        .or_else(|| request.get("connection_id")),
                    Some(&Value::String("first".to_owned()))
                );
                workflow_object(match method {
                    "getBusinessConnection" => connection("first", true),
                    "sendBusinessMessage" => json!({
                        "@type":"businessMessage",
                        "message":{"@type":"message","id":11,"chat_id":7}
                    }),
                    _ => unreachable!(),
                })
            })
            .unwrap();
        assert_eq!(sent.outcome, BusinessMessageOutcome::Sent);
        assert_eq!(sent.message_id, Some(11));
        assert!(
            !serde_json::to_string(&sent)
                .unwrap()
                .contains("CUSTOMER_TEXT_CANARY")
        );

        let mut reads = 0;
        let mut sends = 0;
        let disconnected = send_business_text_with("second", 7, "hello", |method, request| {
            assert_eq!(
                request
                    .get("business_connection_id")
                    .or_else(|| request.get("connection_id")),
                Some(&Value::String("second".to_owned()))
            );
            match method {
                "getBusinessConnection" => {
                    reads += 1;
                    workflow_object(connection("second", reads == 1))
                }
                "sendBusinessMessage" => {
                    sends += 1;
                    Err(ChatWorkflowError::Call(RawApiError::Transport(
                        TransportError::ResponseTimeout,
                    )))
                }
                _ => unreachable!(),
            }
        })
        .unwrap();
        assert_eq!(sends, 1);
        assert_eq!(reads, 2);
        assert_eq!(disconnected.outcome, BusinessMessageOutcome::CapabilityLost);
        assert!(!disconnected.complete);

        assert!(matches!(
            business_connection_with("first", |_, _| workflow_object(connection("second", true))),
            Err(ChatWorkflowError::UnexpectedResult {
                method: "getBusinessConnection"
            })
        ));
    }

    #[test]
    fn stars_payment_uses_fresh_ledger_and_never_resubmits_after_timeout() {
        let invoice_name = "INVOICE_NAME_CANARY";
        let me = || json!({"@type":"user","id":7});
        let transaction = |id: &str| {
            json!({
                "@type":"starTransaction",
                "id":id,
                "star_amount":{"@type":"starAmount","star_count":-10,"nanostar_count":0},
                "is_refund":false,
                "type":{"@type":"starTransactionTypeBotInvoicePurchase","user_id":9}
            })
        };
        let ledger = |balance: i64, transactions: Vec<Value>| {
            json!({
                "@type":"starTransactions",
                "star_amount":{"@type":"starAmount","star_count":balance,"nanostar_count":0},
                "transactions":transactions,
                "next_offset":""
            })
        };
        let form = json!({
            "@type":"paymentForm",
            "id":11,
            "seller_bot_user_id":9,
            "type":{"@type":"paymentFormTypeStars","star_count":10}
        });

        let plan = plan_star_invoice_payment_with(invoice_name, |method, _| {
            workflow_object(match method {
                "getPaymentForm" => form.clone(),
                "getMe" => me(),
                "getStarTransactions" => ledger(50, vec![transaction("old")]),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(plan.risk, RiskClass::Financial);
        assert_eq!(plan.retry, RetryClass::Reconcile);
        assert_eq!(plan.star_count, 10);
        assert!(!serde_json::to_string(&plan).unwrap().contains(invoice_name));

        let mut reads = 0;
        let mut sends = 0;
        let receipt =
            apply_star_invoice_payment_with(&plan, invoice_name, |method, request| match method {
                "getMe" => workflow_object(me()),
                "getStarTransactions" => {
                    reads += 1;
                    workflow_object(if reads == 1 {
                        ledger(50, vec![transaction("old")])
                    } else {
                        ledger(40, vec![transaction("new"), transaction("old")])
                    })
                }
                "sendPaymentForm" => {
                    sends += 1;
                    assert_eq!(request["credentials"], Value::Null);
                    assert_eq!(request["input_invoice"]["name"], invoice_name);
                    Err(ChatWorkflowError::Call(RawApiError::Transport(
                        TransportError::ResponseTimeout,
                    )))
                }
                _ => unreachable!(),
            })
            .unwrap();
        assert_eq!(sends, 1);
        assert_eq!(reads, 2);
        assert_eq!(receipt.outcome, StarPaymentOutcome::Confirmed);
        assert!(receipt.complete);

        let verification = apply_star_invoice_payment_with(&plan, invoice_name, |method, _| {
            workflow_object(match method {
                "getMe" => me(),
                "getStarTransactions" => ledger(50, vec![transaction("old")]),
                "sendPaymentForm" => json!({
                    "@type":"paymentResult",
                    "success":false,
                    "verification_url":"VERIFICATION_URL_CANARY"
                }),
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(
            verification.outcome,
            StarPaymentOutcome::VerificationRequired
        );
        assert!(
            !serde_json::to_string(&verification)
                .unwrap()
                .contains("VERIFICATION_URL_CANARY")
        );
    }

    #[test]
    fn statistics_follow_async_lineage_to_terminal_graph() {
        let mut calls = Vec::new();
        let result = chat_statistics_with(7, false, true, 13, |method, request| {
            calls.push((method, request["token"].as_str().map(str::to_owned)));
            workflow_object(match method {
                "getChatStatistics" => json!({
                    "@type":"chatStatisticsChannel",
                    "views_by_source_graph":{"@type":"statisticalGraphAsync","token":"first"}
                }),
                "getStatisticalGraph" if request["token"] == "first" => {
                    json!({"@type":"statisticalGraphAsync","token":"second"})
                }
                "getStatisticalGraph" => {
                    json!({"@type":"statisticalGraphData","json_data":"{}","zoom_token":""})
                }
                _ => unreachable!(),
            })
        })
        .unwrap();

        assert_eq!(
            calls,
            [
                ("getChatStatistics", None),
                ("getStatisticalGraph", Some("first".to_owned())),
                ("getStatisticalGraph", Some("second".to_owned()))
            ]
        );
        assert_eq!(result.graph_lineage["first"], ["first", "second"]);
        assert_eq!(
            result.statistics["views_by_source_graph"]["@type"],
            "statisticalGraphData"
        );
        assert!(result.unresolved_tokens.is_empty());
        assert!(result.complete);
    }

    #[test]
    fn resource_statistics_are_read_only_aggregated_and_redacted() {
        let mut methods = Vec::new();
        let snapshot = resource_statistics_with(true, |method, request| {
            methods.push(method);
            workflow_object(match method {
                "getStorageStatisticsFast" => json!({
                    "@type":"storageStatisticsFast",
                    "files_size":100,"file_count":2,"database_size":30,
                    "language_pack_database_size":4,"log_size":5
                }),
                "getDatabaseStatistics" => json!({
                    "@type":"databaseStatistics",
                    "statistics":"DATABASE_REPORT_CANARY"
                }),
                "getNetworkStatistics" => {
                    assert_eq!(request["only_current"], true);
                    json!({
                        "@type":"networkStatistics","since_date":10,
                        "entries":[
                            {"@type":"networkStatisticsEntryFile","sent_bytes":7,"received_bytes":11},
                            {"@type":"networkStatisticsEntryCall","sent_bytes":13,"received_bytes":17}
                        ]
                    })
                }
                _ => unreachable!(),
            })
        })
        .unwrap();
        assert_eq!(
            methods,
            [
                "getStorageStatisticsFast",
                "getDatabaseStatistics",
                "getNetworkStatistics"
            ]
        );
        assert_eq!((snapshot.sent_bytes, snapshot.received_bytes), (20, 28));
        assert!(snapshot.database_report_redacted);
        assert!(
            !serde_json::to_string(&snapshot)
                .unwrap()
                .contains("DATABASE_REPORT_CANARY")
        );
    }

    #[test]
    fn proxy_transition_redacts_endpoints_and_reports_connectivity_divergence() {
        let proxies = |enabled_id: i32| {
            let proxies = [1, 2]
                .into_iter()
                .map(|id| json!({
                    "@type":"addedProxy","id":id,"is_enabled":id == enabled_id,
                    "comment":"PROXY_COMMENT_CANARY",
                    "proxy":{
                        "@type":"proxy","server":"PROXY_HOST_CANARY","port":443,
                        "type":{"@type":"proxyTypeSocks5","username":"USER_CANARY","password":"PASS_CANARY"}
                    }
                }))
                .collect::<Vec<_>>();
            json!({
                "@type":"addedProxies",
                "proxies":proxies
            })
        };
        let snapshot = proxy_snapshot(&proxies(1)).unwrap();
        let serialized = serde_json::to_string(&snapshot).unwrap();
        assert!(snapshot.proxies.iter().all(|proxy| proxy.endpoint_redacted));
        assert!(!serialized.contains("CANARY"));

        let mut reads = 0;
        let mut methods = Vec::new();
        let receipt = set_proxy_enabled_with(Some(2), |method, _| {
            methods.push(method);
            match method {
                "getProxies" => {
                    reads += 1;
                    Ok((
                        TdObject::from_value(proxies(if reads == 1 { 1 } else { 2 })).unwrap(),
                        Some(ConnectionObservation {
                            sequence: 10 + reads,
                            ready: reads == 1,
                        }),
                    ))
                }
                "enableProxy" => Ok((TdObject::from_value(json!({"@type":"ok"})).unwrap(), None)),
                _ => unreachable!(),
            }
        })
        .unwrap();
        assert_eq!(methods, ["getProxies", "enableProxy", "getProxies"]);
        assert_eq!(receipt.rollback_proxy_id, Some(1));
        assert_eq!(
            receipt.outcome,
            ProxyTransitionOutcome::ConnectivityDiverged
        );
        assert!(!receipt.complete);
    }

    #[test]
    fn repeated_or_timed_out_graph_token_is_partial() {
        let repeated = chat_statistics_with(7, false, true, 14, |method, _| {
            workflow_object(if method == "getChatStatistics" {
                json!({"@type":"chatStatisticsChannel","graph":{"@type":"statisticalGraphAsync","token":"same"}})
            } else {
                json!({"@type":"statisticalGraphAsync","token":"same"})
            })
        })
        .unwrap();
        assert_eq!(repeated.unresolved_tokens, ["same"]);
        assert_eq!(repeated.graph_lineage["same"], ["same"]);
        assert!(!repeated.complete);

        let timed_out = chat_statistics_with(7, false, true, 15, |method, _| {
            if method == "getChatStatistics" {
                workflow_object(json!({"@type":"chatStatisticsChannel","graph":{"@type":"statisticalGraphAsync","token":"late"}}))
            } else {
                Err(ChatWorkflowError::Call(RawApiError::Transport(
                    TransportError::ResponseTimeout,
                )))
            }
        })
        .unwrap();
        assert_eq!(timed_out.unresolved_tokens, ["late"]);
        assert!(!timed_out.complete);
    }

    #[test]
    fn missing_capability_is_denied_before_dispatch() {
        let members = supergroup_members_with(
            MembersQuery {
                supergroup_id: 7,
                count: 1,
                page_limit: 1,
            },
            false,
            1,
            |_| unreachable!(),
        )
        .unwrap_err();
        assert!(matches!(
            members,
            ChatWorkflowError::CapabilityDenied {
                capability: "can_get_members"
            }
        ));

        let statistics =
            chat_statistics_with(7, false, false, 1, |_, _| unreachable!()).unwrap_err();
        assert!(matches!(
            statistics,
            ChatWorkflowError::CapabilityDenied {
                capability: "can_get_statistics"
            }
        ));
    }

    struct TerminalWorkflowBackend {
        incoming: VecDeque<String>,
        methods: Arc<Mutex<Vec<String>>>,
        snapshot_calls: usize,
        drop_send_message_response: bool,
        flood_history_once: bool,
    }

    impl TerminalWorkflowBackend {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let methods = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    incoming: VecDeque::new(),
                    methods: Arc::clone(&methods),
                    snapshot_calls: 0,
                    drop_send_message_response: false,
                    flood_history_once: false,
                },
                methods,
            )
        }

        fn push(&mut self, value: Value) {
            self.incoming.push_back(value.to_string());
        }
    }

    impl TdJsonBackend for TerminalWorkflowBackend {
        fn send(&mut self, request: &str) -> Result<(), BackendError> {
            let request: Value = serde_json::from_str(request).unwrap();
            let method = request["@type"].as_str().unwrap();
            self.methods.lock().unwrap().push(method.to_owned());
            let extra = request["@extra"].clone();
            match method {
                "setLogStream" | "closeWebApp" => {
                    self.push(json!({"@type":"ok","@extra":extra}));
                }
                "getOption" => {
                    let value = if request["name"] == "version" {
                        crate::registry::SCHEMA.version
                    } else {
                        crate::registry::SCHEMA.commit
                    };
                    self.push(json!({"@type":"optionValueString","value":value,"@extra":extra}));
                }
                "getCurrentState" => {
                    self.snapshot_calls += 1;
                    let updates = if self.snapshot_calls == 1 {
                        vec![json!({
                            "@type":"updateNewChat",
                            "chat":{"@type":"chat","id":9,"title":"Old title","permissions":{"@type":"chatPermissions","can_change_info":true},"has_protected_content":false,"positions":[],"chat_lists":[],"reply_markup_message_id":0}
                        })]
                    } else {
                        vec![json!({
                            "@type":"updateNewChat",
                            "chat":{"@type":"chat","id":44,"positions":[],"chat_lists":[],"reply_markup_message_id":0}
                        })]
                    };
                    self.push(json!({"@type":"updates","updates":updates,"@extra":extra}));
                }
                "downloadFile" => {
                    self.push(json!({"@type":"file","id":5,"size":10,"expected_size":10,"local":{"@type":"localFile","is_downloading_completed":false,"downloaded_size":0},"remote":{"@type":"remoteFile","is_uploading_completed":true,"uploaded_size":10},"@extra":extra}));
                    self.push(json!({"@type":"updateFile","file":{"@type":"file","id":5,"size":10,"expected_size":10,"local":{"@type":"localFile","is_downloading_completed":true,"downloaded_size":10},"remote":{"@type":"remoteFile","is_uploading_completed":true,"uploaded_size":10}}}));
                }
                "uploadStickerFile" => {
                    self.push(json!({"@type":"file","id":6,"size":20,"expected_size":20,"local":{"@type":"localFile","is_downloading_completed":true,"downloaded_size":20},"remote":{"@type":"remoteFile","is_uploading_completed":false,"uploaded_size":0},"@extra":extra}));
                    self.push(json!({"@type":"updateFile","file":{"@type":"file","id":6,"size":20,"expected_size":20,"local":{"@type":"localFile","is_downloading_completed":true,"downloaded_size":20},"remote":{"@type":"remoteFile","is_uploading_completed":true,"uploaded_size":20}}}));
                }
                "sendBotStartMessage" => {
                    self.push(json!({"@type":"message","id":-7,"chat_id":9,"@extra":extra}));
                    self.push(json!({"@type":"updateMessageSendAcknowledged","chat_id":9,"message_id":-7}));
                    self.push(json!({"@type":"updateMessageSendSucceeded","message":{"@type":"message","id":10,"chat_id":9},"old_message_id":-7}));
                    self.push(json!({
                        "@type":"updateNewMessage",
                        "message":{
                            "@type":"message",
                            "id":20,
                            "chat_id":9,
                            "sender_id":{"@type":"messageSenderUser","user_id":8},
                            "is_outgoing":false,
                            "content":{"@type":"messageText","text":{"@type":"formattedText","text":"PRIVATE_BOT_REPLY_CANARY","entities":[]}},
                            "reply_markup":{"@type":"replyMarkupInlineKeyboard","rows":[[
                                {"@type":"inlineKeyboardButton","text":"ok","type":{"@type":"inlineKeyboardButtonTypeCallback","data":"b2s="}},
                                {"@type":"inlineKeyboardButton","text":"timeout","type":{"@type":"inlineKeyboardButtonTypeCallback","data":"dGltZW91dA=="}}
                            ]]}
                        }
                    }));
                }
                "getCallbackQueryAnswer" => {
                    if request["payload"]["data"] == "dGltZW91dA==" {
                        self.push(json!({"@type":"error","code":502,"message":"BOT_RESPONSE_TIMEOUT","@extra":extra}));
                    } else {
                        self.push(json!({"@type":"callbackQueryAnswer","text":"done","show_alert":false,"url":"","@extra":extra}));
                    }
                }
                "sendMessage" => {
                    if !self.drop_send_message_response {
                        self.push(json!({"@type":"message","id":-8,"chat_id":9,"@extra":extra}));
                        self.push(json!({"@type":"updateMessageSendSucceeded","message":{"@type":"message","id":11,"chat_id":9},"old_message_id":-8}));
                    }
                }
                "getChatHistory" => {
                    if self.flood_history_once {
                        self.flood_history_once = false;
                        self.push(json!({"@type":"error","code":429,"message":"Too Many Requests: retry after 0","@extra":extra}));
                    } else {
                        self.push(json!({"@type":"messages","total_count":1,"messages":[
                            {"@type":"message","id":12,"chat_id":9,"date":10}
                        ],"@extra":extra}));
                    }
                }
                "viewMessages" => {
                    assert_eq!(request["message_ids"], json!([12]));
                    assert_eq!(request["force_read"], true);
                    self.push(json!({"@type":"ok","@extra":extra}));
                }
                "openWebApp" => {
                    self.push(json!({"@type":"webAppInfo","launch_id":11,"url":{"@type":"webAppUrl","url":"https://example.invalid/?tgWebAppData=secret","require_same_origin":true},"@extra":extra}));
                    self.push(json!({"@type":"updateWebAppMessageSent","web_app_launch_id":11}));
                }
                "searchPublicChat" => {
                    self.push(json!({"@type":"updateUser","user":test_user(7,"Old","Name")}));
                    self.push(json!({"@type":"chat","id":70,"type":{"@type":"chatTypePrivate","user_id":7},"@extra":extra}));
                }
                "getUser" => {
                    let user = test_user(request["user_id"].as_i64().unwrap(), "Old", "Name");
                    self.push(json!({"@type":"updateUser","user":user.clone()}));
                    self.push(with_extra(user, extra));
                }
                "getMe" => {
                    let user = test_user(7, "Old", "Name");
                    self.push(json!({"@type":"updateUser","user":user.clone()}));
                    self.push(with_extra(user, extra));
                }
                "getUserFullInfo" => {
                    let info = json!({
                        "@type":"userFullInfo",
                        "can_be_called":true,
                        "supports_video_calls":true,
                        "bio":{"@type":"formattedText","text":"Public bio","entities":[]},
                        "birthdate":{"@type":"birthdate","day":1,"month":2,"year":2000},
                        "note":{"@type":"formattedText","text":"PRIVATE_NOTE_CANARY","entities":[]}
                    });
                    self.push(json!({"@type":"updateUserFullInfo","user_id":7,"user_full_info":info.clone()}));
                    self.push(with_extra(info, extra));
                }
                "setName" => {
                    self.push(json!({
                        "@type":"updateUser",
                        "user":test_user(
                            7,
                            request["first_name"].as_str().unwrap(),
                            request["last_name"].as_str().unwrap()
                        )
                    }));
                    self.push(json!({"@type":"ok","@extra":extra}));
                }
                "setChatTitle" => {
                    self.push(json!({
                        "@type":"updateChatTitle",
                        "chat_id":request["chat_id"],
                        "title":request["title"]
                    }));
                    self.push(json!({"@type":"ok","@extra":extra}));
                }
                _ => unreachable!("unexpected test method {method}"),
            }
            Ok(())
        }

        fn receive(&mut self, timeout: Duration) -> Result<Option<String>, BackendError> {
            if self.incoming.is_empty() {
                std::thread::sleep(timeout.min(Duration::from_millis(1)));
            }
            Ok(self.incoming.pop_front())
        }
    }

    fn test_deadline() -> Instant {
        Instant::now() + Duration::from_secs(3)
    }

    fn test_user(user_id: i64, first_name: &str, last_name: &str) -> Value {
        json!({
            "@type":"user",
            "id":user_id,
            "first_name":first_name,
            "last_name":last_name,
            "phone_number":"PRIVATE_PHONE_CANARY",
            "have_access":true,
            "type":{"@type":"userTypeRegular"}
        })
    }

    fn with_extra(mut value: Value, extra: Value) -> Value {
        value["@extra"] = extra;
        value
    }

    #[test]
    fn user_profile_resolves_redacts_and_verifies_name_update() {
        let (backend, methods) = TerminalWorkflowBackend::new();
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Read, RiskClass::ReversibleMutation],
        );

        let profile = user_profile(
            &mut runtime,
            &policy,
            UserTarget::PublicUsername("person"),
            true,
            test_deadline(),
        )
        .unwrap();
        assert!(profile.complete);
        assert_eq!(profile.user_id, 7);
        assert_eq!(
            profile.private_fields["phone_number"],
            PrivateFieldState::Redacted
        );
        assert_eq!(
            profile.private_fields["birthdate"],
            PrivateFieldState::Redacted
        );
        let serialized = serde_json::to_string(&profile).unwrap();
        assert!(!serialized.contains("PRIVATE_PHONE_CANARY"));
        assert!(!serialized.contains("PRIVATE_NOTE_CANARY"));

        let updated =
            update_profile_name(&mut runtime, &policy, "New", "Name", test_deadline()).unwrap();
        assert_eq!(updated.outcome, ProfileMutationOutcome::Verified);
        assert!(updated.complete);
        assert!(methods.lock().unwrap().ends_with(&[
            "searchPublicChat".to_owned(),
            "getUserFullInfo".to_owned(),
            "getMe".to_owned(),
            "setName".to_owned(),
        ]));
        runtime.shutdown().unwrap();
    }

    #[test]
    fn chat_title_requires_exact_approval_and_matching_update() {
        use crate::approval::{ApprovalReceipt, ApprovalVerifier, approval_payload};

        let (backend, methods) = TerminalWorkflowBackend::new();
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let plan = plan_chat_title(&runtime, 9, "New title").unwrap();
        assert!(plan.changed);

        let request = ValidatedRequest::from_value(json!({
            "@type":"setChatTitle","chat_id":9,"title":"New title"
        }))
        .unwrap();
        let preview = PlanPreview::for_request(&request).unwrap();
        assert_eq!(plan.plan_hash, preview.hash.to_hex());
        let signing = SigningKey::from_bytes(&[9; 32]);
        let verifier = ApprovalVerifier::new(signing.verifying_key().to_bytes()).unwrap();
        let expires = (SystemTime::now() + Duration::from_secs(60))
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let nonce = [4; 16];
        let signature = signing
            .sign(&approval_payload(preview.hash, expires, nonce))
            .to_bytes();
        let approval = verifier
            .verify(
                preview,
                ApprovalReceipt::new(preview.hash, expires, nonce, signature),
                SystemTime::now(),
            )
            .unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Admin],
        )
        .with_approval(approval);

        let receipt = apply_chat_title(&mut runtime, &policy, &plan, test_deadline()).unwrap();
        assert_eq!(receipt.outcome, ChatTitleOutcome::Verified);
        assert!(receipt.complete);
        assert!(
            methods
                .lock()
                .unwrap()
                .iter()
                .any(|method| method == "setChatTitle")
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn file_sticker_bot_and_web_app_wait_for_terminal_updates() {
        let (backend, methods) = TerminalWorkflowBackend::new();
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![
                RiskClass::Read,
                RiskClass::ReversibleMutation,
                RiskClass::Send,
                RiskClass::Presence,
            ],
        );

        let download = download_file(
            &mut runtime,
            &policy,
            DownloadQuery {
                file_id: 5,
                priority: 1,
                offset: 0,
                limit: 0,
            },
            test_deadline(),
        )
        .unwrap();
        assert_eq!(download.source, TerminalSource::OrderedUpdate);
        assert_eq!(download.file["id"], 5);
        assert!(download.size_verified);

        let upload = upload_sticker_file(
            &mut runtime,
            &policy,
            0,
            StickerFormat::Webp,
            InputFileSource::Local(Path::new("/tmp/synthetic-sticker.webp")),
            test_deadline(),
        )
        .unwrap();
        assert_eq!(upload.source, TerminalSource::OrderedUpdate);
        assert_eq!(upload.file["id"], 6);
        assert!(upload.size_verified);

        let bot = start_bot(&mut runtime, &policy, 8, 9, "", test_deadline()).unwrap();
        assert!(bot.complete);
        assert!(matches!(bot.outcome, BotStartOutcome::Succeeded { .. }));

        let sent = send_text_message(&mut runtime, &policy, 9, "hello", test_deadline()).unwrap();
        assert!(sent.complete);
        assert!(matches!(sent.outcome, BotStartOutcome::Succeeded { .. }));

        let history = chat_history(
            &runtime,
            &policy,
            HistoryQuery {
                chat_id: 9,
                only_local: false,
                mark_read: true,
                page: PageOptions {
                    count: 1,
                    min_date: None,
                    page_limit: 100,
                },
            },
            test_deadline(),
        )
        .unwrap();
        assert!(history.complete);

        let mut web_app = open_web_app(
            &mut runtime,
            &policy,
            WebAppRequest {
                chat_id: 9,
                bot_user_id: 8,
                button_url: "https://example.invalid/",
                application_name: "telegram_cli",
                mode: WebAppMode::FullSize,
            },
            test_deadline(),
        )
        .unwrap();
        assert_eq!(format!("{:?}", web_app.launch_url()), "<redacted>");
        assert!(web_app.require_same_origin());
        assert!(web_app.wait_message_sent().unwrap().complete);
        web_app.close().unwrap();

        assert_eq!(
            methods.lock().unwrap().as_slice(),
            [
                "setLogStream",
                "getOption",
                "getOption",
                "getCurrentState",
                "downloadFile",
                "uploadStickerFile",
                "sendBotStartMessage",
                "sendMessage",
                "getChatHistory",
                "viewMessages",
                "openWebApp",
                "closeWebApp"
            ]
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn bot_test_correlates_redacted_reply_and_clicks_recorded_callback_once() {
        let (backend, methods) = TerminalWorkflowBackend::new();
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Send],
        );

        let run =
            start_bot_and_wait_reply(&mut runtime, &policy, 8, 9, "", test_deadline()).unwrap();
        let BotTestOutcome::Passed { reply } = &run.outcome else {
            panic!("expected correlated bot reply")
        };
        assert_eq!(reply.message_id, 20);
        assert_eq!(reply.content_type, "messageText");
        assert_eq!(reply.callback_button_count, 2);
        assert!(run.passed && run.complete);
        assert!(
            !serde_json::to_string(&run)
                .unwrap()
                .contains("PRIVATE_BOT_REPLY_CANARY")
        );

        let answered = click_bot_callback(&runtime, &policy, 9, 20, 0, 0, test_deadline()).unwrap();
        assert!(answered.passed && answered.complete);
        assert!(matches!(
            answered.outcome,
            CallbackOutcome::Answered {
                text_present: true,
                show_alert: false,
                url_present: false,
            }
        ));
        let timeout = click_bot_callback(&runtime, &policy, 9, 20, 0, 1, test_deadline()).unwrap();
        assert_eq!(timeout.outcome, CallbackOutcome::BotTimedOut);
        assert!(!timeout.passed);
        assert!(timeout.complete);
        assert_eq!(
            methods
                .lock()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "getCallbackQueryAnswer")
                .count(),
            2
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn web_app_handoff_defers_close_until_browser_finally_step() {
        let (backend, methods) = TerminalWorkflowBackend::new();
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Presence],
        );
        let lease = open_web_app(
            &mut runtime,
            &policy,
            WebAppRequest {
                chat_id: 9,
                bot_user_id: 8,
                button_url: "https://example.invalid/",
                application_name: "telegram_cli",
                mode: WebAppMode::FullSize,
            },
            test_deadline(),
        )
        .unwrap();
        let launch_id = lease.handoff();
        assert!(
            !methods
                .lock()
                .unwrap()
                .iter()
                .any(|method| method == "closeWebApp")
        );
        close_web_app_launch(&runtime, &policy, launch_id, test_deadline()).unwrap();
        assert!(
            methods
                .lock()
                .unwrap()
                .iter()
                .any(|method| method == "closeWebApp")
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn send_timeout_is_uncertain_and_is_not_repeated() {
        let (mut backend, methods) = TerminalWorkflowBackend::new();
        backend.drop_send_message_response = true;
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Send],
        );

        let receipt = send_text_message(
            &mut runtime,
            &policy,
            9,
            "once",
            Instant::now() + Duration::from_millis(20),
        )
        .unwrap();
        assert_eq!(receipt.outcome, BotStartOutcome::Uncertain);
        assert_eq!(receipt.old_message_id, None);
        assert!(!receipt.complete);
        assert_eq!(
            methods
                .lock()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "sendMessage")
                .count(),
            1
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn safe_read_retries_tdlib_flood_once() {
        let (mut backend, methods) = TerminalWorkflowBackend::new();
        backend.flood_history_once = true;
        let runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Read],
        );

        let page = chat_history(
            &runtime,
            &policy,
            HistoryQuery {
                chat_id: 9,
                only_local: false,
                mark_read: false,
                page: PageOptions {
                    count: 1,
                    min_date: None,
                    page_limit: 100,
                },
            },
            test_deadline(),
        )
        .unwrap();
        assert!(page.complete);
        assert_eq!(
            methods
                .lock()
                .unwrap()
                .iter()
                .filter(|method| method.as_str() == "getChatHistory")
                .count(),
            2
        );
        runtime.shutdown().unwrap();
    }

    #[test]
    fn gapped_state_blocks_workflow_until_snapshot_resync() {
        let (backend, methods) = TerminalWorkflowBackend::new();
        let mut runtime = CoreRuntime::start(backend, test_deadline()).unwrap();
        let policy = RawPolicy::new(
            crate::registry::AccountKind::RegularUser,
            vec![RiskClass::Read, RiskClass::Send],
        );

        let gap = runtime.mark_update_gap();
        let blocked = start_bot(&mut runtime, &policy, 8, 9, "", test_deadline()).unwrap_err();
        assert!(matches!(
            blocked,
            ChatWorkflowError::ResyncRequired { gap_after_sequence }
                if gap_after_sequence == gap.after_sequence.map(|value| value.get())
        ));
        assert!(
            !methods
                .lock()
                .unwrap()
                .iter()
                .any(|method| method == "sendBotStartMessage")
        );

        let receipt = resync_after_gap(&mut runtime, &policy, test_deadline()).unwrap();
        assert_eq!(
            receipt.gap_after_sequence,
            gap.after_sequence.map(|value| value.get())
        );
        assert_eq!(receipt.snapshot_updates, 1);
        assert!(receipt.complete);
        assert!(runtime.state().gap().is_none());
        assert!(runtime.state().chat(44).is_some());
        runtime.shutdown().unwrap();
    }
}
