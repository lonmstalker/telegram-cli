use telegram_core::method_capability::{
    ChatAdministratorRight, ChatMemberRight, ResolvedChatKind, SupergroupFlag,
};

const NO_FLAGS: &[(SupergroupFlag, bool)] = &[];
const ORDINARY_SUPERGROUP_FLAGS: &[(SupergroupFlag, bool)] = &[
    (SupergroupFlag::IsBroadcastGroup, false),
    (SupergroupFlag::IsDirectMessagesGroup, false),
];

const GROUPS_AND_CHANNELS: &[ResolvedChatKind] = &[
    ResolvedChatKind::BasicGroup,
    ResolvedChatKind::Supergroup,
    ResolvedChatKind::Channel,
];
const GROUPS: &[ResolvedChatKind] = &[ResolvedChatKind::BasicGroup, ResolvedChatKind::Supergroup];
const SUPERGROUP: &[ResolvedChatKind] = &[ResolvedChatKind::Supergroup];
const CHANNEL: &[ResolvedChatKind] = &[ResolvedChatKind::Channel];

const CONTRACTS: &[ChatSettingContract] = &[
    ChatSettingContract {
        method: "setChatDescription",
        canonical_signature: "setChatDescription chat_id:int53 description:string = Ok;",
        source_text: "changes information about a chat. available for basic groups, supergroups, and channels. requires can_change_info member right",
        supported_chat_kinds: GROUPS_AND_CHANNELS,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: false,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "setChatPermissions",
        canonical_signature: "setChatPermissions chat_id:int53 permissions:chatPermissions = Ok;",
        source_text: "changes the chat members permissions. supported only for basic groups and supergroups. requires can_restrict_members administrator right",
        supported_chat_kinds: GROUPS,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: false,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "setChatSlowModeDelay",
        canonical_signature: "setChatSlowModeDelay chat_id:int53 slow_mode_delay:int32 = Ok;",
        source_text: "changes the slow mode delay of a chat. available only for supergroups; requires can_restrict_members administrator right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: true,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "toggleChatGiftNotifications",
        canonical_signature: "toggleChatGiftNotifications chat_id:int53 are_enabled:Bool = Ok;",
        source_text: "toggles whether notifications for new gifts received by a channel chat are sent to the current user; requires can_post_messages administrator right in the chat",
        supported_chat_kinds: CHANNEL,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanPostMessages),
        regular_user_only: true,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "setSupergroupMainProfileTab",
        canonical_signature: "setSupergroupMainProfileTab supergroup_id:int53 main_profile_tab:ProfileTab = Ok;",
        source_text: "changes the main profile tab of the channel; requires can_change_info administrator right",
        supported_chat_kinds: CHANNEL,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanChangeInfo),
        regular_user_only: true,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "setSupergroupUnrestrictBoostCount",
        canonical_signature: "setSupergroupUnrestrictBoostCount supergroup_id:int53 unrestrict_boost_count:int32 = Ok;",
        source_text: "changes the number of times the supergroup must be boosted by a user to ignore slow mode and chat permission restrictions; requires can_restrict_members administrator right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: false,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "toggleSupergroupIsAllHistoryAvailable",
        canonical_signature: "toggleSupergroupIsAllHistoryAvailable supergroup_id:int53 is_all_history_available:Bool = Ok;",
        source_text: "toggles whether the message history of a supergroup is available to new members; requires can_change_info member right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: true,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "toggleSupergroupSignMessages",
        canonical_signature: "toggleSupergroupSignMessages supergroup_id:int53 sign_messages:Bool show_message_sender:Bool = Ok;",
        source_text: "toggles whether sender signature or link to the account is added to sent messages in a channel; requires can_change_info member right",
        supported_chat_kinds: CHANNEL,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: true,
        required_supergroup_flags: NO_FLAGS,
        target_source_text: None,
    },
    ChatSettingContract {
        method: "toggleSupergroupJoinToSendMessages",
        canonical_signature: "toggleSupergroupJoinToSendMessages supergroup_id:int53 join_to_send_messages:Bool = Ok;",
        source_text: "toggles whether joining is mandatory to send messages to a discussion supergroup; requires can_restrict_members administrator right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: true,
        required_supergroup_flags: ORDINARY_SUPERGROUP_FLAGS,
        target_source_text: Some("identifier of the supergroup that isn't a broadcast group"),
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static ChatSettingContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

#[derive(Clone, Copy)]
pub(super) enum RequiredRight {
    Administrator(ChatAdministratorRight),
    Member(ChatMemberRight),
}

pub(super) struct ChatSettingContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    supported_chat_kinds: &'static [ResolvedChatKind],
    required_right: RequiredRight,
    regular_user_only: bool,
    required_supergroup_flags: &'static [(SupergroupFlag, bool)],
    target_source_text: Option<&'static str>,
}

impl ChatSettingContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }

    pub(super) const fn supported_chat_kinds(&self) -> &'static [ResolvedChatKind] {
        self.supported_chat_kinds
    }

    pub(super) const fn required_right(&self) -> RequiredRight {
        self.required_right
    }

    pub(super) const fn regular_user_only(&self) -> bool {
        self.regular_user_only
    }

    pub(super) const fn required_supergroup_flags(&self) -> &'static [(SupergroupFlag, bool)] {
        self.required_supergroup_flags
    }

    pub(super) const fn target_source_text(&self) -> Option<&'static str> {
        self.target_source_text
    }
}
