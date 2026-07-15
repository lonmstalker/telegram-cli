use telegram_core::method_capability::{ChatAdministratorRight, ResolvedChatKind};

const SUPPORTED_CHAT_KINDS: &[ResolvedChatKind] = &[
    ResolvedChatKind::BasicGroup,
    ResolvedChatKind::Supergroup,
    ResolvedChatKind::Channel,
];

const CONTRACTS: &[ChatInviteLinkContract] = &[
    ChatInviteLinkContract {
        method: "createChatInviteLink",
        canonical_signature: "createChatInviteLink chat_id:int53 name:string expiration_date:int32 member_limit:int32 creates_join_request:Bool = ChatInviteLink;",
        source_text: "creates a new invite link for a chat. available for basic groups, supergroups, and channels. requires administrator privileges and can_invite_users right in the chat",
        required_access: RequiredAccess::AdministratorRight(ChatAdministratorRight::CanInviteUsers),
    },
    ChatInviteLinkContract {
        method: "replacePrimaryChatInviteLink",
        canonical_signature: "replacePrimaryChatInviteLink chat_id:int53 = ChatInviteLink;",
        source_text: "replaces current primary invite link for a chat with a new primary invite link. available for basic groups, supergroups, and channels. requires administrator privileges and can_invite_users right",
        required_access: RequiredAccess::AdministratorRight(ChatAdministratorRight::CanInviteUsers),
    },
    ChatInviteLinkContract {
        method: "getChatInviteLinkCounts",
        canonical_signature: "getChatInviteLinkCounts chat_id:int53 = ChatInviteLinkCounts;",
        source_text: "returns the list of chat administrators with number of their invite links. requires owner privileges in the chat",
        required_access: RequiredAccess::Owner,
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static ChatInviteLinkContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

pub(super) struct ChatInviteLinkContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    required_access: RequiredAccess,
}

impl ChatInviteLinkContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }

    pub(super) const fn supported_chat_kinds(&self) -> &'static [ResolvedChatKind] {
        SUPPORTED_CHAT_KINDS
    }

    pub(super) const fn required_access(&self) -> RequiredAccess {
        self.required_access
    }

    pub(super) const fn regular_user_only(&self) -> bool {
        matches!(self.required_access, RequiredAccess::Owner)
    }
}

#[derive(Clone, Copy)]
pub(super) enum RequiredAccess {
    AdministratorRight(ChatAdministratorRight),
    Owner,
}
