# TDLib SupergroupFullInfo capability digest

Дата: 2026-07-15.

## Scope and immutable sources

- Task: `W-20260715-019`, P0.5b6.
- Pinned TDLib: `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Schema: `vendor/tdlib/td_api.tl`, SHA-256 `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Reviewed source archive: SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- Reviewed source files: `Requests.cpp` SHA-256 `8c0f906d5116b1aebdea5918d6d2602c7dae757544287db61f8d25033f458a7f`, `ChatManager.cpp` SHA-256 `a910974cb9681ac96d5c253692295eb798b8f56622e2643a6fb937a975853b84`, `DialogManager.cpp` SHA-256 `17c65d6af565a62f9e2b8facb8b6ef5bea8b333d514c4c5673d44d65f75c7c5a`.
- Changed implementation surfaces: `crates/telegram-core/src/method_capability.rs` and `tools/tdlib-registry-gen/src/capability.rs` with their tests.

## Exact schema and domain boundary

- `supergroupFullInfo` is pinned as one ordered 42-field constructor. Reorder, rename, omission, addition, type drift and extra constructor fail closed.
- Exact ingress is pinned to `getSupergroupFullInfo supergroup_id:int53` and `updateSupergroupFullInfo`; moving the update into the methods namespace fails closed.
- Closed method-documentation vocabulary contains eight Bool properties: `can_enable_paid_messages`, `can_get_members`, `can_get_revenue_statistics`, `can_get_star_revenue_statistics`, `can_get_statistics`, `can_hide_members`, `can_set_location`, `can_toggle_aggressive_anti_spam`.
- `SupergroupFullInfoProperty` binds a semantic `chat_id` or `supergroup_id` target with exact `int53` type. The atom is account-neutral; method account restrictions are separate source evidence.
- Capability policy/canonical format is `5`. Unknown DTO fields, property values, identifier roles and target types are rejected.

## Reviewed partition and formulas

- Schema-derived family: 12 methods, SHA-256 `009753dda10c34a5efd8a8b210f0c2f7addcb51a66d1a9cbcfae1d89f0b85e77`.
- Safe typed subset: five methods, SHA-256 `e2f70c9e185de98cd0e9116f9e63713fbb5dfe50b5dd13ca237eb86e315b4910`.
- Deferred subset: seven methods, SHA-256 `bf00644532ef19d5546ee88dc805b3c1746867941f7758d613b872e53e63ff47`.
- Complete contracts:
  - `getChatStatistics`: `full_info(chat_id, can_get_statistics)`;
  - `setChatLocation`: `full_info(chat_id, can_set_location)`;
  - `setChatPaidMessageStarCount`: `administrator_right(chat_id, can_restrict_members) AND full_info(chat_id, can_enable_paid_messages)`;
  - `toggleSupergroupHasAggressiveAntiSpamEnabled`: `full_info(supergroup_id, can_toggle_aggressive_anti_spam)`;
  - `toggleSupergroupHasHiddenMembers`: `full_info(supergroup_id, can_hide_members)`.
- Deferred methods are `getChatRevenueStatistics`, `getChatRevenueTransactions`, `getChatRevenueWithdrawalUrl`, `getStarRevenueStatistics`, `getStarTransactions`, `getSupergroupMembers` and `setChatDirectMessagesGroup`. Their owner/filter/value/password and adjacent runtime semantics are not replaced with a false common predicate.
- The five complete contracts consume 12 exact signal keys, SHA-256 `1d23853fea29ffe0578cbe963615eb3d5a9fb3e39ce1cbf03753bf42ca1f63fd`. Eighteen keys remain deferred, SHA-256 `f49347a937191c446fed02419c401e22b8cafc545abda87d0f1b5fe516de95db`.

## Semantic corrections and source evidence

- All five complete request handlers contain `CHECK_IS_USER()` in pinned `Requests.cpp`; their descriptors are therefore regular-user-only. The property atom itself remains usable in bot-compatible methods.
- Ten of the 12 family handlers are user-only. `getSupergroupMembers` and `getStarTransactions` have broader account paths and remain deferred.
- The scanner's `OnlyIfAdministrator` matches two cross-token false positives: `toggleSupergroupHasHiddenMembers` and `getSupergroupMembers`. In both descriptions, `only if` belongs to a `supergroupFullInfo.can_*` property while `administrator` occurs elsewhere.
- Those two exact keys use `not_runtime_gate:supergroup_full_info_cross_token_wording`, SHA-256 `2e071a4014d657bd19c8ac1ee1f2d954eac565fa2214166519d373f92ed948a5`. The real `administrator privileges may be required for some filters` signal of `getSupergroupMembers` stays deferred.
- Non-gate exceptions match exact method, description source, family and normalized full text. Same-family wording drift fails closed.

## Freshness boundary for future runtime

- W019 adds static schema/source-bound predicates only. It does not implement a `supergroupFullInfo` store, `chat_id` resolution, refresh scheduling, invalidation or a runtime evaluator.
- Pinned `ChatManager` uses `CHANNEL_FULL_EXPIRE_TIME = 60`; expired full info may be returned while a background reload is initiated. A future evaluator must therefore distinguish fresh, stale and missing evidence.
- `updateSupergroupFullInfo` replaces the available snapshot, while capability-affecting chat/account changes may alter effective facts. Missing, evicted or stale evidence must evaluate false; if dependency-complete invalidation is unavailable, use conservative age-out.
- Safe false negatives until refreshed evidence arrives are allowed. Returning true from incomplete or stale state is forbidden.

## TDD, oracle and review evidence

- Red checkpoints reproduced the missing property atom, missing schema validator and five deferred contracts before implementation.
- Public generator tests prove exact serialization, semantic target kind and regular-user account boundary. Negative controls cover constructor/getter/update shape, source/property drift, duplicate sources, added signals, wrong target name/type, unknown DTO fields and lexical non-gate drift.
- Exact global signal inventory remains 193 methods, 208 sources and 398 keys; key SHA-256 remains `b0b95745adac694757ae7a46dcbb4dce048129379c3aefa62da62f04a2476545`.
- Semantic disposition SHA-256 is `f3f2c8c344d4082ac918f4b4a279f3d863db51760dcf1a5074711faef5e25a58`. An independent scanner reproduced all 398 rows; an earlier projected `5d3daf...` value was rejected as non-reproducible.
- Supported typed methods: 52, SHA-256 `da693e7ee0f44569abeedc82e0c83e0442893b7b70d67687a40c0a48e3062494`. Terminal complete methods: 55, SHA-256 `95da2d6299f9ca3d9e1b43553f084996bc3106b668ef1385dfc3af20fe3a979f`.
- Remaining open methods: 138, SHA-256 `a2028d7acb1055b4c5fc5a0fda69cf4a8c09200feea2fd3d386596e24fc9aa67`.
- Green checkpoint: 61 generator tests, 24 core tests and 85 whole-workspace tests; Clippy `-D warnings`, fmt and diff checks passed with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`.
- Rust, pinned-evidence and independent oracle reviews are `Approved`. Evidence review corrected one misleading freshness completion claim before approval.
- Build footprint is 147 MiB. No service, TDLib DB, network session or background runtime resource was created.

## Boundary

- Seven family methods and 138 total runtime-signal methods remain open. The canonical 1010-method capability artifact, runtime evaluator, prerequisite/risk/retry layers, registry/codec/router, singleton daemon and live acceptance are not complete.
- W019 does not prove current Telegram permission, policy authorization or invocation readiness.
