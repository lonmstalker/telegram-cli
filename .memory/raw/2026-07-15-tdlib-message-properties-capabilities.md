# TDLib MessageProperties capability digest

Дата: 2026-07-15.

## Scope

- Task: `W-20260715-016`, P0.5b3.
- Pinned TDLib: `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Schema: `vendor/tdlib/td_api.tl`, SHA-256 `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Reviewed source archive: SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- Changed implementation surfaces: `crates/telegram-core/src/method_capability.rs` and `tools/tdlib-registry-gen/src/capability.rs` with their tests.

## Exact schema and domain boundary

- `messageProperties` is pinned as one ordered constructor with exactly 39 `Bool` fields and result `MessageProperties`; field reorder, rename, omission, addition, type drift and duplicate constructor/method fail closed.
- The action vocabulary contains exactly 36 `can_*` fields. `has_protected_content_by_current_user`, `has_protected_content_by_other_user` and `need_show_statistics` are intentionally not runtime action capabilities.
- `getMessageProperties chat_id:int53 message_id:int53 = MessageProperties;` is the only accepted accessor signature.
- `MessageSubjectRef::One` binds `chat_id:int53 + message_id:int53`; `Each` binds `supergroup_id:int53 + message_ids:vector<int53>`. Cardinality is part of the typed atom, not free-form text.
- Capability policy/canonical format is `3`; unknown capability values and unknown DTO fields are rejected.

## Reviewed method partition

- Schema-derived `MessagePropertiesFact` set: 33 methods, SHA-256 `5f04f36c0e2862498474a4a1651d2f5131f3adb80d6cef766b33c9c2bf11e8fc`.
- Safe typed subset: 29 methods, SHA-256 `45d98c8243fd32ac9b9fc0234b73a2946e1eb62813cbac30cabaa043c7e53cba`.
- Exact 30 `(method, source, chat argument, message argument, cardinality, property)` bindings: SHA-256 `fee0c5dc03c67084e46b0d20a8158dcecbf38c58676a149fe9140d8469aeb50b`.
- The 29 methods consume 59 exact signal keys, SHA-256 `c4f91a61456297edd4a9a2fe206d3d37410cc2eb67f6f73e9661985949175ed2`.
- Four mixed/invocation-dependent methods remain deferred: `deleteMessages`, `forwardMessages`, `getMessageLink`, `reportChat`; method-set SHA-256 `a7755c8b6787c2ea596a45f6a17a4af970a735382721ee949aec54beb8602317`, 11-key SHA-256 `c15898d88f92f2bde554308208952d52fd0add6d33bb0f72635b6959ec7beffc`.
- Safe/deferred sets are disjoint and their union is asserted equal to the full schema-derived 33-method set.

## Non-trivial semantics

- `addOffer` is exact DNF: `can_add_offer OR can_edit_suggested_post_info`. In pinned `MessagesManager.cpp`, add eligibility requires `suggested_post == nullptr`, edit eligibility requires `suggested_post != nullptr`, and `add_offer` selects the branch from the target message state; caller input does not choose the branch.
- `reportSupergroupSpam` is one conjunction: `ChatKind(supergroup_id, supergroup) AND ChatAdministrator(supergroup_id) AND Each(message_ids, can_report_supergroup_spam)`. Pinned `ChatManager.cpp` checks every message ID and returns on the first failed status; there is no successful partial subset contract.
- Exact normalized source text is name-first and unique-tag checked. For a reviewed method, source removal, wording drift with the same field, substitution/addition of another valid field, duplicate source tag, wrong identifier space or wrong scalar/vector type returns `SchemaDrift`.

## TDD and verification evidence

- Red checkpoints reproduced missing `MessageCapability` domain types, absent reviewed dispositions, unordered constructor validation and duplicate-source fail-open behavior before their implementations.
- Public generator E2E rejects an omitted message predicate, accepts the exact policy atom and serializes the canonical `One` subject shape.
- Exact global runtime-signal inventory remains 193 methods, 208 sources and 398 keys. Semantic disposition SHA-256 is `f3deda49ce421e04ebe35c1745635299c20f46ceaff1caa85791483ceb28165d`.
- Terminal typed methods: 35, SHA-256 `fcd0437a123ad047e2e8edddbe51ffc63795d49c47e9ae1bb4c986745155a22e`; terminal complete including two lexical non-gates: 37, SHA-256 `43535e24511a033a1e62c27c1e52dcd007d66186fd57254754c08ba5e31ee8a0`.
- Remaining open methods: 156, SHA-256 `e3ce3e31e2f024513cb1f04e5d4f116b05e31eca6483302532da1395197b8e54`.
- Green checkpoint: 52 generator tests, 22 core tests, 74 whole-workspace tests, Clippy `-D warnings`, fmt, diff, workspace-boundary and TDLib pin gates with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`.
- Three independent final reviews are `Approved`; the adversarial review first found and then verified closure of the derived-partition and public-generate proof gaps.
- Build footprint remained about 146 MiB; no service, TDLib DB, network session or background runtime resource was created.

## Boundary

- This task implements static schema-bound requirements only. It does not query live message properties, evaluate runtime truth, grant policy permission or implement request execution.
- 156 recognized runtime-signal methods remain open; the 1010-method capability artifact, risk/prerequisite/retry layers, registry/codec/router, singleton daemon and live acceptance are not complete.
