#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct WorkflowCatalogEntry {
    pub(super) name: &'static str,
    pub(super) input_example: &'static str,
    pub(super) journaled: bool,
}

impl WorkflowCatalogEntry {
    const fn new(name: &'static str, input_example: &'static str, journaled: bool) -> Self {
        Self {
            name,
            input_example,
            journaled,
        }
    }
}

// Неидемпотентные side effects journaled для reconciliation после interrupted dispatch.
// Plan/apply остаются вне журнала: exact one-shot approval hash уже ограничивает их dispatch.
pub(super) const WORKFLOWS: &[WorkflowCatalogEntry] = &[
    WorkflowCatalogEntry::new(
        "user_profile",
        r#"{"target":{"kind":"self"},"include_full_info":true}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "update_profile_name",
        r#"{"first_name":"Name","last_name":""}"#,
        false,
    ),
    WorkflowCatalogEntry::new("plan_chat_title", r#"{"chat_id":0,"title":"Title"}"#, false),
    WorkflowCatalogEntry::new(
        "apply_chat_title",
        r#"{"chat_id":0,"title":"Title"}"#,
        false,
    ),
    WorkflowCatalogEntry::new("resolve_chat", r#"{"kind":"id","chat_id":0}"#, false),
    WorkflowCatalogEntry::new(
        "preview_invite_link",
        r#"{"url":"https://t.me/+INVITE_TOKEN"}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "ensure_membership",
        r#"{"kind":"chat_id","chat_id":0}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "membership_status",
        r#"{"kind":"chat_id","chat_id":0}"#,
        false,
    ),
    WorkflowCatalogEntry::new("leave_chat", r#"{"chat_id":0}"#, true),
    WorkflowCatalogEntry::new(
        "load_chat_list",
        r#"{"list":{"kind":"main"},"limit":100}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "inspect_chat",
        r#"{"target":{"kind":"id","chat_id":0},"open":false}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "forum_topics",
        r#"{"chat_id":0,"query":"","count":100,"page_limit":100}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "set_forum_topic_closed",
        r#"{"chat_id":0,"topic_id":0,"is_closed":true}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "chat_history",
        r#"{"chat_id":0,"only_local":false,"mark_read":false,"page":{"count":100,"min_date":null,"page_limit":100}}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "search_chat_messages",
        r#"{"chat_id":0,"query":"","mark_read":false,"page":{"count":100,"min_date":null,"page_limit":100}}"#,
        false,
    ),
    WorkflowCatalogEntry::new("send_text_message", r#"{"chat_id":0,"text":"hello"}"#, true),
    WorkflowCatalogEntry::new(
        "supergroup_members",
        r#"{"supergroup_id":0,"count":100,"page_limit":100}"#,
        false,
    ),
    WorkflowCatalogEntry::new("chat_statistics", r#"{"chat_id":0,"is_dark":false}"#, false),
    WorkflowCatalogEntry::new(
        "resource_statistics",
        r#"{"only_current_network":true}"#,
        false,
    ),
    WorkflowCatalogEntry::new("proxy_status", "{}", false),
    WorkflowCatalogEntry::new(
        "set_proxy_enabled",
        r#"{"action":"enable","proxy_id":1}"#,
        false,
    ),
    WorkflowCatalogEntry::new("resync_after_gap", "{}", false),
    WorkflowCatalogEntry::new(
        "download_file",
        r#"{"file_id":0,"priority":1,"offset":0,"limit":0}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "cancel_download",
        r#"{"file_id":0,"only_if_pending":false}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "upload_sticker_file",
        r#"{"user_id":0,"format":"webp","source":{"kind":"id","id":0}}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "plan_custom_emoji_set",
        r#"{"action":"create","user_id":1,"title":"Disposable","name":"codex_disposable","format":"webp","sticker_file_id":1,"emojis":"🧪","needs_repainting":false}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "apply_custom_emoji_set",
        r#"{"action":"create","user_id":1,"title":"Disposable","name":"codex_disposable","format":"webp","sticker_file_id":1,"emojis":"🧪","needs_repainting":false}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "plan_story_mutation",
        r#"{"action":"post_photo","chat_id":1,"photo_file_id":1,"caption":"","privacy":{"kind":"selected_users","user_ids":[1]},"active_period":86400,"is_posted_to_chat_page":false,"protect_content":true}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "apply_story_mutation",
        r#"{"action":"post_photo","chat_id":1,"photo_file_id":1,"caption":"","privacy":{"kind":"selected_users","user_ids":[1]},"active_period":86400,"is_posted_to_chat_page":false,"protect_content":true}"#,
        true,
    ),
    WorkflowCatalogEntry::new("inspect_group_call", r#"{"group_call_id":1}"#, false),
    WorkflowCatalogEntry::new("leave_group_call", r#"{"group_call_id":1}"#, false),
    WorkflowCatalogEntry::new(
        "notification_settings",
        r#"{"scope":"private_chats"}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "set_notification_settings",
        r#"{"scope":"private_chats","patch":{"mute_for":60}}"#,
        false,
    ),
    WorkflowCatalogEntry::new("active_sessions", "{}", false),
    WorkflowCatalogEntry::new("plan_terminate_session", r#"{"session_id":1}"#, false),
    WorkflowCatalogEntry::new("apply_terminate_session", r#"{"session_id":1}"#, true),
    WorkflowCatalogEntry::new(
        "business_connection",
        r#"{"connection_id":"connection-id"}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "send_business_text",
        r#"{"connection_id":"connection-id","chat_id":1,"text":"hello"}"#,
        true,
    ),
    WorkflowCatalogEntry::new("star_balance", "{}", false),
    WorkflowCatalogEntry::new(
        "plan_star_invoice_payment",
        r#"{"invoice_name":"invoice-name"}"#,
        false,
    ),
    WorkflowCatalogEntry::new(
        "apply_star_invoice_payment",
        r#"{"invoice_name":"invoice-name"}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "start_bot",
        r#"{"bot_user_id":0,"chat_id":0,"parameter":""}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "start_bot_and_wait_reply",
        r#"{"bot_user_id":0,"chat_id":0,"parameter":""}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "click_bot_callback",
        r#"{"chat_id":0,"message_id":0,"row":0,"column":0}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "open_web_app",
        r#"{"chat_id":0,"bot_user_id":0,"button_url":"https://example.invalid","application_name":"main","mode":"compact"}"#,
        true,
    ),
    WorkflowCatalogEntry::new(
        "prepare_web_app_handoff",
        r#"{"chat_id":0,"bot_user_id":0,"button_url":"https://example.invalid","application_name":"main","mode":"compact"}"#,
        true,
    ),
    WorkflowCatalogEntry::new("close_web_app_handoff", r#"{"launch_id":0}"#, false),
];

pub(super) fn workflow(name: &str) -> Option<WorkflowCatalogEntry> {
    WORKFLOWS.iter().copied().find(|entry| entry.name == name)
}

pub(super) fn is_journaled_workflow(name: &str) -> bool {
    workflow(name).is_some_and(|entry| entry.journaled)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    #[test]
    fn every_catalog_workflow_has_one_explicit_journal_classification() {
        let names = WORKFLOWS
            .iter()
            .map(|entry| entry.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), WORKFLOWS.len());
        for entry in WORKFLOWS {
            assert_eq!(workflow(entry.name), Some(*entry));
            assert_eq!(is_journaled_workflow(entry.name), entry.journaled);
        }
        assert!(is_journaled_workflow("ensure_membership"));
        assert!(!is_journaled_workflow("membership_status"));
        assert!(!is_journaled_workflow("not_a_workflow"));
        assert!(workflow("not_a_workflow").is_none());
    }
}
