pub(super) fn is_journaled_workflow(name: &str) -> bool {
    matches!(
        name,
        "ensure_membership"
            | "leave_chat"
            | "send_text_message"
            | "upload_sticker_file"
            | "apply_custom_emoji_set"
            | "apply_story_mutation"
            | "apply_terminate_session"
            | "send_business_text"
            | "apply_star_invoice_payment"
            | "start_bot"
            | "start_bot_and_wait_reply"
            | "click_bot_callback"
            | "open_web_app"
            | "prepare_web_app_handoff"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_are_not_journaled_but_membership_mutation_is() {
        assert!(!is_journaled_workflow("membership_status"));
        assert!(is_journaled_workflow("ensure_membership"));
    }
}
