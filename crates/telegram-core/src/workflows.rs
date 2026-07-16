//! Curated stateful workflows поверх общего TDJSON call.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::path::Path;
use std::time::{Instant, SystemTime};

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::approval::PlanPreview;
use crate::authorization::SensitiveString;
use crate::raw_api::{td_call, td_call_with_boundary, RawApiError, RawPolicy};
use crate::reducer::{ChatList, ChatListPosition, MessageSendKey, MessageSendState, ReducerError};
use crate::registry::{RetryClass, RiskClass, TdObject, ValidatedRequest};
use crate::runtime::{CoreRuntime, RuntimeError};
use crate::transport::TransportError;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResolutionState {
    Chat { chat_id: i64 },
    InvitePreview { chat_id: Option<i64> },
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChatResolution {
    pub state: ResolutionState,
    pub raw: TdObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MembershipResult {
    pub state: MembershipState,
    pub raw: TdObject,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatListTerminal {
    AllChatsLoaded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatListSnapshot {
    pub positions: Vec<ChatListPosition>,
    pub load_calls: usize,
    pub terminal: ChatListTerminal,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct ForumTopicCursor {
    pub date: i32,
    pub message_id: i64,
    pub topic_id: i32,
}

pub struct ForumTopicsQuery<'query> {
    pub chat_id: i64,
    pub query: &'query str,
    pub count: usize,
    pub page_limit: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ForumTopicsPage {
    pub topics: Vec<Value>,
    pub pages: usize,
    pub total_count: i32,
    pub next_cursor: Option<ForumTopicCursor>,
    pub boundary: PageBoundary,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TopicMutationOutcome {
    AlreadyApplied,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ForumTopicMutationReceipt {
    pub chat_id: i64,
    pub topic_id: i32,
    pub is_closed: bool,
    pub outcome: TopicMutationOutcome,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatInspectionStatus {
    Complete,
    MembershipRequired,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ChatInspection {
    pub status: ChatInspectionStatus,
    pub resolution: ChatResolution,
    pub cached_chat: Option<Value>,
    pub full_info: Option<TdObject>,
    pub used_open_lease: bool,
}

#[derive(Clone, Copy)]
pub enum UserTarget<'value> {
    SelfUser,
    Id(i64),
    PublicUsername(&'value str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PrivateFieldState {
    Unavailable,
    Redacted,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct UserProfileView {
    pub user_id: i64,
    pub user: Value,
    pub full_info: Option<Value>,
    pub private_fields: BTreeMap<&'static str, PrivateFieldState>,
    pub sequence: Option<u64>,
    pub freshness: Freshness,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileMutationOutcome {
    AlreadyApplied,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ProfileNameReceipt {
    pub user_id: i64,
    pub outcome: ProfileMutationOutcome,
    pub sequence: Option<u64>,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTitlePlan {
    pub chat_id: i64,
    pub current_title: String,
    pub desired_title: String,
    pub sequence: u64,
    pub changed: bool,
    pub risk: RiskClass,
    pub retry: RetryClass,
    pub plan_hash: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatTitleOutcome {
    AlreadyApplied,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatTitleReceipt {
    pub chat_id: i64,
    pub title: String,
    pub outcome: ChatTitleOutcome,
    pub sequence: Option<u64>,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageOptions {
    pub count: usize,
    pub min_date: Option<i32>,
    pub page_limit: i32,
}

pub struct HistoryQuery {
    pub chat_id: i64,
    pub only_local: bool,
    pub mark_read: bool,
    pub page: PageOptions,
}

pub struct ChatSearchQuery<'query> {
    pub chat_id: i64,
    pub query: &'query str,
    pub mark_read: bool,
    pub page: PageOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PageBoundary {
    Count,
    Date,
    Exhausted,
    NoProgress,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MessagePage {
    pub messages: Vec<Value>,
    pub pages: usize,
    pub next_from_message_id: Option<i64>,
    pub boundary: PageBoundary,
    pub content_redacted: bool,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    ServerSnapshot,
    OrderedUpdate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MembersQuery {
    pub supergroup_id: i64,
    pub count: usize,
    pub page_limit: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct MembersSnapshot {
    pub members: Vec<Value>,
    pub pages: usize,
    pub total_count: i32,
    pub boundary: PageBoundary,
    pub complete: bool,
    pub capability_sequence: u64,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct StatisticsSnapshot {
    pub statistics: Value,
    pub graph_lineage: BTreeMap<String, Vec<String>>,
    pub unresolved_tokens: Vec<String>,
    pub complete: bool,
    pub capability_sequence: u64,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
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

#[derive(Clone, Copy)]
pub struct DownloadQuery {
    pub file_id: i32,
    pub priority: i32,
    pub offset: i64,
    pub limit: i64,
}

pub enum InputFileSource<'source> {
    Id(i32),
    Remote(&'source str),
    Local(&'source Path),
    Generated {
        original_path: &'source Path,
        conversion: &'source str,
        expected_size: i64,
    },
}

#[derive(Clone, Copy)]
pub enum StickerFormat {
    Webp,
    Tgs,
    Webm,
}

#[derive(Clone, PartialEq, Serialize)]
pub struct FileTransferReceipt {
    pub file: Value,
    pub sequence: Option<u64>,
    pub source: TerminalSource,
    pub size_verified: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferCancellationOutcome {
    AlreadyStopped,
    Verified,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TransferCancellationReceipt {
    pub file_id: i32,
    pub outcome: TransferCancellationOutcome,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BotStartOutcome {
    Succeeded { message: Value },
    Failed { message: Value, error: Value },
    Uncertain,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct BotStartReceipt {
    pub old_message_id: i64,
    pub outcome: BotStartOutcome,
    pub source: Option<TerminalSource>,
    pub complete: bool,
    pub observed_at: SystemTime,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BotReplySummary {
    pub message_id: i64,
    pub chat_id: i64,
    pub sender_user_id: i64,
    pub content_type: String,
    pub callback_button_count: usize,
    pub content_redacted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum BotTestOutcome {
    Passed { reply: BotReplySummary },
    TriggerFailed,
    TriggerUncertain,
    ReplyTimedOut,
    Gap,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BotTestReceipt {
    pub boundary_sequence: u64,
    pub trigger_old_message_id: i64,
    pub sent_message_id: Option<i64>,
    pub outcome: BotTestOutcome,
    pub passed: bool,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum CallbackOutcome {
    Answered {
        text_present: bool,
        show_alert: bool,
        url_present: bool,
    },
    BotTimedOut,
    Uncertain,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub struct CallbackReceipt {
    pub outcome: CallbackOutcome,
    pub passed: bool,
    pub complete: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TextMessageReceipt {
    pub old_message_id: Option<i64>,
    pub outcome: BotStartOutcome,
    pub source: Option<TerminalSource>,
    pub complete: bool,
    pub observed_at: SystemTime,
}

#[derive(Clone, Copy)]
pub enum WebAppMode {
    Compact,
    FullSize,
    FullScreen,
}

pub struct WebAppRequest<'request> {
    pub chat_id: i64,
    pub bot_user_id: i64,
    pub button_url: &'request str,
    pub application_name: &'request str,
    pub mode: WebAppMode,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WebAppMessageReceipt {
    pub launch_id: i64,
    pub source: Option<TerminalSource>,
    pub complete: bool,
    pub observed_at: SystemTime,
}

pub struct WebAppLease<'runtime> {
    runtime: &'runtime mut CoreRuntime,
    policy: &'runtime RawPolicy,
    launch_id: i64,
    launch_url: SensitiveString,
    require_same_origin: bool,
    baseline_sequence: u64,
    deadline: Instant,
    active: bool,
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
    require_resynced(runtime)?;
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
    require_resynced(runtime)?;
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
    require_resynced(runtime)?;
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

pub fn forum_topics(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: ForumTopicsQuery<'_>,
    deadline: Instant,
) -> Result<ForumTopicsPage, ChatWorkflowError> {
    require_resynced(runtime)?;
    forum_topics_with(query, |request| {
        invoke(runtime, policy, "getForumTopics", request, deadline)
    })
}

pub fn set_forum_topic_closed(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
    deadline: Instant,
) -> Result<ForumTopicMutationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    set_forum_topic_closed_with(chat_id, topic_id, is_closed, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub fn user_profile(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    target: UserTarget<'_>,
    include_full_info: bool,
    deadline: Instant,
) -> Result<UserProfileView, ChatWorkflowError> {
    require_resynced(runtime)?;
    let (user, freshness) = match target {
        UserTarget::SelfUser => (
            call_and_apply(runtime, policy, "getMe", json!({"@type":"getMe"}), deadline)?
                .into_value(),
            Freshness::ServerSnapshot,
        ),
        UserTarget::Id(user_id) => match runtime.state().user(user_id) {
            Some(user) => (user.value.clone(), Freshness::OrderedUpdate),
            None => (
                call_and_apply(
                    runtime,
                    policy,
                    "getUser",
                    json!({"@type":"getUser","user_id":user_id}),
                    deadline,
                )?
                .into_value(),
                Freshness::ServerSnapshot,
            ),
        },
        UserTarget::PublicUsername(username) => {
            let chat = call_and_apply(
                runtime,
                policy,
                "searchPublicChat",
                json!({"@type":"searchPublicChat","username":username_value(username)?}),
                deadline,
            )?
            .into_value();
            let user_id = match chat["type"]["@type"].as_str() {
                Some("chatTypePrivate") => required_i64(&chat["type"], "user_id", "chat")?,
                _ => return Err(ChatWorkflowError::InvalidTarget),
            };
            match runtime.state().user(user_id) {
                Some(user) => (user.value.clone(), Freshness::OrderedUpdate),
                None => (
                    call_and_apply(
                        runtime,
                        policy,
                        "getUser",
                        json!({"@type":"getUser","user_id":user_id}),
                        deadline,
                    )?
                    .into_value(),
                    Freshness::ServerSnapshot,
                ),
            }
        }
    };
    if user["@type"] != "user" {
        return Err(ChatWorkflowError::UnexpectedResult { method: "getUser" });
    }
    let user_id = required_i64(&user, "id", "user")?;
    let full_info = include_full_info
        .then(|| {
            call_and_apply(
                runtime,
                policy,
                "getUserFullInfo",
                json!({"@type":"getUserFullInfo","user_id":user_id}),
                deadline,
            )
            .map(TdObject::into_value)
        })
        .transpose()?;
    let sequence = runtime
        .state()
        .user(user_id)
        .map(|value| value.sequence.get());
    Ok(redacted_profile(
        user_id, user, full_info, sequence, freshness,
    ))
}

pub fn update_profile_name(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    first_name: &str,
    last_name: &str,
    deadline: Instant,
) -> Result<ProfileNameReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    if !(1..=64).contains(&first_name.chars().count()) || last_name.chars().count() > 64 {
        return Err(ChatWorkflowError::InvalidProfileInput);
    }
    let current =
        call_and_apply(runtime, policy, "getMe", json!({"@type":"getMe"}), deadline)?.into_value();
    let user_id = required_i64(&current, "id", "getMe")?;
    if current["first_name"] == first_name && current["last_name"] == last_name {
        return Ok(ProfileNameReceipt {
            user_id,
            outcome: ProfileMutationOutcome::AlreadyApplied,
            sequence: runtime
                .state()
                .user(user_id)
                .map(|value| value.sequence.get()),
            complete: true,
        });
    }
    let baseline = last_sequence(runtime);
    expect_ok(
        call_and_apply(
            runtime,
            policy,
            "setName",
            json!({"@type":"setName","first_name":first_name,"last_name":last_name}),
            deadline,
        )?,
        "setName",
    )?;
    loop {
        if let Some(sequence) =
            matching_profile_name(runtime, user_id, first_name, last_name, baseline)
        {
            return Ok(ProfileNameReceipt {
                user_id,
                outcome: ProfileMutationOutcome::Verified,
                sequence: Some(sequence),
                complete: true,
            });
        }
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => {
                return Ok(ProfileNameReceipt {
                    user_id,
                    outcome: ProfileMutationOutcome::Uncertain,
                    sequence: None,
                    complete: false,
                });
            }
            Err(error) => return Err(ChatWorkflowError::Runtime(error)),
        }
    }
}

pub fn plan_chat_title(
    runtime: &CoreRuntime,
    chat_id: i64,
    desired_title: &str,
) -> Result<ChatTitlePlan, ChatWorkflowError> {
    require_resynced(runtime)?;
    if !(1..=128).contains(&desired_title.chars().count()) {
        return Err(ChatWorkflowError::InvalidChatConfiguration);
    }
    let chat = runtime
        .state()
        .chat(chat_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "chat",
        })?;
    if !required_bool(&chat.value["permissions"], "can_change_info", "chat")? {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "can_change_info",
        });
    }
    let current_title = required_string(&chat.value, "title", "chat")?.to_owned();
    let preview = title_preview(chat_id, desired_title)?;
    Ok(ChatTitlePlan {
        chat_id,
        changed: current_title != desired_title,
        current_title,
        desired_title: desired_title.to_owned(),
        sequence: chat.sequence.get(),
        risk: preview.risk,
        retry: preview.retry,
        plan_hash: preview.hash.to_hex(),
    })
}

pub fn apply_chat_title(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    plan: &ChatTitlePlan,
    deadline: Instant,
) -> Result<ChatTitleReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    if !plan.changed {
        return Ok(chat_title_receipt(
            plan,
            ChatTitleOutcome::AlreadyApplied,
            Some(plan.sequence),
        ));
    }
    let current = runtime
        .state()
        .chat(plan.chat_id)
        .filter(|chat| {
            chat.sequence.get() == plan.sequence
                && chat.value["title"] == plan.current_title
                && chat.value["permissions"]["can_change_info"] == true
        })
        .ok_or(ChatWorkflowError::PlanStale)?;
    let preview = title_preview(plan.chat_id, &plan.desired_title)?;
    if preview.hash.to_hex() != plan.plan_hash {
        return Err(ChatWorkflowError::PlanStale);
    }
    let baseline = current.sequence.get();
    expect_ok(
        call_and_apply(
            runtime,
            policy,
            "setChatTitle",
            json!({
                "@type":"setChatTitle",
                "chat_id":plan.chat_id,
                "title":plan.desired_title
            }),
            deadline,
        )?,
        "setChatTitle",
    )?;
    loop {
        if let Some(chat) = runtime.state().chat(plan.chat_id).filter(|chat| {
            chat.sequence.get() > baseline && chat.value["title"] == plan.desired_title
        }) {
            return Ok(chat_title_receipt(
                plan,
                ChatTitleOutcome::Verified,
                Some(chat.sequence.get()),
            ));
        }
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => {
                return Ok(chat_title_receipt(plan, ChatTitleOutcome::Uncertain, None));
            }
            Err(error) => return Err(ChatWorkflowError::Runtime(error)),
        }
    }
}

pub fn chat_history(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: HistoryQuery,
    deadline: Instant,
) -> Result<MessagePage, ChatWorkflowError> {
    require_resynced(runtime)?;
    let mark_read = query.mark_read;
    let chat_id = query.chat_id;
    let protected_content = chat_has_protected_content(runtime, chat_id)?;
    let mut result = chat_history_with(query, |request| {
        invoke(runtime, policy, "getChatHistory", request, deadline)
    })?;
    redact_message_content(protected_content, &mut result);
    mark_message_page_read(runtime, policy, chat_id, mark_read, &result, deadline)?;
    Ok(result)
}

pub fn search_chat_messages(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: ChatSearchQuery<'_>,
    deadline: Instant,
) -> Result<MessagePage, ChatWorkflowError> {
    require_resynced(runtime)?;
    let mark_read = query.mark_read;
    let chat_id = query.chat_id;
    let protected_content = chat_has_protected_content(runtime, chat_id)?;
    let mut result = search_chat_messages_with(query, |request| {
        invoke(runtime, policy, "searchChatMessages", request, deadline)
    })?;
    redact_message_content(protected_content, &mut result);
    mark_message_page_read(runtime, policy, chat_id, mark_read, &result, deadline)?;
    Ok(result)
}

pub fn supergroup_members(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: MembersQuery,
    deadline: Instant,
) -> Result<MembersSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    let capability = runtime
        .state()
        .supergroup_full_info(query.supergroup_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "supergroupFullInfo",
        })?;
    let allowed =
        capability.value["can_get_members"]
            .as_bool()
            .ok_or(ChatWorkflowError::InvalidResult {
                method: "supergroupFullInfo",
                field: "can_get_members",
            })?;
    supergroup_members_with(query, allowed, capability.sequence.get(), |request| {
        invoke(runtime, policy, "getSupergroupMembers", request, deadline)
    })
}

pub fn chat_statistics(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    is_dark: bool,
    deadline: Instant,
) -> Result<StatisticsSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    let chat = runtime
        .state()
        .chat(chat_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "chat",
        })?;
    let supergroup_id = match chat.value["type"]["@type"].as_str() {
        Some("chatTypeSupergroup") => required_i64(&chat.value["type"], "supergroup_id", "chat")?,
        _ => {
            return Err(ChatWorkflowError::CapabilityDenied {
                capability: "can_get_statistics",
            });
        }
    };
    let capability = runtime.state().supergroup_full_info(supergroup_id).ok_or(
        ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "supergroupFullInfo",
        },
    )?;
    let allowed = capability.value["can_get_statistics"].as_bool().ok_or(
        ChatWorkflowError::InvalidResult {
            method: "supergroupFullInfo",
            field: "can_get_statistics",
        },
    )?;
    chat_statistics_with(
        chat_id,
        is_dark,
        allowed,
        capability.sequence.get(),
        |method, request| invoke(runtime, policy, method, request, deadline),
    )
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

pub fn download_file(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    query: DownloadQuery,
    deadline: Instant,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    if !(1..=32).contains(&query.priority) || query.offset < 0 || query.limit < 0 {
        return Err(ChatWorkflowError::InvalidFileTransfer);
    }
    let baseline_sequence = last_sequence(runtime);
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"downloadFile",
            "file_id":query.file_id,
            "priority":query.priority,
            "offset":query.offset,
            "limit":query.limit,
            "synchronous":false
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("downloadFile", response)?.into_value();
    wait_file_terminal(
        runtime,
        response,
        baseline_sequence,
        FileDirection::Download,
        deadline,
    )
}

pub fn upload_sticker_file(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    user_id: i64,
    format: StickerFormat,
    source: InputFileSource<'_>,
    deadline: Instant,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let baseline_sequence = last_sequence(runtime);
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"uploadStickerFile",
            "user_id":user_id,
            "sticker_format":format.tdjson(),
            "sticker":source.tdjson()?
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("uploadStickerFile", response)?.into_value();
    wait_file_terminal(
        runtime,
        response,
        baseline_sequence,
        FileDirection::Upload,
        deadline,
    )
}

pub fn cancel_download(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    file_id: i32,
    only_if_pending: bool,
    deadline: Instant,
) -> Result<TransferCancellationReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    cancel_download_with(file_id, only_if_pending, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub fn start_bot(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    bot_user_id: i64,
    chat_id: i64,
    parameter: &str,
    deadline: Instant,
) -> Result<BotStartReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"sendBotStartMessage",
            "bot_user_id":bot_user_id,
            "chat_id":chat_id,
            "parameter":parameter
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("sendBotStartMessage", response)?;
    if response.as_value()["@type"] != "message" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "sendBotStartMessage",
        });
    }
    let key = MessageSendKey {
        chat_id: required_i64(response.as_value(), "chat_id", "sendBotStartMessage")?,
        old_message_id: required_i64(response.as_value(), "id", "sendBotStartMessage")?,
    };
    wait_message_send(runtime, key, deadline)
}

pub fn start_bot_and_wait_reply(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    bot_user_id: i64,
    chat_id: i64,
    parameter: &str,
    deadline: Instant,
) -> Result<BotTestReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let boundary_sequence = runtime
        .state()
        .last_sequence()
        .map(crate::reducer::UpdateSequence::get)
        .unwrap_or_default();
    let trigger = start_bot(runtime, policy, bot_user_id, chat_id, parameter, deadline)?;
    let sent_message_id = match &trigger.outcome {
        BotStartOutcome::Succeeded { message } => {
            Some(required_i64(message, "id", "sendBotStartMessage")?)
        }
        BotStartOutcome::Failed { .. } => {
            return Ok(bot_test_receipt(
                boundary_sequence,
                trigger.old_message_id,
                None,
                BotTestOutcome::TriggerFailed,
            ));
        }
        BotStartOutcome::Uncertain => {
            return Ok(bot_test_receipt(
                boundary_sequence,
                trigger.old_message_id,
                None,
                BotTestOutcome::TriggerUncertain,
            ));
        }
    };
    loop {
        if runtime.state().gap().is_some() {
            return Ok(bot_test_receipt(
                boundary_sequence,
                trigger.old_message_id,
                sent_message_id,
                BotTestOutcome::Gap,
            ));
        }
        if let Some(reply) = bot_reply_after(runtime, boundary_sequence, chat_id, bot_user_id) {
            return Ok(bot_test_receipt(
                boundary_sequence,
                trigger.old_message_id,
                sent_message_id,
                BotTestOutcome::Passed { reply },
            ));
        }
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => {
                return Ok(bot_test_receipt(
                    boundary_sequence,
                    trigger.old_message_id,
                    sent_message_id,
                    BotTestOutcome::ReplyTimedOut,
                ));
            }
            Err(error) => return Err(ChatWorkflowError::Runtime(error)),
        }
    }
}

pub fn click_bot_callback(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    message_id: i64,
    row: usize,
    column: usize,
    deadline: Instant,
) -> Result<CallbackReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let data = callback_button_data(runtime, chat_id, message_id, row, column)
        .ok_or(ChatWorkflowError::InvalidBotInteraction)?;
    let response = match td_call(
        runtime,
        policy,
        json!({
            "@type": "getCallbackQueryAnswer",
            "chat_id": chat_id,
            "message_id": message_id,
            "payload": {"@type": "callbackQueryPayloadData", "data": data},
        }),
        deadline,
    ) {
        Ok(response) => response,
        Err(RawApiError::Transport(TransportError::ResponseTimeout)) => {
            return Ok(callback_receipt(CallbackOutcome::Uncertain));
        }
        Err(error) => return Err(ChatWorkflowError::Call(error)),
    };
    if response.as_value()["@type"] == "error" && response.as_value()["code"] == 502 {
        return Ok(callback_receipt(CallbackOutcome::BotTimedOut));
    }
    let response = checked_response("getCallbackQueryAnswer", response)?;
    if response.as_value()["@type"] != "callbackQueryAnswer" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getCallbackQueryAnswer",
        });
    }
    Ok(callback_receipt(CallbackOutcome::Answered {
        text_present: !required_string(response.as_value(), "text", "getCallbackQueryAnswer")?
            .is_empty(),
        show_alert: required_bool(response.as_value(), "show_alert", "getCallbackQueryAnswer")?,
        url_present: !required_string(response.as_value(), "url", "getCallbackQueryAnswer")?
            .is_empty(),
    }))
}

pub fn send_text_message(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    text: &str,
    deadline: Instant,
) -> Result<TextMessageReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let request = json!({
        "@type":"sendMessage",
        "chat_id":chat_id,
        "topic_id":null,
        "reply_to":null,
        "options":null,
        "reply_markup":null,
        "input_message_content":{
            "@type":"inputMessageText",
            "text":{"@type":"formattedText","text":text,"entities":[]},
            "link_preview_options":null,
            "clear_draft":false
        }
    });
    let (response, boundary) = match td_call_with_boundary(runtime, policy, request, deadline) {
        Ok(value) => value,
        Err(RawApiError::Transport(TransportError::ResponseTimeout)) => {
            return Ok(text_message_receipt(
                None,
                BotStartOutcome::Uncertain,
                false,
            ));
        }
        Err(error) => return Err(ChatWorkflowError::Call(error)),
    };
    let response = checked_response("sendMessage", response)?;
    if response.as_value()["@type"] != "message" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "sendMessage",
        });
    }
    let key = MessageSendKey {
        chat_id: required_i64(response.as_value(), "chat_id", "sendMessage")?,
        old_message_id: required_i64(response.as_value(), "id", "sendMessage")?,
    };
    match runtime.apply_through_boundary(boundary, deadline) {
        Ok(_) => {}
        Err(RuntimeError::DeadlineExceeded) => {
            return Ok(text_message_receipt(
                Some(key.old_message_id),
                BotStartOutcome::Uncertain,
                false,
            ));
        }
        Err(error) => return Err(ChatWorkflowError::Runtime(error)),
    }
    let receipt = wait_message_send(runtime, key, deadline)?;
    Ok(text_message_receipt(
        Some(receipt.old_message_id),
        receipt.outcome,
        receipt.complete,
    ))
}

pub fn open_web_app<'runtime>(
    runtime: &'runtime mut CoreRuntime,
    policy: &'runtime RawPolicy,
    request: WebAppRequest<'_>,
    deadline: Instant,
) -> Result<WebAppLease<'runtime>, ChatWorkflowError> {
    require_resynced(runtime)?;
    let baseline_sequence = last_sequence(runtime);
    let (response, boundary) = td_call_with_boundary(
        runtime,
        policy,
        json!({
            "@type":"openWebApp",
            "chat_id":request.chat_id,
            "bot_user_id":request.bot_user_id,
            "url":request.button_url,
            "topic_id":null,
            "reply_to":null,
            "parameters":{
                "@type":"webAppOpenParameters",
                "theme":null,
                "application_name":request.application_name,
                "mode":request.mode.tdjson()
            }
        }),
        deadline,
    )
    .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let response = checked_response("openWebApp", response)?;
    if response.as_value()["@type"] != "webAppInfo" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "openWebApp",
        });
    }
    let url = &response.as_value()["url"];
    if url["@type"] != "webAppUrl" {
        return Err(ChatWorkflowError::InvalidResult {
            method: "openWebApp",
            field: "url",
        });
    }
    Ok(WebAppLease {
        runtime,
        policy,
        launch_id: required_i64(response.as_value(), "launch_id", "openWebApp")?,
        launch_url: SensitiveString::new(required_string(url, "url", "openWebApp")?),
        require_same_origin: required_bool(url, "require_same_origin", "openWebApp")?,
        baseline_sequence,
        deadline,
        active: true,
    })
}

impl WebAppLease<'_> {
    pub fn launch_id(&self) -> i64 {
        self.launch_id
    }

    pub fn launch_url(&self) -> &SensitiveString {
        &self.launch_url
    }

    pub fn require_same_origin(&self) -> bool {
        self.require_same_origin
    }

    pub fn wait_message_sent(&mut self) -> Result<WebAppMessageReceipt, ChatWorkflowError> {
        loop {
            if self
                .runtime
                .state()
                .web_app_message_sent(self.launch_id)
                .is_some_and(|sequence| sequence.get() > self.baseline_sequence)
            {
                return Ok(WebAppMessageReceipt {
                    launch_id: self.launch_id,
                    source: Some(TerminalSource::OrderedUpdate),
                    complete: true,
                    observed_at: SystemTime::now(),
                });
            }
            match self.runtime.next_event_until(self.deadline) {
                Ok(_) => {}
                Err(RuntimeError::DeadlineExceeded) => {
                    return Ok(WebAppMessageReceipt {
                        launch_id: self.launch_id,
                        source: None,
                        complete: false,
                        observed_at: SystemTime::now(),
                    });
                }
                Err(error) => return Err(ChatWorkflowError::Runtime(error)),
            }
        }
    }

    pub fn close(mut self) -> Result<(), ChatWorkflowError> {
        close_web_app(self.runtime, self.policy, self.launch_id, self.deadline)?;
        self.active = false;
        Ok(())
    }
}

impl Drop for WebAppLease<'_> {
    fn drop(&mut self) {
        if self.active {
            self.active = false;
            let _ = close_web_app(self.runtime, self.policy, self.launch_id, self.deadline);
        }
    }
}

#[derive(Clone, Copy)]
enum FileDirection {
    Download,
    Upload,
}

impl InputFileSource<'_> {
    fn tdjson(&self) -> Result<Value, ChatWorkflowError> {
        Ok(match self {
            Self::Id(id) => json!({"@type":"inputFileId","id":id}),
            Self::Remote(id) if !id.is_empty() => {
                json!({"@type":"inputFileRemote","id":id})
            }
            Self::Local(file) => json!({"@type":"inputFileLocal","path":file_path(file)?}),
            Self::Generated {
                original_path,
                conversion,
                expected_size,
            } if *expected_size >= 0 => json!({
                "@type":"inputFileGenerated",
                "original_path":file_path(original_path)?,
                "conversion":conversion,
                "expected_size":expected_size
            }),
            _ => return Err(ChatWorkflowError::InvalidFileTransfer),
        })
    }
}

fn file_path(path: &Path) -> Result<&str, ChatWorkflowError> {
    path.to_str().ok_or(ChatWorkflowError::InvalidFileTransfer)
}

impl StickerFormat {
    fn tdjson(self) -> Value {
        let kind = match self {
            Self::Webp => "stickerFormatWebp",
            Self::Tgs => "stickerFormatTgs",
            Self::Webm => "stickerFormatWebm",
        };
        json!({"@type":kind})
    }
}

impl WebAppMode {
    fn tdjson(self) -> Value {
        let mode = match self {
            Self::Compact => "webAppOpenModeCompact",
            Self::FullSize => "webAppOpenModeFullSize",
            Self::FullScreen => "webAppOpenModeFullScreen",
        };
        json!({"@type":mode})
    }
}

fn last_sequence(runtime: &CoreRuntime) -> u64 {
    runtime
        .state()
        .last_sequence()
        .map(|sequence| sequence.get())
        .unwrap_or(0)
}

fn mark_message_page_read(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    mark_read: bool,
    page: &MessagePage,
    deadline: Instant,
) -> Result<(), ChatWorkflowError> {
    if !mark_read || !page.complete || page.messages.is_empty() {
        return Ok(());
    }
    let message_ids = page
        .messages
        .iter()
        .map(|message| required_i64(message, "id", "message page"))
        .collect::<Result<Vec<_>, _>>()?;
    expect_ok(
        invoke(
            runtime,
            policy,
            "viewMessages",
            json!({
                "@type":"viewMessages",
                "chat_id":chat_id,
                "message_ids":message_ids,
                "source":null,
                "force_read":true
            }),
            deadline,
        )?,
        "viewMessages",
    )
}

fn chat_has_protected_content(
    runtime: &CoreRuntime,
    chat_id: i64,
) -> Result<bool, ChatWorkflowError> {
    let chat = runtime
        .state()
        .chat(chat_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "chat",
        })?;
    required_bool(&chat.value, "has_protected_content", "chat")
}

fn redact_message_content(protected: bool, page: &mut MessagePage) {
    if !protected {
        return;
    }
    for message in &mut page.messages {
        let content_type = message["content"]["@type"].clone();
        message["content"] = json!({"@type":content_type,"redacted":true});
    }
    page.content_redacted = true;
}

fn require_resynced(runtime: &CoreRuntime) -> Result<(), ChatWorkflowError> {
    match runtime.state().gap() {
        Some(gap) => Err(ChatWorkflowError::ResyncRequired {
            gap_after_sequence: gap.after_sequence.map(|sequence| sequence.get()),
        }),
        None => Ok(()),
    }
}

fn wait_file_terminal(
    runtime: &mut CoreRuntime,
    response: Value,
    baseline_sequence: u64,
    direction: FileDirection,
    deadline: Instant,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    if response["@type"] != "file" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "file transfer",
        });
    }
    let file_id = i32::try_from(required_i64(&response, "id", "file transfer")?).map_err(|_| {
        ChatWorkflowError::InvalidResult {
            method: "file transfer",
            field: "id",
        }
    })?;
    if file_complete(&response, direction)? {
        return file_receipt(response, None, TerminalSource::Response, direction);
    }
    loop {
        if let Some(file) = runtime
            .state()
            .file(file_id)
            .filter(|file| file.sequence.get() > baseline_sequence)
        {
            if file_complete(&file.value, direction)? {
                return file_receipt(
                    file.value.clone(),
                    Some(file.sequence.get()),
                    TerminalSource::OrderedUpdate,
                    direction,
                );
            }
        }
        runtime
            .next_event_until(deadline)
            .map_err(ChatWorkflowError::Runtime)?;
    }
}

fn file_complete(file: &Value, direction: FileDirection) -> Result<bool, ChatWorkflowError> {
    let (part, field) = match direction {
        FileDirection::Download => (&file["local"], "is_downloading_completed"),
        FileDirection::Upload => (&file["remote"], "is_uploading_completed"),
    };
    required_bool(part, field, "file transfer")
}

fn file_receipt(
    file: Value,
    sequence: Option<u64>,
    source: TerminalSource,
    direction: FileDirection,
) -> Result<FileTransferReceipt, ChatWorkflowError> {
    let size_verified = file_size_verified(&file, direction)?;
    Ok(FileTransferReceipt {
        file,
        sequence,
        source,
        size_verified,
        complete: true,
        observed_at: SystemTime::now(),
        freshness: match source {
            TerminalSource::Response => Freshness::ServerSnapshot,
            TerminalSource::OrderedUpdate => Freshness::OrderedUpdate,
        },
    })
}

fn file_size_verified(file: &Value, direction: FileDirection) -> Result<bool, ChatWorkflowError> {
    let size = required_i64(file, "size", "file transfer")?;
    let expected = if size > 0 {
        size
    } else {
        required_i64(file, "expected_size", "file transfer")?
    };
    if expected == 0 {
        return Ok(false);
    }
    let (part, field) = match direction {
        FileDirection::Download => (&file["local"], "downloaded_size"),
        FileDirection::Upload => (&file["remote"], "uploaded_size"),
    };
    if required_i64(part, field, "file transfer")? < expected {
        return Err(ChatWorkflowError::InvalidResult {
            method: "file transfer",
            field: "terminal_size",
        });
    }
    Ok(true)
}

fn cancel_download_with(
    file_id: i32,
    only_if_pending: bool,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<TransferCancellationReceipt, ChatWorkflowError> {
    let request = || json!({"@type":"getFile","file_id":file_id});
    let active = |file: TdObject| {
        if file.as_value()["@type"] != "file" {
            return Err(ChatWorkflowError::UnexpectedResult { method: "getFile" });
        }
        required_bool(
            &file.as_value()["local"],
            "is_downloading_active",
            "getFile",
        )
    };
    if !call("getFile", request()).and_then(active)? {
        return Ok(transfer_cancellation_receipt(
            file_id,
            TransferCancellationOutcome::AlreadyStopped,
        ));
    }
    match call(
        "cancelDownloadFile",
        json!({
            "@type":"cancelDownloadFile",
            "file_id":file_id,
            "only_if_pending":only_if_pending
        }),
    ) {
        Ok(response) => expect_ok(response, "cancelDownloadFile")?,
        Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {}
        Err(error) => return Err(error),
    }
    let outcome = match call("getFile", request()).and_then(active) {
        Ok(false) => TransferCancellationOutcome::Verified,
        Ok(true)
        | Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {
            TransferCancellationOutcome::Uncertain
        }
        Err(error) => return Err(error),
    };
    Ok(transfer_cancellation_receipt(file_id, outcome))
}

fn transfer_cancellation_receipt(
    file_id: i32,
    outcome: TransferCancellationOutcome,
) -> TransferCancellationReceipt {
    TransferCancellationReceipt {
        file_id,
        outcome,
        complete: outcome != TransferCancellationOutcome::Uncertain,
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

fn bot_start_receipt(
    old_message_id: i64,
    outcome: BotStartOutcome,
    complete: bool,
) -> BotStartReceipt {
    BotStartReceipt {
        old_message_id,
        outcome,
        source: complete.then_some(TerminalSource::OrderedUpdate),
        complete,
        observed_at: SystemTime::now(),
    }
}

fn text_message_receipt(
    old_message_id: Option<i64>,
    outcome: BotStartOutcome,
    complete: bool,
) -> TextMessageReceipt {
    TextMessageReceipt {
        old_message_id,
        outcome,
        source: complete.then_some(TerminalSource::OrderedUpdate),
        complete,
        observed_at: SystemTime::now(),
    }
}

fn close_web_app(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    launch_id: i64,
    deadline: Instant,
) -> Result<(), ChatWorkflowError> {
    expect_ok(
        invoke(
            runtime,
            policy,
            "closeWebApp",
            json!({"@type":"closeWebApp","web_app_launch_id":launch_id}),
            deadline,
        )?,
        "closeWebApp",
    )
}

fn forum_topics_with(
    query: ForumTopicsQuery<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ForumTopicsPage, ChatWorkflowError> {
    if query.count == 0 || !(1..=100).contains(&query.page_limit) {
        return Err(ChatWorkflowError::InvalidPageOptions);
    }
    let mut cursor = ForumTopicCursor {
        date: 0,
        message_id: 0,
        topic_id: 0,
    };
    let mut seen_cursors = BTreeSet::from([cursor]);
    let mut seen_topics = BTreeSet::new();
    let mut topics = Vec::new();
    let mut pages = 0;
    loop {
        let response = call(json!({
            "@type":"getForumTopics",
            "chat_id":query.chat_id,
            "query":query.query,
            "offset_date":cursor.date,
            "offset_message_id":cursor.message_id,
            "offset_forum_topic_id":cursor.topic_id,
            "limit":query.page_limit
        }))?;
        pages += 1;
        if response.as_value()["@type"] != "forumTopics" {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getForumTopics",
            });
        }
        let total_count = required_i32(response.as_value(), "total_count", "getForumTopics")?;
        let page =
            response.as_value()["topics"]
                .as_array()
                .ok_or(ChatWorkflowError::InvalidResult {
                    method: "getForumTopics",
                    field: "topics",
                })?;
        for topic in page {
            if topic["@type"] != "forumTopic" || topic["info"]["@type"] != "forumTopicInfo" {
                return Err(ChatWorkflowError::InvalidResult {
                    method: "getForumTopics",
                    field: "topics[].@type",
                });
            }
            let topic_id = required_i32(&topic["info"], "forum_topic_id", "getForumTopics")?;
            if seen_topics.insert(topic_id) {
                topics.push(topic.clone());
                if topics.len() == query.count {
                    let next_cursor = forum_topic_cursor(response.as_value())?;
                    return Ok(forum_topics_page(
                        topics,
                        pages,
                        total_count,
                        Some(next_cursor),
                        PageBoundary::Count,
                    ));
                }
            }
        }
        let next_cursor = forum_topic_cursor(response.as_value())?;
        if next_cursor
            == (ForumTopicCursor {
                date: 0,
                message_id: 0,
                topic_id: 0,
            })
        {
            return Ok(forum_topics_page(
                topics,
                pages,
                total_count,
                None,
                PageBoundary::Exhausted,
            ));
        }
        if !seen_cursors.insert(next_cursor) {
            return Ok(forum_topics_page(
                topics,
                pages,
                total_count,
                Some(next_cursor),
                PageBoundary::NoProgress,
            ));
        }
        cursor = next_cursor;
    }
}

fn forum_topic_cursor(value: &Value) -> Result<ForumTopicCursor, ChatWorkflowError> {
    Ok(ForumTopicCursor {
        date: required_i32(value, "next_offset_date", "getForumTopics")?,
        message_id: required_i64(value, "next_offset_message_id", "getForumTopics")?,
        topic_id: required_i32(value, "next_offset_forum_topic_id", "getForumTopics")?,
    })
}

fn forum_topics_page(
    topics: Vec<Value>,
    pages: usize,
    total_count: i32,
    next_cursor: Option<ForumTopicCursor>,
    boundary: PageBoundary,
) -> ForumTopicsPage {
    ForumTopicsPage {
        topics,
        pages,
        total_count,
        next_cursor,
        boundary,
        complete: boundary != PageBoundary::NoProgress,
    }
}

fn set_forum_topic_closed_with(
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<ForumTopicMutationReceipt, ChatWorkflowError> {
    let request = || json!({"@type":"getForumTopic","chat_id":chat_id,"forum_topic_id":topic_id});
    if call("getForumTopic", request()).and_then(|topic| forum_topic_closed(topic.as_value()))?
        == is_closed
    {
        return Ok(forum_topic_mutation_receipt(
            chat_id,
            topic_id,
            is_closed,
            TopicMutationOutcome::AlreadyApplied,
        ));
    }
    let mutation = call(
        "toggleForumTopicIsClosed",
        json!({
            "@type":"toggleForumTopicIsClosed",
            "chat_id":chat_id,
            "forum_topic_id":topic_id,
            "is_closed":is_closed
        }),
    );
    match mutation {
        Ok(response) => expect_ok(response, "toggleForumTopicIsClosed")?,
        Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {}
        Err(error) => return Err(error),
    }
    let outcome = match call("getForumTopic", request())
        .and_then(|topic| forum_topic_closed(topic.as_value()))
    {
        Ok(actual) if actual == is_closed => TopicMutationOutcome::Verified,
        Ok(_)
        | Err(ChatWorkflowError::Call(RawApiError::Transport(TransportError::ResponseTimeout))) => {
            TopicMutationOutcome::Uncertain
        }
        Err(error) => return Err(error),
    };
    Ok(forum_topic_mutation_receipt(
        chat_id, topic_id, is_closed, outcome,
    ))
}

fn forum_topic_closed(value: &Value) -> Result<bool, ChatWorkflowError> {
    if value["@type"] != "forumTopic" || value["info"]["@type"] != "forumTopicInfo" {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getForumTopic",
        });
    }
    required_bool(&value["info"], "is_closed", "getForumTopic")
}

fn forum_topic_mutation_receipt(
    chat_id: i64,
    topic_id: i32,
    is_closed: bool,
    outcome: TopicMutationOutcome,
) -> ForumTopicMutationReceipt {
    ForumTopicMutationReceipt {
        chat_id,
        topic_id,
        is_closed,
        outcome,
        complete: outcome != TopicMutationOutcome::Uncertain,
    }
}

fn supergroup_members_with(
    query: MembersQuery,
    allowed: bool,
    capability_sequence: u64,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<MembersSnapshot, ChatWorkflowError> {
    if !allowed {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "can_get_members",
        });
    }
    if query.count == 0 || !(1..=200).contains(&query.page_limit) {
        return Err(ChatWorkflowError::InvalidPageOptions);
    }

    let mut offset = 0_i32;
    let mut pages = 0;
    let mut total_count: i32;
    let mut members = Vec::new();
    let mut seen = BTreeSet::new();
    let boundary = loop {
        let response = call(json!({
            "@type":"getSupergroupMembers",
            "supergroup_id":query.supergroup_id,
            "filter":null,
            "offset":offset,
            "limit":query.page_limit
        }))?;
        pages += 1;
        if response.as_value()["@type"] != "chatMembers" {
            return Err(ChatWorkflowError::UnexpectedResult {
                method: "getSupergroupMembers",
            });
        }
        total_count = i32::try_from(required_i64(
            response.as_value(),
            "total_count",
            "getSupergroupMembers",
        )?)
        .map_err(|_| ChatWorkflowError::InvalidResult {
            method: "getSupergroupMembers",
            field: "total_count",
        })?;
        let page =
            response.as_value()["members"]
                .as_array()
                .ok_or(ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members",
                })?;
        for member in page {
            if member["@type"] != "chatMember" {
                return Err(ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members[].@type",
                });
            }
            let key = serde_json::to_string(&member["member_id"]).map_err(|_| {
                ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members[].member_id",
                }
            })?;
            if seen.insert(key) {
                members.push(member.clone());
                if members.len() == query.count {
                    break;
                }
            }
        }
        offset = offset
            .checked_add(i32::try_from(page.len()).map_err(|_| {
                ChatWorkflowError::InvalidResult {
                    method: "getSupergroupMembers",
                    field: "members",
                }
            })?)
            .ok_or(ChatWorkflowError::InvalidPageOptions)?;
        if members.len() == query.count {
            break PageBoundary::Count;
        }
        if offset >= total_count {
            break PageBoundary::Exhausted;
        }
        if page.is_empty() {
            break PageBoundary::NoProgress;
        }
    };
    Ok(MembersSnapshot {
        members,
        pages,
        total_count,
        boundary,
        complete: boundary != PageBoundary::NoProgress,
        capability_sequence,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

fn chat_statistics_with(
    chat_id: i64,
    is_dark: bool,
    allowed: bool,
    capability_sequence: u64,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<StatisticsSnapshot, ChatWorkflowError> {
    if !allowed {
        return Err(ChatWorkflowError::CapabilityDenied {
            capability: "can_get_statistics",
        });
    }
    let mut statistics = call(
        "getChatStatistics",
        json!({"@type":"getChatStatistics","chat_id":chat_id,"is_dark":is_dark}),
    )?
    .into_value();
    if !matches!(
        statistics["@type"].as_str(),
        Some("chatStatisticsSupergroup" | "chatStatisticsChannel")
    ) {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getChatStatistics",
        });
    }

    let mut initial_tokens = BTreeSet::new();
    collect_async_tokens(&statistics, &mut initial_tokens)?;
    let mut resolved = BTreeMap::<String, Value>::new();
    let mut unresolved = BTreeSet::new();
    let mut graph_lineage = BTreeMap::new();
    for initial in initial_tokens {
        let mut token = initial.clone();
        let mut lineage = Vec::new();
        let terminal = loop {
            if let Some(graph) = resolved.get(&token) {
                break Some(graph.clone());
            }
            if lineage.contains(&token) {
                break None;
            }
            lineage.push(token.clone());
            let graph = match call(
                "getStatisticalGraph",
                json!({"@type":"getStatisticalGraph","chat_id":chat_id,"token":token,"x":0}),
            ) {
                Ok(graph) => graph.into_value(),
                Err(ChatWorkflowError::Call(RawApiError::Transport(
                    TransportError::ResponseTimeout,
                ))) => break None,
                Err(error) => return Err(error),
            };
            match graph["@type"].as_str() {
                Some("statisticalGraphData" | "statisticalGraphError") => {
                    for seen in &lineage {
                        resolved.insert(seen.clone(), graph.clone());
                    }
                    break Some(graph);
                }
                Some("statisticalGraphAsync") => {
                    token = required_string(&graph, "token", "getStatisticalGraph")?.to_owned();
                }
                _ => {
                    return Err(ChatWorkflowError::UnexpectedResult {
                        method: "getStatisticalGraph",
                    });
                }
            }
        };
        if let Some(graph) = terminal {
            replace_async_graph(&mut statistics, &initial, &graph);
        } else {
            unresolved.insert(initial.clone());
        }
        graph_lineage.insert(initial, lineage);
    }
    Ok(StatisticsSnapshot {
        statistics,
        graph_lineage,
        complete: unresolved.is_empty(),
        unresolved_tokens: unresolved.into_iter().collect(),
        capability_sequence,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

fn collect_async_tokens(
    value: &Value,
    tokens: &mut BTreeSet<String>,
) -> Result<(), ChatWorkflowError> {
    match value {
        Value::Array(values) => {
            for value in values {
                collect_async_tokens(value, tokens)?;
            }
        }
        Value::Object(object)
            if object.get("@type").and_then(Value::as_str) == Some("statisticalGraphAsync") =>
        {
            tokens.insert(required_string(value, "token", "getChatStatistics")?.to_owned());
        }
        Value::Object(object) => {
            for value in object.values() {
                collect_async_tokens(value, tokens)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn replace_async_graph(value: &mut Value, token: &str, graph: &Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                replace_async_graph(value, token, graph);
            }
        }
        Value::Object(object)
            if object.get("@type").and_then(Value::as_str) == Some("statisticalGraphAsync")
                && object.get("token").and_then(Value::as_str) == Some(token) =>
        {
            *value = graph.clone();
        }
        Value::Object(object) => {
            for value in object.values_mut() {
                replace_async_graph(value, token, graph);
            }
        }
        _ => {}
    }
}

fn chat_history_with(
    query: HistoryQuery,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<MessagePage, ChatWorkflowError> {
    validate_page_options(query.page)?;
    let mut cursor = 0;
    let mut pages = 0;
    let mut messages = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_cursors = BTreeSet::from([0]);
    loop {
        let response = call(json!({
            "@type":"getChatHistory",
            "chat_id":query.chat_id,
            "from_message_id":cursor,
            "offset":0,
            "limit":query.page.page_limit,
            "only_local":query.only_local
        }))?;
        pages += 1;
        let page = message_values(&response, "messages", "getChatHistory")?;
        let progress = append_messages(page, query.page, &mut seen, &mut messages)?;
        if let Some(boundary) = progress.boundary {
            return Ok(message_page(messages, pages, progress.cursor, boundary));
        }
        let Some(next) = progress.cursor else {
            return Ok(message_page(
                messages,
                pages,
                None,
                PageBoundary::NoProgress,
            ));
        };
        if !seen_cursors.insert(next) || !progress.advanced {
            return Ok(message_page(
                messages,
                pages,
                Some(next),
                PageBoundary::NoProgress,
            ));
        }
        cursor = next;
    }
}

fn search_chat_messages_with(
    query: ChatSearchQuery<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<MessagePage, ChatWorkflowError> {
    validate_page_options(query.page)?;
    let mut cursor = 0;
    let mut pages = 0;
    let mut messages = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seen_cursors = BTreeSet::from([0]);
    loop {
        let response = call(json!({
            "@type":"searchChatMessages",
            "chat_id":query.chat_id,
            "topic_id":null,
            "query":query.query,
            "sender_id":null,
            "from_message_id":cursor,
            "offset":0,
            "limit":query.page.page_limit,
            "filter":null
        }))?;
        pages += 1;
        let page = message_values(&response, "foundChatMessages", "searchChatMessages")?;
        let progress = append_messages(page, query.page, &mut seen, &mut messages)?;
        let next = required_i64(
            response.as_value(),
            "next_from_message_id",
            "searchChatMessages",
        )?;
        if let Some(boundary) = progress.boundary {
            return Ok(message_page(messages, pages, Some(next), boundary));
        }
        if next == 0 {
            return Ok(message_page(messages, pages, None, PageBoundary::Exhausted));
        }
        if !seen_cursors.insert(next) {
            return Ok(message_page(
                messages,
                pages,
                Some(next),
                PageBoundary::NoProgress,
            ));
        }
        cursor = next;
    }
}

struct PageProgress {
    cursor: Option<i64>,
    boundary: Option<PageBoundary>,
    advanced: bool,
}

fn append_messages(
    page: &[Value],
    options: PageOptions,
    seen: &mut BTreeSet<i64>,
    messages: &mut Vec<Value>,
) -> Result<PageProgress, ChatWorkflowError> {
    let mut cursor = None;
    let mut advanced = false;
    for message in page {
        if message["@type"] != "message" {
            return Err(ChatWorkflowError::InvalidResult {
                method: "message page",
                field: "messages[].@type",
            });
        }
        let id = required_i64(message, "id", "message page")?;
        let date = i32::try_from(required_i64(message, "date", "message page")?).map_err(|_| {
            ChatWorkflowError::InvalidResult {
                method: "message page",
                field: "messages[].date",
            }
        })?;
        cursor = Some(cursor.map_or(id, |known: i64| known.min(id)));
        if options.min_date.is_some_and(|minimum| date < minimum) {
            return Ok(PageProgress {
                cursor,
                boundary: Some(PageBoundary::Date),
                advanced,
            });
        }
        if seen.insert(id) {
            messages.push(message.clone());
            advanced = true;
            if messages.len() == options.count {
                return Ok(PageProgress {
                    cursor,
                    boundary: Some(PageBoundary::Count),
                    advanced,
                });
            }
        }
    }
    Ok(PageProgress {
        cursor,
        boundary: None,
        advanced,
    })
}

fn message_values<'response>(
    response: &'response TdObject,
    expected: &'static str,
    method: &'static str,
) -> Result<&'response [Value], ChatWorkflowError> {
    if response.as_value()["@type"] != expected {
        return Err(ChatWorkflowError::UnexpectedResult { method });
    }
    match response.as_value().get("messages") {
        Some(Value::Array(messages)) => Ok(messages),
        Some(Value::Null) => Ok(&[]),
        _ => Err(ChatWorkflowError::InvalidResult {
            method,
            field: "messages",
        }),
    }
}

fn validate_page_options(options: PageOptions) -> Result<(), ChatWorkflowError> {
    if options.count == 0 || !(1..=100).contains(&options.page_limit) {
        return Err(ChatWorkflowError::InvalidPageOptions);
    }
    Ok(())
}

fn message_page(
    messages: Vec<Value>,
    pages: usize,
    next_from_message_id: Option<i64>,
    boundary: PageBoundary,
) -> MessagePage {
    MessagePage {
        messages,
        pages,
        next_from_message_id,
        boundary,
        content_redacted: false,
        complete: boundary != PageBoundary::NoProgress,
    }
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

fn redacted_profile(
    user_id: i64,
    user: Value,
    full_info: Option<Value>,
    sequence: Option<u64>,
    freshness: Freshness,
) -> UserProfileView {
    const USER_FIELDS: &[&str] = &[
        "id",
        "first_name",
        "last_name",
        "usernames",
        "status",
        "is_contact",
        "is_mutual_contact",
        "is_close_friend",
        "verification_status",
        "is_premium",
        "is_support",
        "restricts_new_chats",
        "paid_message_star_count",
        "have_access",
        "type",
    ];
    const FULL_INFO_FIELDS: &[&str] = &[
        "can_be_called",
        "supports_video_calls",
        "has_private_calls",
        "has_private_forwards",
        "has_restricted_voice_and_video_note_messages",
        "need_phone_number_privacy_exception",
        "bio",
        "personal_chat_id",
        "gift_count",
        "group_in_common_count",
        "incoming_paid_message_star_count",
        "outgoing_paid_message_star_count",
        "bot_info",
    ];
    let mut private_fields = BTreeMap::from([(
        "phone_number",
        private_field_state(user.get("phone_number")),
    )]);
    if let Some(info) = &full_info {
        for field in ["birthdate", "note", "business_info"] {
            private_fields.insert(field, private_field_state(info.get(field)));
        }
    }
    UserProfileView {
        user_id,
        user: selected_fields(&user, USER_FIELDS),
        full_info: full_info.map(|value| selected_fields(&value, FULL_INFO_FIELDS)),
        private_fields,
        sequence,
        freshness,
        complete: true,
    }
}

fn selected_fields(value: &Value, fields: &[&str]) -> Value {
    let fields = fields
        .iter()
        .filter_map(|field| {
            value
                .get(*field)
                .map(|value| ((*field).to_owned(), value.clone()))
        })
        .collect::<Map<_, _>>();
    Value::Object(fields)
}

fn private_field_state(value: Option<&Value>) -> PrivateFieldState {
    match value {
        None | Some(Value::Null) => PrivateFieldState::Unavailable,
        Some(Value::String(value)) if value.is_empty() => PrivateFieldState::Unavailable,
        _ => PrivateFieldState::Redacted,
    }
}

fn matching_profile_name(
    runtime: &CoreRuntime,
    user_id: i64,
    first_name: &str,
    last_name: &str,
    baseline: u64,
) -> Option<u64> {
    let user = runtime.state().user(user_id)?;
    (user.sequence.get() > baseline
        && user.value["first_name"] == first_name
        && user.value["last_name"] == last_name)
        .then_some(user.sequence.get())
}

fn bot_reply_after(
    runtime: &CoreRuntime,
    boundary_sequence: u64,
    chat_id: i64,
    bot_user_id: i64,
) -> Option<BotReplySummary> {
    runtime
        .state()
        .unknown_updates()
        .iter()
        .find(|update| {
            update.sequence.get() > boundary_sequence
                && update.value["@type"] == "updateNewMessage"
                && update.value["message"]["chat_id"] == chat_id
                && update.value["message"]["is_outgoing"] == false
                && update.value["message"]["sender_id"]["@type"] == "messageSenderUser"
                && update.value["message"]["sender_id"]["user_id"] == bot_user_id
        })
        .and_then(|update| {
            let message = &update.value["message"];
            Some(BotReplySummary {
                message_id: message["id"].as_i64()?,
                chat_id,
                sender_user_id: bot_user_id,
                content_type: message["content"]["@type"].as_str()?.to_owned(),
                callback_button_count: callback_button_count(message),
                content_redacted: true,
            })
        })
}

fn callback_button_count(message: &Value) -> usize {
    message["reply_markup"]["rows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_array)
        .flatten()
        .filter(|button| button["type"]["@type"] == "inlineKeyboardButtonTypeCallback")
        .count()
}

fn callback_button_data(
    runtime: &CoreRuntime,
    chat_id: i64,
    message_id: i64,
    row: usize,
    column: usize,
) -> Option<String> {
    let message = runtime
        .state()
        .unknown_updates()
        .iter()
        .rev()
        .find_map(|update| {
            (update.value["@type"] == "updateNewMessage"
                && update.value["message"]["chat_id"] == chat_id
                && update.value["message"]["id"] == message_id)
                .then_some(&update.value["message"])
        })?;
    let button = message["reply_markup"]["rows"]
        .as_array()?
        .get(row)?
        .as_array()?
        .get(column)?;
    (button["type"]["@type"] == "inlineKeyboardButtonTypeCallback")
        .then(|| button["type"]["data"].as_str().map(str::to_owned))?
}

fn bot_test_receipt(
    boundary_sequence: u64,
    trigger_old_message_id: i64,
    sent_message_id: Option<i64>,
    outcome: BotTestOutcome,
) -> BotTestReceipt {
    let passed = matches!(outcome, BotTestOutcome::Passed { .. });
    let complete = !matches!(
        outcome,
        BotTestOutcome::TriggerUncertain | BotTestOutcome::Gap
    );
    BotTestReceipt {
        boundary_sequence,
        trigger_old_message_id,
        sent_message_id,
        outcome,
        passed,
        complete,
    }
}

fn callback_receipt(outcome: CallbackOutcome) -> CallbackReceipt {
    CallbackReceipt {
        passed: matches!(outcome, CallbackOutcome::Answered { .. }),
        complete: outcome != CallbackOutcome::Uncertain,
        outcome,
    }
}

fn title_preview(chat_id: i64, title: &str) -> Result<PlanPreview, ChatWorkflowError> {
    let request = ValidatedRequest::from_value(
        json!({"@type":"setChatTitle","chat_id":chat_id,"title":title}),
    )
    .map_err(|_| ChatWorkflowError::InvalidChatConfiguration)?;
    PlanPreview::for_request(&request).map_err(|_| ChatWorkflowError::InvalidChatConfiguration)
}

fn chat_title_receipt(
    plan: &ChatTitlePlan,
    outcome: ChatTitleOutcome,
    sequence: Option<u64>,
) -> ChatTitleReceipt {
    ChatTitleReceipt {
        chat_id: plan.chat_id,
        title: plan.desired_title.clone(),
        outcome,
        sequence,
        complete: outcome != ChatTitleOutcome::Uncertain,
    }
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

    use super::*;
    use crate::registry::{capability, CapabilityDisposition, RiskClass};
    use crate::transport::{BackendError, TdJsonBackend};
    use ed25519_dalek::{Signer, SigningKey};

    fn object(value: Value) -> Result<TdObject, RawApiError> {
        Ok(TdObject::from_value(value).unwrap())
    }

    fn workflow_object(value: Value) -> Result<TdObject, ChatWorkflowError> {
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
        assert!(!serde_json::to_string(&page)
            .unwrap()
            .contains("PROTECTED_CONTENT_CANARY"));
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
        assert!(file_size_verified(
            &json!({
                "size":10,"expected_size":10,
                "local":{"downloaded_size":9},"remote":{"uploaded_size":0}
            }),
            FileDirection::Download,
        )
        .is_err());
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
                    self.push(json!({"@type":"messages","total_count":1,"messages":[
                        {"@type":"message","id":12,"chat_id":9,"date":10}
                    ],"@extra":extra}));
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
        use crate::approval::{approval_payload, ApprovalReceipt, ApprovalVerifier};

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
        assert!(methods
            .lock()
            .unwrap()
            .iter()
            .any(|method| method == "setChatTitle"));
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
        assert!(!serde_json::to_string(&run)
            .unwrap()
            .contains("PRIVATE_BOT_REPLY_CANARY"));

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
        assert!(!methods
            .lock()
            .unwrap()
            .iter()
            .any(|method| method == "sendBotStartMessage"));

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
