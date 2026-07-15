const CONTRACT: ChatBoostContract = ChatBoostContract {
    method: "getChatBoosts",
    canonical_signature: "getChatBoosts chat_id:int53 only_gift_codes:Bool offset:string limit:int32 = FoundChatBoosts;",
    source_text: "returns the list of boosts applied to a chat; requires administrator rights in the chat",
};

pub(super) fn reviewed_contract(method: &str) -> Option<&'static ChatBoostContract> {
    (method == CONTRACT.method).then_some(&CONTRACT)
}

pub(super) struct ChatBoostContract {
    method: &'static str,
    canonical_signature: &'static str,
    source_text: &'static str,
}

impl ChatBoostContract {
    pub(super) const fn canonical_signature(&self) -> &'static str {
        self.canonical_signature
    }

    pub(super) const fn source_text(&self) -> &'static str {
        self.source_text
    }
}
