use telegram_core::method_capability::{ChatAdministratorRight, ChatMemberRight, ResolvedChatKind};

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
    },
    ChatSettingContract {
        method: "setChatPermissions",
        canonical_signature: "setChatPermissions chat_id:int53 permissions:chatPermissions = Ok;",
        source_text: "changes the chat members permissions. supported only for basic groups and supergroups. requires can_restrict_members administrator right",
        supported_chat_kinds: GROUPS,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: false,
    },
    ChatSettingContract {
        method: "setChatSlowModeDelay",
        canonical_signature: "setChatSlowModeDelay chat_id:int53 slow_mode_delay:int32 = Ok;",
        source_text: "changes the slow mode delay of a chat. available only for supergroups; requires can_restrict_members administrator right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: true,
    },
    ChatSettingContract {
        method: "setSupergroupMainProfileTab",
        canonical_signature: "setSupergroupMainProfileTab supergroup_id:int53 main_profile_tab:ProfileTab = Ok;",
        source_text: "changes the main profile tab of the channel; requires can_change_info administrator right",
        supported_chat_kinds: CHANNEL,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanChangeInfo),
        regular_user_only: true,
    },
    ChatSettingContract {
        method: "setSupergroupUnrestrictBoostCount",
        canonical_signature: "setSupergroupUnrestrictBoostCount supergroup_id:int53 unrestrict_boost_count:int32 = Ok;",
        source_text: "changes the number of times the supergroup must be boosted by a user to ignore slow mode and chat permission restrictions; requires can_restrict_members administrator right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: false,
    },
    ChatSettingContract {
        method: "toggleSupergroupIsAllHistoryAvailable",
        canonical_signature: "toggleSupergroupIsAllHistoryAvailable supergroup_id:int53 is_all_history_available:Bool = Ok;",
        source_text: "toggles whether the message history of a supergroup is available to new members; requires can_change_info member right",
        supported_chat_kinds: SUPERGROUP,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: true,
    },
    ChatSettingContract {
        method: "toggleSupergroupSignMessages",
        canonical_signature: "toggleSupergroupSignMessages supergroup_id:int53 sign_messages:Bool show_message_sender:Bool = Ok;",
        source_text: "toggles whether sender signature or link to the account is added to sent messages in a channel; requires can_change_info member right",
        supported_chat_kinds: CHANNEL,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: true,
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
}
