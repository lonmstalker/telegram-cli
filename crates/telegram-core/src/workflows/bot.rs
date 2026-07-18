//! bot workflows.

use super::*;

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

pub(super) fn bot_start_receipt(
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
