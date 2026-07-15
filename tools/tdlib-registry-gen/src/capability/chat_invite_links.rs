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
    },
    ChatInviteLinkContract {
        method: "replacePrimaryChatInviteLink",
        canonical_signature: "replacePrimaryChatInviteLink chat_id:int53 = ChatInviteLink;",
        source_text: "replaces current primary invite link for a chat with a new primary invite link. available for basic groups, supergroups, and channels. requires administrator privileges and can_invite_users right",
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static ChatInviteLinkContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

pub(super) struct ChatInviteLinkContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
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

    pub(super) const fn required_right(&self) -> ChatAdministratorRight {
        ChatAdministratorRight::CanInviteUsers
    }
}
