//! Curated stateful workflows поверх общего TDJSON call.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;
use std::time::{Instant, SystemTime};

use serde_json::{Value, json};

use crate::raw_api::{RawApiError, RawPolicy, td_call, td_call_with_boundary};
use crate::reducer::{ChatList, ChatListPosition, ReducerError};
use crate::registry::TdObject;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PageOptions {
    pub count: usize,
    pub min_date: Option<i32>,
    pub page_limit: i32,
}

pub struct HistoryQuery {
    pub chat_id: i64,
    pub only_local: bool,
    pub page: PageOptions,
}

pub struct ChatSearchQuery<'query> {
    pub chat_id: i64,
    pub query: &'query str,
    pub page: PageOptions,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PageBoundary {
    Count,
    Date,
    Exhausted,
    NoProgress,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MessagePage {
    pub messages: Vec<Value>,
    pub pages: usize,
    pub next_from_message_id: Option<i64>,
    pub boundary: PageBoundary,
    pub complete: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Freshness {
    ServerSnapshot,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MembersQuery {
    pub supergroup_id: i64,
    pub count: usize,
    pub page_limit: i32,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct StatisticsSnapshot {
    pub statistics: Value,
    pub graph_lineage: BTreeMap<String, Vec<String>>,
    pub unresolved_tokens: Vec<String>,
    pub complete: bool,
    pub capability_sequence: u64,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
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

pub fn chat_history(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: HistoryQuery,
    deadline: Instant,
) -> Result<MessagePage, ChatWorkflowError> {
    chat_history_with(query, |request| {
        invoke(runtime, policy, "getChatHistory", request, deadline)
    })
}

pub fn search_chat_messages(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: ChatSearchQuery<'_>,
    deadline: Instant,
) -> Result<MessagePage, ChatWorkflowError> {
    search_chat_messages_with(query, |request| {
        invoke(runtime, policy, "searchChatMessages", request, deadline)
    })
}

pub fn supergroup_members(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    query: MembersQuery,
    deadline: Instant,
) -> Result<MembersSnapshot, ChatWorkflowError> {
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

fn required_string<'value>(
    value: &'value Value,
    field: &'static str,
    method: &'static str,
) -> Result<&'value str, ChatWorkflowError> {
    value[field]
        .as_str()
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
            | Self::InvalidPageOptions => None,
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
}
