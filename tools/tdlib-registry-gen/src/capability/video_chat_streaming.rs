use telegram_core::method_capability::{ChatAdministratorRight, ResolvedChatKind};

const CHAT_KINDS: &[ResolvedChatKind] = &[
    ResolvedChatKind::BasicGroup,
    ResolvedChatKind::Supergroup,
    ResolvedChatKind::Channel,
];

const CONTRACTS: &[VideoChatStreamingContract] = &[
    VideoChatStreamingContract {
        method: "getVideoChatRtmpUrl",
        canonical_signature: "getVideoChatRtmpUrl chat_id:int53 = RtmpUrl;",
        source_text: "returns rtmp url for streaming to the video chat of a chat; requires can_manage_video_chats administrator right",
        required_access: RequiredAccess::AdministratorRight(
            ChatAdministratorRight::CanManageVideoChats,
        ),
    },
    VideoChatStreamingContract {
        method: "replaceVideoChatRtmpUrl",
        canonical_signature: "replaceVideoChatRtmpUrl chat_id:int53 = RtmpUrl;",
        source_text: "replaces the current rtmp url for streaming to the video chat of a chat; requires owner privileges in the chat",
        required_access: RequiredAccess::Owner,
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static VideoChatStreamingContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

#[derive(Clone, Copy)]
pub(super) enum RequiredAccess {
    AdministratorRight(ChatAdministratorRight),
    Owner,
}

pub(super) struct VideoChatStreamingContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    required_access: RequiredAccess,
}

impl VideoChatStreamingContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }

    pub(super) const fn supported_chat_kinds(&self) -> &'static [ResolvedChatKind] {
        CHAT_KINDS
    }

    pub(super) const fn required_access(&self) -> RequiredAccess {
        self.required_access
    }
}
