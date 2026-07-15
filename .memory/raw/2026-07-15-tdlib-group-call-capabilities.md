# TDLib GroupCall capability digest

Дата: 2026-07-15.

## Scope and immutable sources

- Task: `W-20260715-018`, P0.5b5.
- Pinned TDLib: `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Schema: `vendor/tdlib/td_api.tl`, SHA-256 `10a00b48d557d00c0daa231a8dad38a9d0c99de78360a1e4b0b7579b28188f31`.
- Reviewed source archive: SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- Reviewed source files: `Requests.cpp` SHA-256 `8c0f906d5116b1aebdea5918d6d2602c7dae757544287db61f8d25033f458a7f`, `GroupCallManager.cpp` SHA-256 `26fadaf0af131b0423e1d31d2551a1805c2996e4256312c2244eee5fc45d9dbb`, `GroupCallMessage.cpp` SHA-256 `f6e23b447ce21bd35e910ca160df80b0bef713ca15734c21971b293d83928442`.
- Changed implementation surfaces: `crates/telegram-core/src/method_capability.rs` and `tools/tdlib-registry-gen/src/capability.rs` with their tests.

## Exact schema and domain boundary

- `groupCall` is pinned as one ordered 32-field constructor; `groupCallMessage` is pinned as one ordered 7-field constructor. Reorder, rename, omission, addition, type drift and extra constructor fail closed.
- Exact ingress is pinned to `getGroupCall group_call_id:int32`, `updateGroupCall`, `updateNewGroupCallMessage` and `updateGroupCallMessagesDeleted`. Moving any update declaration into the methods namespace fails closed.
- Closed domain vocabulary contains three mutually exclusive resolved kinds (`video_chat`, `live_story`, `unbound`), seven reviewed `groupCall` Bool properties and `groupCallMessage.can_be_deleted`.
- `GroupCallIdRef` binds only exact `group_call_id:int32`. `GroupCallMessageSubjectRef::Each` binds exact `message_ids:vector<int32>` and makes universal cardinality part of the atom.
- Two different kinds for one group-call target in one AND-clause are rejected. Group-call atoms are incompatible with bot alternatives.
- Capability policy/canonical format is `4`; unknown DTO fields, enum values, identifier roles and scalar/vector shapes are rejected.

## Reviewed partition and formulas

- Schema-derived family: 14 methods, SHA-256 `19f031588f1a95638917e614017be240ae1bcb7a139d1c9b8e74c0882b79c2e5`.
- Safe typed subset: 12 methods, SHA-256 `a889f96632f1e7e61cb292ec5bb97fc6eaf260ce524f2742636216b2fd7c9570`.
- Deferred argument-dependent subset: `getVideoChatInviteLink` and `toggleGroupCallParticipantIsHandRaised`, SHA-256 `08e959dd01394969d60f64a26448a2baf959b82d1deb7b775aeb32b35b336d3e`.
- The 12 complete contracts consume 38 exact signal keys, SHA-256 `baa12c60379a31fd62a3f030b65ac3e87f0827793c340bb7f63f7bff000f1df5`. Six argument keys remain deferred, SHA-256 `c7d82927a49fb17def723966a8b964c8d4a725fdc755b1c72b97cf59fd5878ef`.
- Single-kind formulas: video-chat managed title/recording, live-story managed paid-message price, live-story delete rights, unbound ownership, and direct `can_send_messages`/`can_toggle_are_messages_allowed` properties.
- `revokeGroupCallInviteLink` is `(video_chat AND can_be_managed) OR (unbound AND is_owned)`.
- `endGroupCall` is `(video_chat AND can_be_managed) OR (live_story AND can_be_managed) OR (unbound AND is_owned)`.
- `deleteGroupCallMessages` is `live_story AND Each(message_ids, can_be_deleted)`.
- Safe/deferred sets are disjoint and their union is asserted equal to the full schema-derived family.

## Semantic corrections and source evidence

- All 14 reviewed request handlers contain `CHECK_IS_USER()` in pinned `Requests.cpp`; complete group-call descriptors are therefore regular-user-only. This is pinned source evidence, not a schema inference.
- `toggleVideoChatMuteNewParticipants` wording `only by administrators` describes the value being configured: who may unmute new participants. It is not a caller prerequisite. The exact key receives `not_runtime_gate:group_call_participant_unmute_policy`; the typed requirement remains `video_chat AND can_toggle_mute_new_participants`.
- The lexical exception matches exact method, description source, family and normalized full text. Same-family wording drift remains fail closed.
- `getVideoChatInviteLink` depends on the `can_self_unmute` Bool argument; `toggleGroupCallParticipantIsHandRaised` depends on participant identity and requested Bool value. Neither receives a false common predicate.
- Pinned manager code also checks lifecycle and invocation state such as `is_active`, `is_joined`, `need_rejoin`, input validity and Star balance. These are future prerequisite/parameter/runtime facts, not silently folded into static capability atoms.

## Freshness boundary for future runtime

- Arbitrary `GroupCallMessage` lookup is unavailable; message evidence arrives through `updateNewGroupCallMessage` and deletion invalidation.
- Pinned `can_delete_group_call_message` computes the snapshot from active/live-story state, sender=self, group-call delete/manage facts, current channel-administrator status and membership of the sender in `created_public_broadcasts`. These group-call/chat/account facts can change without re-emitting old message objects.
- A future evaluator must invalidate or age out cached `can_be_deleted` evidence after any capability-affecting group-call, chat or account change, including `updateGroupCall`, and treat missing, evicted or stale evidence as false. If complete dependency tracking is unavailable, conservative age-out is required.
- Safe invalidation may produce a false negative until new evidence arrives. Returning true from incomplete or stale message state is forbidden.

## TDD, oracle and review evidence

- Red checkpoints reproduced missing domain atoms, contradictory kind acceptance, bot-compatible group-call atoms, absent schema validator, wrong global hashes and the mistaken consumption of `OnlyByAdministrator`.
- Public generator tests prove exact title DNF, regular-user-only account boundary and universal group-call-message serialization. Negative controls cover schema/source drift, duplicate sources, added signals, wrong identifier/type/cardinality and update-to-method namespace drift.
- Exact global signal inventory remains 193 methods, 208 sources and 398 keys. Semantic disposition SHA-256 is `97dd27f0432fe34a6a1c0af4e8cfc2cec955067a735971205341bf99a7c81859`.
- Supported typed methods: 47, SHA-256 `952b7a34cad37987b3a0914e451c79111042a819dcb9df84594d91c410297979`. Terminal complete methods: 50, SHA-256 `f08a00cc6fc7377504637ef3fb84b75b52455cd2397fa1dd669bf2ef41f16175`.
- Remaining open methods: 143, SHA-256 `a6e5b3c9d53a657e7ee3f9f4f5ed4bad7043292418b08849273d406f513b3a12`.
- Green checkpoint: 57 generator tests, 23 core tests, 80 whole-workspace tests, Clippy `-D warnings`, fmt, diff, workspace-boundary, schema/native pin and wiki-journal gates with `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`.
- Three independent final reviews are `Approved`; the adversarial review found the setting-semantics classification bug and verified its exact correction.
- Build footprint is 147 MiB; no service, TDLib DB, network session or background runtime resource was created.

## Boundary

- W018 implements static schema/source-bound requirements, not runtime truth, policy permission or invocation readiness.
- Two mixed methods and 143 total runtime-signal methods remain open. The canonical 1010-method capability artifact, runtime evaluator, prerequisite/risk/retry layers, registry/codec/router, singleton daemon and live acceptance are not complete.
