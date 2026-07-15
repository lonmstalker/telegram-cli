use telegram_core::method_capability::ResolvedChatKind;

const SUPPORTED_CHAT_KINDS: &[ResolvedChatKind] =
    &[ResolvedChatKind::Supergroup, ResolvedChatKind::Channel];

const CONTRACT: ChatEventLogContract = ChatEventLogContract {
    method: "getChatEventLog",
    canonical_signature: "getChatEventLog chat_id:int53 query:string from_event_id:int64 limit:int32 filters:chatEventLogFilters user_ids:vector<int53> = ChatEvents;",
    source_text: "returns a list of service actions taken by chat members and administrators in the last 48 hours. available only for supergroups and channels. requires administrator rights. returns results in reverse chronological order (i.e., in order of decreasing event_id)",
};

pub(super) fn reviewed_contract(method: &str) -> Option<&'static ChatEventLogContract> {
    (method == CONTRACT.method).then_some(&CONTRACT)
}

pub(super) struct ChatEventLogContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
}

impl ChatEventLogContract {
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
