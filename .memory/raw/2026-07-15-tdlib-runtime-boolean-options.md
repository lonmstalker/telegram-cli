# TDLib runtime boolean option capability digest

Дата: 2026-07-15.

## Scope and immutable sources

- Task: `W-20260715-020`, P0.5b7.
- Pinned TDLib: `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Schema: `vendor/tdlib/td_api.tl`, SHA-256 `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Reviewed source archive: SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- Reviewed source files: `Requests.cpp` SHA-256 `8c0f906d5116b1aebdea5918d6d2602c7dae757544287db61f8d25033f458a7f`, `OptionManager.cpp` SHA-256 `44a50463b78d1eb859d7c48edf414426f5c839137d1fd47776efecee0fb68f38`, `StoryManager.cpp` SHA-256 `8a8c082441c0f483cd2b9b2f286cd2d0547879baa86e9f0cc0e8c49e6d2e838e`, `GlobalPrivacySettings.cpp` SHA-256 `ff08313e0a318d2ace30718155373ec9e3a69277c4758db4e32a27e33f4f46cf`.
- Changed implementation surfaces: `crates/telegram-core/src/method_capability.rs` and `tools/tdlib-registry-gen/src/capability.rs` with their tests.

## Exact schema and domain boundary

- Exact `OptionValue` vocabulary contains four constructors: `optionValueBoolean`, `optionValueEmpty`, `optionValueInteger` and `optionValueString`. Addition, removal, type drift or namespace drift fails closed.
- Ingress is pinned to method `getOption name:string = OptionValue` and update constructor `updateOption name:string value:OptionValue = Update`.
- Closed `RuntimeBooleanOption` vocabulary contains exactly `can_set_new_chat_privacy_settings`, `can_use_text_entities_in_story_caption` and `can_withdraw_chat_revenue`; ordered string equality is tested explicitly.
- `BooleanOptionEnabled` is account-neutral and has no argument references. Exact reviewed method evidence separately determines account and source constraints.
- Capability policy/canonical format is `6`. Unknown DTO fields and option values are rejected.

## Reviewed partition and formula

- Exact `OptionGate` family contains three methods, SHA-256 `337f9b8f67b5f19afbb8b58f88406cb9ee7a338cffc66af621af7d906bac7638`.
- Complete subset: only `setNewChatPrivacySettings`, SHA-256 `f78d3c7ea100d2cef7c1cc10b2f8168a8b5ff2517b0fb555060bb392bb3f12cb`.
- Deferred subset: `getChatRevenueWithdrawalUrl` and `postStory`, SHA-256 `0100c0c220a0ac85bb1814964c9d3ed0b7dc28bd1b9dd6d70d81cf6b40619caa`.
- Exact complete formula: `boolean_option_enabled(can_set_new_chat_privacy_settings)`; the request handler contains `CHECK_IS_USER()`, so the method descriptor is regular-user-only while the atom remains account-neutral.
- The consumed signal key is `setNewChatPrivacySettings\tdescription\toption_gate`, SHA-256 `1483fb444083358b5439743f1af6830437f5796f9675dad37bf76bf9adbe42b6`.
- Seven signal rows of the two mixed methods remain deferred, SHA-256 `c7b18d2efd06804b7bbd1168195df2e6557d0a942c1b49b5a2a9fb215d4355a8`.

## Semantic source evidence

- `OptionManager` sets `can_set_new_chat_privacy_settings=true` for Premium accounts. For non-Premium accounts it uses `!need_premium_for_new_chat_privacy`; the option is therefore not equivalent to a Premium entitlement.
- `OptionManager` publishes non-internal values through ordered `updateOption` objects and returns typed `OptionValue` objects through `getOption`.
- `postStory.caption` uses `can_use_text_entities_in_story_caption` only for entity handling. The pinned path clears entities when unavailable and differs for bots; it is not a complete method-availability predicate.
- `getChatRevenueWithdrawalUrl` combines the option with owner kind, channel `supergroupFullInfo`, bot `userFullInfo`, revenue eligibility and password/input semantics. Replacing that DNF with a single option atom would be fail-open.
- `newChatPrivacySettings.incoming_paid_message_star_count > 0` separately requires `can_enable_paid_messages`. W020 proves the method-level gate only and does not claim that every settings payload is valid.

## Freshness boundary for future runtime

- W020 adds a static schema/source-bound requirement only; it does not implement an option store or runtime evaluator.
- A future evaluator may return true only for `optionValueBoolean(true)` from the current TDLib session/account/DC generation after a complete ordered initial/update stream.
- Missing, empty, false, wrong-typed, reset, incomplete or gap-affected evidence must evaluate false. Account/session/DC replacement invalidates prior evidence.
- A complete gapless option stream does not need an invented TTL; freshness follows generation identity and ordered update completeness.

## TDD, oracle and review evidence

- Red checkpoints reproduced the missing domain atom and the formerly deferred exact contract before implementation.
- Public generator tests cover omitted requirement, wrong known option, bot policy, exact serialization, unknown fields, source removal/substitution/wording drift, duplicate source, added signal and signature drift.
- Schema negative controls cover every `OptionValue` shape plus getter/update signature and constructor namespace.
- Reviewer P2 found that enum round-trip alone did not pin two exact strings. The final test now asserts ordered equality of all three names; repeat review approved the correction.
- Exact global signal inventory remains 193 methods, 208 sources and 398 keys; key SHA-256 remains `b0b95745adac694757ae7a46dcbb4dce048129379c3aefa62da62f04a2476545`.
- Semantic disposition SHA-256 is `a6a47059f166850f6dfb6a22e867bd69f7999d4b8d371d67f11115b267495ad9`; exactly one row changed from deferred to consumed.
- Supported typed methods: 53, SHA-256 `28c3996aea8c326235460012c8bbadf95aa3ce78d051ab4b0000f6f2a9058dd4`. Terminal complete methods: 56, SHA-256 `0d6710cf033722a06ecb2509644662db04f7dd4f8609effc56126fa208cbd4ba`.
- Remaining open methods: 137, SHA-256 `c05b282773cfd9ecaa1e8ab0c24a0ad08d7589a1fbf05a08901fe355db6c959e`.
- Green checkpoint: 65 generator tests, 25 core tests and 90 whole-workspace tests; Clippy `-D warnings`, fmt and diff checks passed with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`.
- Rust, pinned-evidence and independent oracle reviews are `Approved`; Rust repeat review verified the P2 correction.
- Build footprint is 147 MiB. No service, TDLib DB, network session or background runtime resource was created.

## Boundary

- Two family methods and 137 total runtime-signal methods remain open. The canonical 1010-method capability artifact, runtime evaluator, prerequisite/risk/retry layers, registry/codec/router, singleton daemon and live acceptance are not complete.
- W020 does not prove current Telegram permission, arbitrary settings payload validity or invocation readiness.
