use telegram_core::method_capability::ResolvedChatKind;

const SUPPORTED_CHAT_KINDS: &[ResolvedChatKind] =
    &[ResolvedChatKind::Supergroup, ResolvedChatKind::Channel];

const CONTRACTS: &[SupergroupUsernameContract] = &[
    SupergroupUsernameContract {
        method: "disableAllSupergroupUsernames",
        canonical_signature: "disableAllSupergroupUsernames supergroup_id:int53 = Ok;",
        source_text: "disables all active non-editable usernames of a supergroup or channel, requires owner privileges in the supergroup or channel",
    },
    SupergroupUsernameContract {
        method: "reorderSupergroupActiveUsernames",
        canonical_signature: "reorderSupergroupActiveUsernames supergroup_id:int53 usernames:vector<string> = Ok;",
        source_text: "changes order of active usernames of a supergroup or channel, requires owner privileges in the supergroup or channel",
    },
    SupergroupUsernameContract {
        method: "setSupergroupUsername",
        canonical_signature: "setSupergroupUsername supergroup_id:int53 username:string = Ok;",
        source_text: "changes the editable username of a supergroup or channel, requires owner privileges in the supergroup or channel",
    },
    SupergroupUsernameContract {
        method: "toggleSupergroupUsernameIsActive",
        canonical_signature: "toggleSupergroupUsernameIsActive supergroup_id:int53 username:string is_active:Bool = Ok;",
        source_text: "changes active state for a username of a supergroup or channel, requires owner privileges in the supergroup or channel. the editable username can't be disabled. may return an error with a message \"usernames_active_too_much\" if the maximum number of active usernames has been reached",
    },
];

pub(super) fn reviewed_contract(method: &str) -> Option<&'static SupergroupUsernameContract> {
    CONTRACTS.iter().find(|contract| contract.method == method)
}

pub(super) struct SupergroupUsernameContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
}

impl SupergroupUsernameContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }

    pub(super) const fn supported_chat_kinds(&self) -> &'static [ResolvedChatKind] {
        SUPPORTED_CHAT_KINDS
    }
}
