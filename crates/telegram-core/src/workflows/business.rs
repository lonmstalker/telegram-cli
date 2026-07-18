//! business workflows.

use super::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BusinessConnectionSnapshot {
    pub connection_id: String,
    pub is_enabled: bool,
    pub can_reply: bool,
    pub can_read_messages: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BusinessMessageOutcome {
    Sent,
    CapabilityUnavailable,
    CapabilityLost,
    Uncertain,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct BusinessMessageReceipt {
    pub connection_id: String,
    pub chat_id: i64,
    pub message_id: Option<i64>,
    pub outcome: BusinessMessageOutcome,
    pub content_redacted: bool,
    pub complete: bool,
    pub observed_at: SystemTime,
    pub freshness: Freshness,
}

pub fn business_connection(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    connection_id: &str,
    deadline: Instant,
) -> Result<BusinessConnectionSnapshot, ChatWorkflowError> {
    require_resynced(runtime)?;
    business_connection_with(connection_id, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub fn send_business_text(
    runtime: &CoreRuntime,
    policy: &RawPolicy,
    connection_id: &str,
    chat_id: i64,
    text: &str,
    deadline: Instant,
) -> Result<BusinessMessageReceipt, ChatWorkflowError> {
    require_resynced(runtime)?;
    send_business_text_with(connection_id, chat_id, text, |method, request| {
        invoke(runtime, policy, method, request, deadline)
    })
}

pub(super) fn business_connection_with(
    connection_id: &str,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<BusinessConnectionSnapshot, ChatWorkflowError> {
    validate_business_input(connection_id, None, None)?;
    fetch_business_connection(connection_id, &mut call)
}

pub(super) fn send_business_text_with(
    connection_id: &str,
    chat_id: i64,
    text: &str,
    mut call: impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<BusinessMessageReceipt, ChatWorkflowError> {
    validate_business_input(connection_id, Some(chat_id), Some(text))?;
    let before = fetch_business_connection(connection_id, &mut call)?;
    if !before.is_enabled || !before.can_reply {
        return Ok(business_message_receipt(
            connection_id,
            chat_id,
            None,
            BusinessMessageOutcome::CapabilityUnavailable,
        ));
    }
    let request = json!({
        "@type":"sendBusinessMessage",
        "business_connection_id":connection_id,
        "chat_id":chat_id,
        "reply_to":null,
        "disable_notification":false,
        "protect_content":false,
        "effect_id":0,
        "reply_markup":null,
        "input_message_content":{
            "@type":"inputMessageText",
            "text":{"@type":"formattedText","text":text,"entities":[]},
            "link_preview_options":null,
            "clear_draft":false
        }
    });
    match call("sendBusinessMessage", request) {
        Ok(response) => {
            if response.as_value()["@type"] != "businessMessage"
                || response.as_value()["message"]["@type"] != "message"
                || required_i64(
                    &response.as_value()["message"],
                    "chat_id",
                    "sendBusinessMessage",
                )? != chat_id
            {
                return Err(ChatWorkflowError::UnexpectedResult {
                    method: "sendBusinessMessage",
                });
            }
            Ok(business_message_receipt(
                connection_id,
                chat_id,
                Some(required_i64(
                    &response.as_value()["message"],
                    "id",
                    "sendBusinessMessage",
                )?),
                BusinessMessageOutcome::Sent,
            ))
        }
        Err(error) if response_timed_out(&error) => {
            let outcome = fetch_business_connection(connection_id, &mut call).map_or(
                BusinessMessageOutcome::Uncertain,
                |connection| {
                    if connection.is_enabled && connection.can_reply {
                        BusinessMessageOutcome::Uncertain
                    } else {
                        BusinessMessageOutcome::CapabilityLost
                    }
                },
            );
            Ok(business_message_receipt(
                connection_id,
                chat_id,
                None,
                outcome,
            ))
        }
        Err(error) => Err(error),
    }
}

fn fetch_business_connection(
    connection_id: &str,
    call: &mut impl FnMut(&'static str, Value) -> Result<TdObject, ChatWorkflowError>,
) -> Result<BusinessConnectionSnapshot, ChatWorkflowError> {
    let response = call(
        "getBusinessConnection",
        json!({"@type":"getBusinessConnection","connection_id":connection_id}),
    )?;
    let value = response.as_value();
    if value["@type"] != "businessConnection"
        || required_string(value, "id", "getBusinessConnection")? != connection_id
        || value["rights"]["@type"] != "businessBotRights"
    {
        return Err(ChatWorkflowError::UnexpectedResult {
            method: "getBusinessConnection",
        });
    }
    Ok(BusinessConnectionSnapshot {
        connection_id: connection_id.to_owned(),
        is_enabled: required_bool(value, "is_enabled", "getBusinessConnection")?,
        can_reply: required_bool(&value["rights"], "can_reply", "getBusinessConnection")?,
        can_read_messages: required_bool(
            &value["rights"],
            "can_read_messages",
            "getBusinessConnection",
        )?,
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    })
}

fn business_message_receipt(
    connection_id: &str,
    chat_id: i64,
    message_id: Option<i64>,
    outcome: BusinessMessageOutcome,
) -> BusinessMessageReceipt {
    BusinessMessageReceipt {
        connection_id: connection_id.to_owned(),
        chat_id,
        message_id,
        outcome,
        content_redacted: true,
        complete: matches!(
            outcome,
            BusinessMessageOutcome::Sent | BusinessMessageOutcome::CapabilityUnavailable
        ),
        observed_at: SystemTime::now(),
        freshness: Freshness::ServerSnapshot,
    }
}

fn validate_business_input(
    connection_id: &str,
    chat_id: Option<i64>,
    text: Option<&str>,
) -> Result<(), ChatWorkflowError> {
    if connection_id.is_empty()
        || connection_id.len() > 256
        || connection_id.chars().any(char::is_control)
        || chat_id.is_some_and(|chat_id| chat_id <= 0)
        || text.is_some_and(|text| text.is_empty() || text.chars().count() > 4096)
    {
        return Err(ChatWorkflowError::InvalidBusinessInput);
    }
    Ok(())
}
