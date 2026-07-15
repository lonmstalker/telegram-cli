use telegram_core::method_capability::{ChatAdministratorRight, ResolvedChatKind};

const CHAT_KINDS: &[ResolvedChatKind] = &[
    ResolvedChatKind::BasicGroup,
    ResolvedChatKind::Supergroup,
    ResolvedChatKind::Channel,
];

const CONTRACTS: &[VideoChatContract] = &[
    VideoChatContract {
        method: "createVideoChat",
        canonical_signature: "createVideoChat chat_id:int53 title:string start_date:int32 is_rtmp_stream:Bool = GroupCallId;",
        source_text: "creates a video chat (a group call bound to a chat); for basic groups, supergroups and channels only; requires can_manage_video_chats administrator right",
        required_access: RequiredAccess::AdministratorRight(
            ChatAdministratorRight::CanManageVideoChats,
        ),
    },
    VideoChatContract {
        method: "getVideoChatRtmpUrl",
        canonical_signature: "getVideoChatRtmpUrl chat_id:int53 = RtmpUrl;",
        source_text: "returns rtmp url for streaming to the video chat of a chat; requires can_manage_video_chats administrator right",
        required_access: RequiredAccess::AdministratorRight(
            ChatAdministratorRight::CanManageVideoChats,
        ),
    },
    VideoChatContract {
        method: "replaceVideoChatRtmpUrl",
        canonical_signature: "replaceVideoChatRtmpUrl chat_id:int53 = RtmpUrl;",
        source_text: "replaces the current rtmp url for streaming to the video chat of a chat; requires owner privileges in the chat",
        required_access: RequiredAccess::Owner,
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static VideoChatContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

#[derive(Clone, Copy)]
pub(super) enum RequiredAccess {
    AdministratorRight(ChatAdministratorRight),
    Owner,
}

pub(super) struct VideoChatContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    required_access: RequiredAccess,
}

impl VideoChatContract {
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
