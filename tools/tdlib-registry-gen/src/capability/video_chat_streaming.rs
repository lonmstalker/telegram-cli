use telegram_core::method_capability::{ChatAdministratorRight, ResolvedChatKind};

const CHAT_KINDS: &[ResolvedChatKind] = &[
    ResolvedChatKind::BasicGroup,
    ResolvedChatKind::Supergroup,
    ResolvedChatKind::Channel,
];

const RTMP_ACCESS: VideoChatStreamingContract = VideoChatStreamingContract {
    method: "getVideoChatRtmpUrl",
    canonical_signature: "getVideoChatRtmpUrl chat_id:int53 = RtmpUrl;",
    source_text: "returns rtmp url for streaming to the video chat of a chat; requires can_manage_video_chats administrator right",
    required_right: ChatAdministratorRight::CanManageVideoChats,
};

pub(super) fn reviewed_contract(method: &str) -> Option<&'static VideoChatStreamingContract> {
    (method == RTMP_ACCESS.method).then_some(&RTMP_ACCESS)
}

pub(super) struct VideoChatStreamingContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    required_right: ChatAdministratorRight,
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

    pub(super) const fn required_right(&self) -> ChatAdministratorRight {
        self.required_right
    }
}
