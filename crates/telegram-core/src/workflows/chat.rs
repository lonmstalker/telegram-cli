//! chat workflows.

use super::*;

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

pub(super) fn resolve_with(
    target: ChatTarget<'_>,
    mut call: impl FnMut(Value) -> Result<TdObject, RawApiError>,
) -> Result<ChatResolution, ChatWorkflowError> {
    let (method, request) = resolution_request(target)?;
    let raw = checked_call(method, request, &mut call)?;
    resolution_from_raw(method, raw)
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
