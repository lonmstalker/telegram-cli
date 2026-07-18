//! message workflows.

use super::*;

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

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TextMessageReceipt {
    pub old_message_id: Option<i64>,
    pub outcome: BotStartOutcome,
    pub source: Option<TerminalSource>,
    pub complete: bool,
    pub observed_at: SystemTime,
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

pub(super) fn redact_message_content(protected: bool, page: &mut MessagePage) {
    if !protected {
        return;
    }
    for message in &mut page.messages {
        let content_type = message["content"]["@type"].clone();
        message["content"] = json!({"@type":content_type,"redacted":true});
    }
    page.content_redacted = true;
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

pub(super) fn chat_history_with(
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

pub(super) fn search_chat_messages_with(
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

pub(super) fn message_page(
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
