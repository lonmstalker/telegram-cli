use telegram_core::method_capability::{ChatAdministratorRight, ChatMemberRight, ResolvedChatKind};

const CONTRACTS: &[SupergroupSettingContract] = &[
    SupergroupSettingContract {
        method: "setSupergroupMainProfileTab",
        canonical_signature: "setSupergroupMainProfileTab supergroup_id:int53 main_profile_tab:ProfileTab = Ok;",
        source_text: "changes the main profile tab of the channel; requires can_change_info administrator right",
        chat_kind: ResolvedChatKind::Channel,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanChangeInfo),
        regular_user_only: true,
    },
    SupergroupSettingContract {
        method: "setSupergroupUnrestrictBoostCount",
        canonical_signature: "setSupergroupUnrestrictBoostCount supergroup_id:int53 unrestrict_boost_count:int32 = Ok;",
        source_text: "changes the number of times the supergroup must be boosted by a user to ignore slow mode and chat permission restrictions; requires can_restrict_members administrator right",
        chat_kind: ResolvedChatKind::Supergroup,
        required_right: RequiredRight::Administrator(ChatAdministratorRight::CanRestrictMembers),
        regular_user_only: false,
    },
    SupergroupSettingContract {
        method: "toggleSupergroupIsAllHistoryAvailable",
        canonical_signature: "toggleSupergroupIsAllHistoryAvailable supergroup_id:int53 is_all_history_available:Bool = Ok;",
        source_text: "toggles whether the message history of a supergroup is available to new members; requires can_change_info member right",
        chat_kind: ResolvedChatKind::Supergroup,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: true,
    },
    SupergroupSettingContract {
        method: "toggleSupergroupSignMessages",
        canonical_signature: "toggleSupergroupSignMessages supergroup_id:int53 sign_messages:Bool show_message_sender:Bool = Ok;",
        source_text: "toggles whether sender signature or link to the account is added to sent messages in a channel; requires can_change_info member right",
        chat_kind: ResolvedChatKind::Channel,
        required_right: RequiredRight::Member(ChatMemberRight::CanChangeInfo),
        regular_user_only: true,
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static SupergroupSettingContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

#[derive(Clone, Copy)]
pub(super) enum RequiredRight {
    Administrator(ChatAdministratorRight),
    Member(ChatMemberRight),
}

pub(super) struct SupergroupSettingContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
    chat_kind: ResolvedChatKind,
    required_right: RequiredRight,
    regular_user_only: bool,
}

impl SupergroupSettingContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }

    pub(super) const fn chat_kind(&self) -> ResolvedChatKind {
        self.chat_kind
    }

    pub(super) const fn required_right(&self) -> RequiredRight {
        self.required_right
    }

    pub(super) const fn regular_user_only(&self) -> bool {
        self.regular_user_only
    }
}
