use telegram_core::method_capability::{ChatAdministratorRight, ResolvedChatKind, SupergroupFlag};

const DELETE_BY_SENDER_KINDS: &[ResolvedChatKind] = &[ResolvedChatKind::Supergroup];
const DELETE_BY_SENDER_FLAGS: &[(SupergroupFlag, bool)] =
    &[(SupergroupFlag::IsDirectMessagesGroup, false)];
const DELETE_RECENT_REACTIONS_KINDS: &[ResolvedChatKind] =
    &[ResolvedChatKind::BasicGroup, ResolvedChatKind::Supergroup];

const CONTRACTS: &[MessageModerationContract] = &[
    MessageModerationContract {
        method: "deleteChatMessagesBySender",
        canonical_signature: "deleteChatMessagesBySender chat_id:int53 sender_id:MessageSender = Ok;",
        source_text: "deletes all messages sent by the specified message sender in a chat. supported only for supergroups; requires can_delete_messages administrator right",
        regular_user_only: true,
        supported_chat_kinds: DELETE_BY_SENDER_KINDS,
        required_supergroup_flags: DELETE_BY_SENDER_FLAGS,
        required_right: ChatAdministratorRight::CanDeleteMessages,
    },
    MessageModerationContract {
        method: "deleteAllRecentMessageReactionsFromSender",
        canonical_signature: "deleteAllRecentMessageReactionsFromSender chat_id:int53 sender_id:MessageSender = Ok;",
        source_text: "deletes all recent reactions added by the specified sender in a chat. supported only for basic groups and supergroups; requires can_delete_messages administrator right",
        regular_user_only: false,
        supported_chat_kinds: DELETE_RECENT_REACTIONS_KINDS,
        required_supergroup_flags: &[],
        required_right: ChatAdministratorRight::CanDeleteMessages,
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static MessageModerationContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

pub(super) struct MessageModerationContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    regular_user_only: bool,
    supported_chat_kinds: &'static [ResolvedChatKind],
    required_supergroup_flags: &'static [(SupergroupFlag, bool)],
    required_right: ChatAdministratorRight,
}

impl MessageModerationContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }

    pub(super) const fn regular_user_only(&self) -> bool {
        self.regular_user_only
    }

    pub(super) const fn supported_chat_kinds(&self) -> &'static [ResolvedChatKind] {
        self.supported_chat_kinds
    }

    pub(super) const fn required_supergroup_flags(&self) -> &'static [(SupergroupFlag, bool)] {
        self.required_supergroup_flags
    }

    pub(super) const fn required_right(&self) -> ChatAdministratorRight {
        self.required_right
    }
}
