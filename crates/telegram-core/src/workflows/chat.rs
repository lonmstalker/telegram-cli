//! chat workflows.

use super::*;

#[derive(Clone, Copy)]
pub enum ChatTarget<'value> {
    Id(i64),
    PublicUsername(&'value str),
    PublicLink(&'value str),
}

#[derive(Clone, Copy)]
pub enum MembershipTarget<'value> {
    ChatId(i64),
    InviteLink(&'value str),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatVisibility {
    Public,
    NonPublic,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InviteAccess {
    Accessible,
    PreviewOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatIdentity {
    pub chat_id: i64,
    pub title: String,
    pub kind: ChatListEntryKind,
    pub visibility: ChatVisibility,
    pub active_usernames: Vec<String>,
    pub canonical_public_url: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatResolution {
    pub chat: ChatIdentity,
    pub freshness: Freshness,
    pub complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct InvitePreview {
    pub chat_id: Option<i64>,
    pub title: String,
    pub kind: ChatListEntryKind,
    pub visibility: ChatVisibility,
    pub access: InviteAccess,
    pub accessible_for: i32,
    pub creates_join_request: bool,
    pub complete: bool,
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
    pub fn submission_complete(self) -> bool {
        !matches!(self, Self::Unknown)
    }

    pub fn membership_complete(self) -> bool {
        matches!(self, Self::Member { .. } | Self::Declined)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MembershipResult {
    pub chat_id: Option<i64>,
    pub state: MembershipState,
    pub submission_complete: bool,
    pub membership_complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipStatusState {
    Member,
    NotMember,
    Banned,
    Unresolved,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MembershipStatus {
    pub chat_id: Option<i64>,
    pub state: MembershipStatusState,
    pub freshness: Freshness,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LeaveChatOutcome {
    AlreadyNotMember,
    VerifiedLeft,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LeaveChatReceipt {
    pub chat_id: i64,
    pub outcome: LeaveChatOutcome,
    pub sequence: Option<u64>,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatListTerminal {
    AllChatsLoaded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatListEntryKind {
    Private,
    BasicGroup,
    Supergroup,
    Channel,
    Secret,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatListEntry {
    pub chat_id: i64,
    pub title: String,
    pub kind: ChatListEntryKind,
    pub is_pinned: bool,
    pub order: i64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatListSnapshot {
    pub positions: Vec<ChatListPosition>,
    pub entries: Vec<ChatListEntry>,
    pub load_calls: usize,
    pub terminal: ChatListTerminal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChatFullInfoKind {
    User,
    BasicGroup,
    Supergroup,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ChatInspection {
    pub resolution: ChatResolution,
    pub full_info_kind: ChatFullInfoKind,
    pub used_open_lease: bool,
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

impl ChatInspection {
    pub fn complete(&self) -> bool {
        self.complete
    }
}

pub fn resolve(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    target: ChatTarget<'_>,
    deadline: Instant,
) -> Result<ChatResolution, ChatWorkflowError> {
    require_resynced(runtime)?;
    let (method, request) = resolution_request(target)?;
    let (raw, boundary) = td_call_with_boundary(runtime, policy, request, deadline)
        .map_err(ChatWorkflowError::Call)?;
    runtime
        .apply_through_boundary(boundary, deadline)
        .map_err(ChatWorkflowError::Runtime)?;
    let raw = checked_response(method, raw)?;
    resolution_from_raw(Some(runtime), target, method, &raw)
}

pub fn preview_invite_link(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    invite_link: &str,
    deadline: Instant,
) -> Result<InvitePreview, ChatWorkflowError> {
    preview_invite_link_with(invite_link, |request| {
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

pub fn membership_status(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    target: MembershipTarget<'_>,
    deadline: Instant,
) -> Result<MembershipStatus, ChatWorkflowError> {
    require_resynced(runtime)?;
    membership_status_with(target, |request| {
        let method = match request["@type"].as_str() {
            Some("checkChatInviteLink") => "checkChatInviteLink",
            Some("getChat") => "getChat",
            Some("getBasicGroup") => "getBasicGroup",
            Some("getSupergroup") => "getSupergroup",
            _ => {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "membership_status",
                });
            }
        };
        call_and_apply(runtime, policy, method, request, deadline)
    })
}

pub fn leave_chat(
    runtime: &mut CoreRuntime,
    policy: &RawPolicy,
    chat_id: i64,
    deadline: Instant,
) -> Result<LeaveChatReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    let cache = membership_cache(runtime, chat_id)?;
    let before = membership_observation(runtime, cache)?;
    if before.not_member {
        return Ok(leave_chat_receipt(
            chat_id,
            LeaveChatOutcome::AlreadyNotMember,
            Some(before.sequence),
        ));
    }
    expect_ok(
        call_and_apply(
            runtime,
            policy,
            "leaveChat",
            json!({"@type":"leaveChat","chat_id":chat_id}),
            deadline,
        )?,
        "leaveChat",
    )?;
    loop {
        let observed = membership_observation(runtime, cache)?;
        if observed.sequence > before.sequence && observed.not_member {
            return Ok(leave_chat_receipt(
                chat_id,
                LeaveChatOutcome::VerifiedLeft,
                Some(observed.sequence),
            ));
        }
        match runtime.next_event_until(deadline) {
            Ok(_) => {}
            Err(RuntimeError::DeadlineExceeded) => {
                return Ok(leave_chat_receipt(
                    chat_id,
                    LeaveChatOutcome::Uncertain,
                    None,
                ));
            }
            Err(error) => return Err(ChatWorkflowError::Runtime(error)),
        }
    }
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
    let positions = runtime
        .state()
        .chat_list_positions(list)
        .map_err(ChatWorkflowError::Reducer)?;
    let entries = chat_list_entries(runtime, &positions)?;
    Ok(ChatListSnapshot {
        positions,
        entries,
        load_calls,
        terminal: ChatListTerminal::AllChatsLoaded,
    })
}

fn chat_list_entries(
    runtime: &CoreRuntime,
    positions: &[ChatListPosition],
) -> Result<Vec<ChatListEntry>, ChatWorkflowError> {
    positions
        .iter()
        .map(|position| {
            let chat = runtime.state().chat(position.chat_id).ok_or(
                ChatWorkflowError::PrerequisiteMissing {
                    prerequisite: "chat",
                },
            )?;
            Ok(ChatListEntry {
                chat_id: position.chat_id,
                title: required_string(&chat.value, "title", "chat")?.to_owned(),
                kind: chat_list_entry_kind(&chat.value)?,
                is_pinned: position.is_pinned,
                order: position.order,
            })
        })
        .collect()
}

fn chat_list_entry_kind(chat: &Value) -> Result<ChatListEntryKind, ChatWorkflowError> {
    Ok(match chat["type"]["@type"].as_str() {
        Some("chatTypePrivate") => ChatListEntryKind::Private,
        Some("chatTypeBasicGroup") => ChatListEntryKind::BasicGroup,
        Some("chatTypeSupergroup") if required_bool(&chat["type"], "is_channel", "chat")? => {
            ChatListEntryKind::Channel
        }
        Some("chatTypeSupergroup") => ChatListEntryKind::Supergroup,
        Some("chatTypeSecret") => ChatListEntryKind::Secret,
        _ => ChatListEntryKind::Unknown,
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
    let raw = checked_response(method, raw)?;
    let resolution = resolution_from_raw(Some(runtime), target, method, &raw)?;
    let chat = raw.as_value();
    let full_info_kind = full_info_kind(chat)?;
    let full_info = load_full_info(runtime, policy, chat, open, deadline)?;
    validate_full_info(&full_info, full_info_kind)?;
    Ok(ChatInspection {
        resolution,
        full_info_kind,
        used_open_lease: open,
        complete: true,
    })
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

#[derive(Clone, Copy)]
enum MembershipCache {
    BasicGroup(i64),
    Supergroup(i64),
}

struct MembershipObservation {
    sequence: u64,
    not_member: bool,
}

fn membership_cache(
    runtime: &CoreRuntime,
    chat_id: i64,
) -> Result<MembershipCache, ChatWorkflowError> {
    let chat = runtime
        .state()
        .chat(chat_id)
        .ok_or(ChatWorkflowError::PrerequisiteMissing {
            prerequisite: "chat",
        })?;
    match chat.value["type"]["@type"].as_str() {
        Some("chatTypeBasicGroup") => Ok(MembershipCache::BasicGroup(required_i64(
            &chat.value["type"],
            "basic_group_id",
            "chat",
        )?)),
        Some("chatTypeSupergroup") => Ok(MembershipCache::Supergroup(required_i64(
            &chat.value["type"],
            "supergroup_id",
            "chat",
        )?)),
        _ => Err(ChatWorkflowError::InvalidTarget),
    }
}

fn membership_observation(
    runtime: &CoreRuntime,
    cache: MembershipCache,
) -> Result<MembershipObservation, ChatWorkflowError> {
    let entity = match cache {
        MembershipCache::BasicGroup(group_id) => runtime.state().basic_group(group_id),
        MembershipCache::Supergroup(group_id) => runtime.state().supergroup(group_id),
    }
    .ok_or(ChatWorkflowError::PrerequisiteMissing {
        prerequisite: "chat membership",
    })?;
    let status = required_string(&entity.value["status"], "@type", "chat membership")?;
    Ok(MembershipObservation {
        sequence: entity.sequence.get(),
        not_member: matches!(status, "chatMemberStatusLeft" | "chatMemberStatusBanned"),
    })
}

fn leave_chat_receipt(
    chat_id: i64,
    outcome: LeaveChatOutcome,
    sequence: Option<u64>,
) -> LeaveChatReceipt {
    LeaveChatReceipt {
        chat_id,
        sequence,
        complete: outcome != LeaveChatOutcome::Uncertain,
        outcome,
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

#[cfg(test)]
pub(super) fn resolve_with(
    target: ChatTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<ChatResolution, ChatWorkflowError> {
    let (method, request) = resolution_request(target)?;
    let raw = checked_call(method, request, &mut call)?;
    resolution_from_raw(None, target, method, &raw)
}

pub(super) fn preview_invite_link_with(
    invite_link: &str,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<InvitePreview, ChatWorkflowError> {
    let invite_link = invite_link_value(invite_link)?;
    let method = "checkChatInviteLink";
    let raw = checked_call(
        method,
        json!({"@type":method,"invite_link":invite_link}),
        &mut call,
    )?;
    if raw.as_value()["@type"] != "chatInviteLinkInfo" {
        return Err(ChatWorkflowError::UnexpectedResult { method });
    }
    let chat_id = optional_nonzero_i64(raw.as_value(), "chat_id", method)?;
    let accessible_for = required_i32(raw.as_value(), "accessible_for", method)?;
    Ok(InvitePreview {
        chat_id,
        title: required_string(raw.as_value(), "title", method)?.to_owned(),
        kind: invite_chat_kind(raw.as_value())?,
        visibility: if required_bool(raw.as_value(), "is_public", method)? {
            ChatVisibility::Public
        } else {
            ChatVisibility::NonPublic
        },
        access: if chat_id.is_some() {
            InviteAccess::Accessible
        } else {
            InviteAccess::PreviewOnly
        },
        accessible_for,
        creates_join_request: required_bool(raw.as_value(), "creates_join_request", method)?,
        complete: true,
    })
}

pub(super) fn resolution_request(
    target: ChatTarget<'_>,
) -> Result<(&'static str, Value), ChatWorkflowError> {
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
    })
}

fn resolution_from_raw(
    runtime: Option<&CoreRuntime>,
    target: ChatTarget<'_>,
    method: &'static str,
    raw: &TdObject,
) -> Result<ChatResolution, ChatWorkflowError> {
    if raw.as_value()["@type"] != "chat" {
        return Err(ChatWorkflowError::UnexpectedResult { method });
    }
    Ok(ChatResolution {
        chat: chat_identity(runtime, raw.as_value(), target_public_username(target)?)?,
        freshness: Freshness::ServerSnapshot,
        complete: true,
    })
}

fn chat_identity(
    runtime: Option<&CoreRuntime>,
    chat: &Value,
    known_public_username: Option<&str>,
) -> Result<ChatIdentity, ChatWorkflowError> {
    let mut active_usernames = Vec::new();
    let mut has_location = false;
    let mut has_supergroup_snapshot = false;
    if chat["type"]["@type"] == "chatTypeSupergroup" {
        let supergroup_id = required_i64(&chat["type"], "supergroup_id", "chat")?;
        if let Some(supergroup) =
            runtime.and_then(|runtime| runtime.state().supergroup(supergroup_id))
        {
            has_supergroup_snapshot = true;
            has_location = supergroup.value["has_location"].as_bool().unwrap_or(false);
            if let Some(usernames) = supergroup.value["usernames"]["active_usernames"].as_array() {
                active_usernames.extend(
                    usernames
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_owned),
                );
            }
        }
    }
    if let Some(username) = known_public_username {
        if !active_usernames
            .iter()
            .any(|candidate| candidate == username)
        {
            active_usernames.insert(0, username.to_owned());
        }
    }
    let visibility = if !active_usernames.is_empty() || has_location {
        ChatVisibility::Public
    } else if has_supergroup_snapshot {
        ChatVisibility::NonPublic
    } else {
        ChatVisibility::Unknown
    };
    let canonical_public_url = active_usernames
        .first()
        .map(|username| format!("https://t.me/{username}"));
    Ok(ChatIdentity {
        chat_id: required_i64(chat, "id", "chat")?,
        title: required_string(chat, "title", "chat")?.to_owned(),
        kind: chat_list_entry_kind(chat)?,
        visibility,
        active_usernames,
        canonical_public_url,
    })
}

fn target_public_username(target: ChatTarget<'_>) -> Result<Option<&str>, ChatWorkflowError> {
    match target {
        ChatTarget::Id(_) => Ok(None),
        ChatTarget::PublicUsername(username) => username_value(username).map(Some),
        ChatTarget::PublicLink(link) => public_link_username(link).map(Some),
    }
}

fn invite_chat_kind(value: &Value) -> Result<ChatListEntryKind, ChatWorkflowError> {
    Ok(match value["type"]["@type"].as_str() {
        Some("inviteLinkChatTypeBasicGroup") => ChatListEntryKind::BasicGroup,
        Some("inviteLinkChatTypeSupergroup") => ChatListEntryKind::Supergroup,
        Some("inviteLinkChatTypeChannel") => ChatListEntryKind::Channel,
        _ => {
            return Err(ChatWorkflowError::InvalidResult {
                method: "checkChatInviteLink",
                field: "type.@type",
            });
        }
    })
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

fn invite_link_value(link: &str) -> Result<&str, ChatWorkflowError> {
    let valid_web = [
        "https://t.me/+",
        "http://t.me/+",
        "https://telegram.me/+",
        "http://telegram.me/+",
        "https://t.me/joinchat/",
        "http://t.me/joinchat/",
        "https://telegram.me/joinchat/",
        "http://telegram.me/joinchat/",
    ]
    .iter()
    .any(|prefix| {
        link.strip_prefix(prefix)
            .is_some_and(|value| !value.is_empty())
    });
    let valid_tg = link
        .strip_prefix("tg://join?invite=")
        .is_some_and(|value| !value.is_empty());
    if valid_web || valid_tg {
        Ok(link)
    } else {
        Err(ChatWorkflowError::InvalidTarget)
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

fn full_info_kind(chat: &Value) -> Result<ChatFullInfoKind, ChatWorkflowError> {
    Ok(match chat["type"]["@type"].as_str() {
        Some("chatTypePrivate" | "chatTypeSecret") => ChatFullInfoKind::User,
        Some("chatTypeBasicGroup") => ChatFullInfoKind::BasicGroup,
        Some("chatTypeSupergroup") => ChatFullInfoKind::Supergroup,
        _ => {
            return Err(ChatWorkflowError::InvalidResult {
                method: "chat",
                field: "type.@type",
            });
        }
    })
}

fn validate_full_info(
    full_info: &TdObject,
    kind: ChatFullInfoKind,
) -> Result<(), ChatWorkflowError> {
    let expected = match kind {
        ChatFullInfoKind::User => "userFullInfo",
        ChatFullInfoKind::BasicGroup => "basicGroupFullInfo",
        ChatFullInfoKind::Supergroup => "supergroupFullInfo",
    };
    if full_info.as_value()["@type"] == expected {
        Ok(())
    } else {
        Err(ChatWorkflowError::UnexpectedResult {
            method: "chat full info",
        })
    }
}

pub(super) fn full_info_request(chat: &Value) -> Result<(&'static str, Value), ChatWorkflowError> {
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

pub(super) fn ensure_membership_with(
    target: MembershipTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<MembershipResult, ChatWorkflowError> {
    let target_chat_id = match target {
        MembershipTarget::ChatId(chat_id) => Some(chat_id),
        MembershipTarget::InviteLink(_) => None,
    };
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
    let chat_id = match state {
        MembershipState::Member { chat_id } => Some(chat_id),
        _ => target_chat_id,
    };
    Ok(MembershipResult {
        chat_id,
        state,
        submission_complete: state.submission_complete(),
        membership_complete: state.membership_complete(),
    })
}

pub(super) fn membership_status_with(
    target: MembershipTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<MembershipStatus, ChatWorkflowError> {
    let chat_id = match target {
        MembershipTarget::ChatId(chat_id) if chat_id != 0 => chat_id,
        MembershipTarget::ChatId(_) => return Err(ChatWorkflowError::InvalidTarget),
        MembershipTarget::InviteLink(invite_link) => {
            let invite_link = invite_link_value(invite_link)?;
            let preview = membership_status_call(
                "checkChatInviteLink",
                json!({"@type":"checkChatInviteLink","invite_link":invite_link}),
                &mut call,
            )?;
            if preview.as_value()["@type"] != "chatInviteLinkInfo" {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "checkChatInviteLink",
                });
            }
            let chat_id = required_i64(preview.as_value(), "chat_id", "checkChatInviteLink")?;
            if chat_id == 0 {
                return Ok(MembershipStatus {
                    chat_id: None,
                    state: MembershipStatusState::Unresolved,
                    freshness: Freshness::ServerSnapshot,
                    complete: false,
                });
            }
            chat_id
        }
    };
    let chat = membership_status_call(
        "getChat",
        json!({"@type":"getChat","chat_id":chat_id}),
        &mut call,
    )?;
    if chat.as_value()["@type"] != "chat" {
        return Err(ChatWorkflowError::UnexpectedResult { method: "getChat" });
    }
    let (method, request, expected) = match chat.as_value()["type"]["@type"].as_str() {
        Some("chatTypeBasicGroup") => (
            "getBasicGroup",
            json!({
                "@type":"getBasicGroup",
                "basic_group_id":required_i64(
                    &chat.as_value()["type"],
                    "basic_group_id",
                    "getChat",
                )?
            }),
            "basicGroup",
        ),
        Some("chatTypeSupergroup") => (
            "getSupergroup",
            json!({
                "@type":"getSupergroup",
                "supergroup_id":required_i64(
                    &chat.as_value()["type"],
                    "supergroup_id",
                    "getChat",
                )?
            }),
            "supergroup",
        ),
        _ => return Err(ChatWorkflowError::InvalidTarget),
    };
    let group = membership_status_call(method, request, &mut call)?;
    if group.as_value()["@type"] != expected {
        return Err(ChatWorkflowError::UnexpectedResult { method });
    }
    let state = membership_status_state(&group.as_value()["status"])?;
    Ok(MembershipStatus {
        chat_id: Some(chat_id),
        state,
        freshness: Freshness::ServerSnapshot,
        complete: !matches!(
            state,
            MembershipStatusState::Unresolved | MembershipStatusState::Unknown
        ),
    })
}

fn membership_status_call(
    method: &'static str,
    request: Value,
    call: &mut impl FnMut(Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<TdObject, ChatWorkflowError> {
    checked_response(method, call(request)?)
}

fn membership_status_state(status: &Value) -> Result<MembershipStatusState, ChatWorkflowError> {
    Ok(match required_string(status, "@type", "chat membership")? {
        "chatMemberStatusMember" | "chatMemberStatusAdministrator" => MembershipStatusState::Member,
        "chatMemberStatusCreator" | "chatMemberStatusRestricted"
            if required_bool(status, "is_member", "chat membership")? =>
        {
            MembershipStatusState::Member
        }
        "chatMemberStatusCreator" | "chatMemberStatusRestricted" => {
            MembershipStatusState::NotMember
        }
        "chatMemberStatusLeft" => MembershipStatusState::NotMember,
        "chatMemberStatusBanned" => MembershipStatusState::Banned,
        _ => MembershipStatusState::Unknown,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{CapabilityDisposition, capability};

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
        assert_eq!(member.chat_id, Some(7));
        assert!(member.submission_complete);
        assert!(member.membership_complete);

        let pending =
            ensure_membership_with(MembershipTarget::InviteLink("private-link"), |request| {
                assert_eq!(request["@type"], "joinChatByInviteLink");
                object(json!({"@type":"chatJoinResultRequestSent"}))
            })
            .unwrap();
        assert_eq!(pending.state, MembershipState::RequestPending);
        assert_eq!(pending.chat_id, None);
        assert!(pending.submission_complete);
        assert!(!pending.membership_complete);
        let serialized = serde_json::to_string(&pending).unwrap();
        assert!(!serialized.contains("@type"));
        assert!(!serialized.contains("raw"));

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
        assert!(approval.submission_complete);
        assert!(!approval.membership_complete);
    }

    #[test]
    fn pending_join_can_be_observed_as_member_later_without_rejoining() {
        let mut methods = Vec::new();
        let pending = ensure_membership_with(
            MembershipTarget::InviteLink("https://t.me/+private-link"),
            |request| {
                methods.push(request["@type"].as_str().unwrap().to_owned());
                object(json!({"@type":"chatJoinResultRequestSent"}))
            },
        )
        .unwrap();
        assert_eq!(pending.state, MembershipState::RequestPending);

        let status = membership_status_with(
            MembershipTarget::InviteLink("https://t.me/+private-link"),
            |request| {
                let method = request["@type"].as_str().unwrap().to_owned();
                methods.push(method.clone());
                match method.as_str() {
                    "checkChatInviteLink" => workflow_object(json!({
                        "@type":"chatInviteLinkInfo",
                        "chat_id":7,
                        "accessible_for":0,
                        "type":{"@type":"inviteLinkChatTypeChannel"},
                        "title":"Channel",
                        "creates_join_request":true,
                        "is_public":false
                    })),
                    "getChat" => workflow_object(json!({
                        "@type":"chat",
                        "id":7,
                        "title":"Channel",
                        "type":{
                            "@type":"chatTypeSupergroup",
                            "supergroup_id":8,
                            "is_channel":true
                        }
                    })),
                    "getSupergroup" => workflow_object(json!({
                        "@type":"supergroup",
                        "id":8,
                        "status":{"@type":"chatMemberStatusMember","member_until_date":0}
                    })),
                    _ => unreachable!(),
                }
            },
        )
        .unwrap();

        assert_eq!(status.chat_id, Some(7));
        assert_eq!(status.state, MembershipStatusState::Member);
        assert_eq!(status.freshness, Freshness::ServerSnapshot);
        assert!(status.complete);
        assert_eq!(
            methods,
            [
                "joinChatByInviteLink",
                "checkChatInviteLink",
                "getChat",
                "getSupergroup"
            ]
        );
    }

    #[test]
    fn membership_status_preserves_unresolved_invite_without_guessing() {
        let mut methods = Vec::new();
        let status = membership_status_with(
            MembershipTarget::InviteLink("https://t.me/+private-link"),
            |request| {
                methods.push(request["@type"].as_str().unwrap().to_owned());
                workflow_object(json!({
                    "@type":"chatInviteLinkInfo",
                    "chat_id":0,
                    "accessible_for":0,
                    "type":{"@type":"inviteLinkChatTypeChannel"},
                    "title":"Channel",
                    "creates_join_request":true,
                    "is_public":false
                }))
            },
        )
        .unwrap();

        assert_eq!(status.chat_id, None);
        assert_eq!(status.state, MembershipStatusState::Unresolved);
        assert!(!status.complete);
        assert_eq!(methods, ["checkChatInviteLink"]);
    }

    #[test]
    fn discovery_never_dispatches_membership_or_presence_methods() {
        let mut methods = Vec::new();
        for target in [
            ChatTarget::Id(7),
            ChatTarget::PublicUsername("public_name"),
            ChatTarget::PublicLink("https://t.me/public_name?single"),
        ] {
            resolve_with(target, |request| {
                let method = request["@type"].as_str().unwrap().to_owned();
                methods.push(method.clone());
                object(json!({
                    "@type":"chat",
                    "id":7,
                    "title":"Public chat",
                    "type":{"@type":"chatTypeSupergroup","supergroup_id":8,"is_channel":true}
                }))
            })
            .unwrap();
        }
        let preview = preview_invite_link_with("https://t.me/+invite", |request| {
            methods.push(request["@type"].as_str().unwrap().to_owned());
            object(json!({
                "@type":"chatInviteLinkInfo",
                "chat_id":7,
                "accessible_for":0,
                "type":{"@type":"inviteLinkChatTypeChannel"},
                "title":"Public channel",
                "creates_join_request":false,
                "is_public":true,
                "description":"PRIVATE_DESCRIPTION_CANARY",
                "member_user_ids":[9]
            }))
        })
        .unwrap();

        assert_eq!(
            methods,
            [
                "getChat",
                "searchPublicChat",
                "searchPublicChat",
                "checkChatInviteLink"
            ]
        );
        assert_eq!(preview.visibility, ChatVisibility::Public);
        assert_eq!(preview.access, InviteAccess::Accessible);
        let serialized = serde_json::to_string(&preview).unwrap();
        assert!(!serialized.contains("PRIVATE_DESCRIPTION_CANARY"));
        assert!(!serialized.contains("member_user_ids"));
        assert!(matches!(
            resolution_request(ChatTarget::PublicLink("https://t.me/+invite")),
            Err(ChatWorkflowError::InvalidTarget)
        ));
        assert!(matches!(
            preview_invite_link_with("https://t.me/public_name", |_| unreachable!()),
            Err(ChatWorkflowError::InvalidTarget)
        ));
        assert!(methods.iter().all(|method| !matches!(
            method.as_str(),
            "joinChat" | "joinChatByInviteLink" | "openChat" | "closeChat"
        )));
    }

    #[test]
    fn policy_data_separates_read_resolution_from_membership_mutation() {
        for method in ["searchPublicChat", "getBasicGroup", "getSupergroup"] {
            assert!(matches!(
                capability(method).unwrap().disposition,
                CapabilityDisposition::Reviewed {
                    risk: RiskClass::Read,
                    retry: RetryClass::SafeRead,
                    ..
                }
            ));
        }
        for method in ["joinChat", "joinChatByInviteLink", "leaveChat"] {
            assert!(matches!(
                capability(method).unwrap().disposition,
                CapabilityDisposition::Reviewed {
                    risk: RiskClass::ReversibleMutation,
                    retry: RetryClass::Reconcile,
                    ..
                }
            ));
        }
    }

    #[test]
    fn membership_status_maps_all_current_tdlib_member_states() {
        let cases = [
            (
                json!({"@type":"chatMemberStatusMember"}),
                MembershipStatusState::Member,
            ),
            (
                json!({"@type":"chatMemberStatusAdministrator"}),
                MembershipStatusState::Member,
            ),
            (
                json!({"@type":"chatMemberStatusCreator","is_member":true}),
                MembershipStatusState::Member,
            ),
            (
                json!({"@type":"chatMemberStatusCreator","is_member":false}),
                MembershipStatusState::NotMember,
            ),
            (
                json!({"@type":"chatMemberStatusRestricted","is_member":true}),
                MembershipStatusState::Member,
            ),
            (
                json!({"@type":"chatMemberStatusRestricted","is_member":false}),
                MembershipStatusState::NotMember,
            ),
            (
                json!({"@type":"chatMemberStatusLeft"}),
                MembershipStatusState::NotMember,
            ),
            (
                json!({"@type":"chatMemberStatusBanned"}),
                MembershipStatusState::Banned,
            ),
        ];
        for (status, expected) in cases {
            assert_eq!(membership_status_state(&status).unwrap(), expected);
        }
        assert_eq!(
            membership_status_state(&json!({"@type":"futureMemberStatus"})).unwrap(),
            MembershipStatusState::Unknown
        );
    }
}
