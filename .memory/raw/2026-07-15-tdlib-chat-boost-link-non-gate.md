# TDLib getChatBoostLinkInfo lexical non-gate digest

Дата: 2026-07-15.

## Scope and source

- Task: `W-20260715-017`, P0.5b4.
- Pinned TDLib: `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Exact schema source: `getChatBoostLinkInfo` description says the input can be any internal link of type `internalLinkTypeChatBoost`; it does not state a boost/right/account prerequisite.
- Exact normalized source: `returns information about a link to boost a chat. can be called for any internal link of the type internallinktypechatboost`.
- Pinned source path: `Requests.cpp:5954-5957,774-798` forwards the cleaned URL to `get_dialog_boost_link_info`; `BoostManager.cpp:458-478` and `LinkManager.cpp:4502-4608` parse/resolve the link without a current-account right or boostability check.

## Disposition and proof

- Exact key `getChatBoostLinkInfo\tdescription\tchat_boost_reference` changes from `deferred:unclassified_description` to `not_runtime_gate:chat_boost_vocabulary`.
- The exception matches method, description source, family and one exact normalized tag. A same-family mutation to `only after chatBoost activation` remains deferred and returns `SchemaDrift`.
- Existing boost-level lexical exceptions were refactored to the same unique-tag exact matcher; their semantics did not change.
- The 398-key semantic disposition SHA-256 becomes `9261d9aa49c7bb6dd37a973029a356efb6f44381f59be4ed5a4766ec14b681f7`.
- Supported typed methods remain 35. Terminal non-gates become 3 with SHA-256 `93add10667b68f96b5f8005668163b3627d1ed9eface6d7c06c5b5ab414cbdc0`; terminal complete becomes 38 with SHA-256 `cc79495102cf0d22c42f412154433e46b5ba1c1559d880a724627aba17893115`.
- Exact open set becomes 155 methods with SHA-256 `4ed02dd1adbb3c87c61b4f6fccc009e331670c22fa7ac0c406e782d917ef9c1b`.

## Verification and boundary

- Red test first reproduced `SchemaDrift` for the pinned method; the narrow exact arm made it green.
- Green checkpoint: 52 generator tests, 22 core tests, 74 whole-workspace tests, Clippy `-D warnings`, fmt and diff checks with bounded `jobs=2`; two independent reviews are `Approved`.
- No capability atom, runtime evaluator, service, TDLib DB or network session was added. The result only removes one lexical false positive; 155 runtime-signal methods remain open.
