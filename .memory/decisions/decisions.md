# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-024 | Chat event log требует explicit account, kind и administrator evidence

- Context: public schema даёт kind/admin signals, но account boundary и exact handler semantics нужно подтвердить pinned C++ path, а не выводить из общей lexical family.
- Decision: `getChatEventLog` regular-user-only и имеет ровно две static DNF alternatives: `supergroup AND ChatAdministrator` и `channel AND ChatAdministrator`. Contract pin-ит full signature и exact normalized method source; argument-level additions не поглощаются молча.
- Runtime rule: administrator status — prerequisite, не current proof. Будущий singleton daemon должен давать account/target/session-bound evidence и invalidates его при role, membership, kind, generation и update-gap changes; missing/stale evidence fail closed.
- Evidence: [chat event log capability digest](../raw/2026-07-15-tdlib-chat-event-log-capability.md), pinned schema/Requests/DialogEventLog, red/green exact tests, oracle transitions и independent code-review `APPROVED`.
- Alternatives: допустить bots, свести kinds к broad group, положиться только на description или добавить generic event-log DSL отклонены как неточные либо лишние.
- Consequences: supported typed set 66 → 67, terminal set 69 → 70, open set 124 → 123; capability format остаётся `8`. Runtime evaluator/freshness и zero-open full corpus остаются separate gates.
- Supersedes: none; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md) и [D-20260715-012](archive/2026-07-15--2026-07-15-011.md).

## [2026-07-15] accepted correction | D-20260715-025 | unpinChatMessage не закрывается description-only DNF

- Context: historical contract перенёс public wording в five-branch DNF без deeper-handler review. Pinned source добавляет account-conditioned basic-group guard, monoforum exception, secret rejection, concrete message-state и write-access checks.
- Decision: real `unpinChatMessage` возвращён в deferred `SchemaDrift`. Generic conditional-right tests используют explicit `requireSyntheticConditionalPinRight`, отсутствующий в pinned schema и real-method oracles.
- Runtime rule: future contract должен выразить account-conditioned implication, direct-messages subtype, concrete message eligibility и current write-peer access. Missing/stale facts fail closed.
- Evidence: [unpinChatMessage correction digest](../raw/2026-07-15-tdlib-unpin-chat-message-overclaim-correction.md), pinned Requests/MessagesManager/DialogManager, red/green regression, oracle transitions и independent code-review `APPROVED`.
- Alternatives: оставить broad DNF, запретить bots целиком, игнорировать monoforum или добавить free-form predicate отклонены как fail-open, overly narrow либо unbounded.
- Consequences: supported typed set 67 → 66, terminal set 70 → 69, open set 123 → 124; format остаётся `8`. Это correction ложного coverage, а не regression Telegram support.
- Supersedes: historical `unpinChatMessage` five-branch contract; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](archive/2026-07-15--2026-07-15-009.md) и [D-20260715-012](archive/2026-07-15--2026-07-15-011.md).

## [2026-07-15] accepted | D-20260715-026 | Invite-link counts используют closed owner access

- Context: count method делит invite-link domain с create/replace, но требует owner, а не `can_invite_users`; отдельные bool/right tables могли бы разойтись.
- Decision: local closed `RequiredAccess` становится единым source для account scope, DNF и consumed keys. `getChatInviteLinkCounts` — regular user и three-kind `ChatOwner`; create/replace сохраняют administrator-right contract и bot support.
- Runtime rule: owner/write/active-chat evidence обязано быть current account/target/session-bound; missing, stale или gap-affected facts fail closed.
- Evidence: [invite-link counts digest](../raw/2026-07-15-tdlib-chat-invite-link-counts-capability.md), pinned schema/Requests/DialogInviteLinkManager, TDD/drift controls, exhaustive owner partition и independent `APPROVED`.
- Alternatives: новый параллельный module, generic role DSL, duplicate regular-user bool и hardcoded named right отклонены как duplication или unbounded abstraction.
- Consequences: supported 66 → 67, terminal 69 → 70, open 124 → 123; format остаётся `8`. Runtime evaluator/full corpus остаются separate gates.
- Supersedes: none; extends [D-20260715-010](archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md) и [D-20260715-019](decisions.md).

## [2026-07-15] archive link map | D-20260715-026 | Rotated decision targets

- Immutable [D-20260715-018 shard](archive/2026-07-15--2026-07-15-020.md) ссылается на historical active paths; canonical targets: [D-20260715-010](archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md), [D-20260715-017](archive/2026-07-15--2026-07-15-019.md).
- Active historical D019 reference на D018 разрешается в [canonical shard 020](archive/2026-07-15--2026-07-15-020.md). Decision status не изменён.

## [2026-07-15] accepted | D-20260715-027 | Video chat RTMP access не требует group-call state

- Context: `getVideoChatRtmpUrl` относится к video-chat streaming, но pinned request принимает `chat_id`, а не `group_call_id`; перенос существующей `GroupCallProperty` модели создал бы ложную зависимость от active call state.
- Decision: отдельный semantic contract pin-ит regular-user scope и три static alternatives: `basic_group|supergroup|channel AND ChatAdministratorRight(can_manage_video_chats)`. Private/secret branches отсутствуют; source/signature/additional-signal drift fail closed.
- Runtime rule: dialog read access, chat kind и administrator right должны быть current account/target/session-bound. Missing, stale или gap-affected evidence fail closed; static prerequisite не обещает server success.
- Evidence: [video chat RTMP access digest](../raw/2026-07-15-tdlib-video-chat-rtmp-access-capability.md), pinned schema/Requests/GroupCallManager, TDD/drift controls, exact corpus oracles и independent `APPROVED`.
- Alternatives: использовать group-call kind/property, добавить owner/active-call/RTMP-state predicate, положиться только на description или расширить generic DSL отклонены как неверный aggregate, invented gate либо лишняя абстракция.
- Consequences: supported 67 → 68, terminal 70 → 71, open 123 → 122; format остаётся `8`. Runtime evaluator/full corpus остаются separate gates.
- Supersedes: none; extends [D-20260715-010](archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md), [D-20260715-014](archive/2026-07-15--2026-07-15-014.md) и [D-20260715-017](archive/2026-07-15--2026-07-15-019.md).

## [2026-07-15] archive link map | D-20260715-027 | Rotated D019

- Canonical [D-20260715-019](archive/2026-07-15--2026-07-15-021.md); historical active-path targets остаются immutable, status не изменён.

## [2026-07-15] archive link correction | D-20260715-027 | Shard 021 targets

- [Shard 021](archive/2026-07-15--2026-07-15-021.md): canonical [D-20260715-010](archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md), [D-20260715-018](archive/2026-07-15--2026-07-15-020.md) и D019 в самом shard 021.

## [2026-07-15] archive link correction | D-20260715-027 | Shard 022 targets

- [Shard 022](archive/2026-07-15--2026-07-15-022.md): canonical [D-20260715-010](archive/2026-07-15--2026-07-15-009.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md), [D-20260715-019](archive/2026-07-15--2026-07-15-021.md) и D020 в самом shard 022.

## [2026-07-15] split link correction | D-20260715-020 | Base и correction

- [Historical base](archive/2026-07-15--2026-07-15-022.md) исправлена [accepted correction](decisions.md); current claims используют correction.

## [2026-07-15] accepted | D-20260715-028 | RTMP revoke требует owner поверх shared precheck

- Context: revoke использует общий handler с `can_manage_video_chats`, но exact public contract требует owner.
- Decision: `RequiredAccess::Owner` задаёт regular-user и three-kind `ChatOwner` DNF; owner имплицирует manage right, поэтому второй atom не дублируется. Active-call/RTMP-state predicates не изобретаются.
- Runtime rule: owner/kind/read-access evidence current и session-bound; stale/gap fail closed, static pass не обещает server success.
- Evidence: [replacement digest](../raw/2026-07-15-tdlib-video-chat-rtmp-replacement-capability.md), pinned sources, exact tests/oracles, independent `APPROVED`.
- Consequences: supported 68→69, terminal 71→72, open 122→121; format `8`.
- Extends: [D-20260715-027](decisions.md); full corpus/evaluator остаются open.

## [2026-07-15] accepted | D-20260715-029 | Video-chat creation переиспользует typed administrator right

- Context: `createVideoChat` добавляет title/start-date/RTMP-mode values, но pinned dispatcher/handler подтверждают только regular-user, dialog read и `can_manage_video_chats` gates.
- Decision: `capability/video_chats.rs` pin-ит signature/source и три `basic_group|supergroup|channel AND ChatAdministratorRight(can_manage_video_chats)` alternatives. Values остаются RPC/server validation и не расширяют capability grammar.
- Runtime rule: kind/right/read evidence должно быть current account/target/session-bound; missing, stale или gap-affected evidence fail closed.
- Evidence: [creation digest](../raw/2026-07-15-tdlib-video-chat-creation-capability.md), exact TDD/drift controls, corpus oracles и independent `APPROVED`.
- Consequences: supported 69→70, terminal 72→73, open 121→120; format `8`. Runtime evaluator/full corpus остаются open.
- Extends: [D-20260715-027](decisions.md); executable code не содержит planning/task IDs.

## [2026-07-15] archive link map | D-20260715-029 | Rotated D020 correction and D021

- [Shard 023](archive/2026-07-15--2026-07-15-023.md) содержит current accepted correction D020 и accepted D021.
- D020 resolve: [historical base](archive/2026-07-15--2026-07-15-022.md) + [accepted correction](archive/2026-07-15--2026-07-15-023.md). Historical active-path links immutable.
- Shard-023 dependencies: canonical [D010](archive/2026-07-15--2026-07-15-009.md), [D012](archive/2026-07-15--2026-07-15-011.md), [D017](archive/2026-07-15--2026-07-15-019.md), D020 base shard 022 и correction/D021 shard 023.

## [2026-07-15] accepted correction | D-20260715-030 | Message deletion needs account and subtype guards

- Context: historical `deleteChatMessagesBySender` DNF copied public supergroup/right wording but pinned dispatcher and deeper handler also reject bots and monoforum.
- Decision: `message_moderation.rs` requires `RegularUser + Supergroup + is_direct_messages_group=false + can_delete_messages`; exact signature/source are pinned. Broad generic row is removed.
- Runtime rule: write/right/target/sender evidence must be current account/target/session-bound; missing, stale or gap-affected facts fail closed.
- Evidence: [correction digest](../raw/2026-07-15-tdlib-delete-chat-messages-by-sender-correction.md), two red controls, exact drift/account/DNF tests and independent `APPROVED`.
- Consequences: counts stay 70/73/120 and format `8`; false-positive capability path is removed without speculative predicates.
- Supersedes: historical broad contract documented in W014/D011; extends closed subtype rule [D-20260715-023](decisions.md).

## [2026-07-15] accepted | D-20260715-031 | Recent-reaction moderation has no invented subtype gate

- Context: public schema requires basic-group/supergroup kind and `can_delete_messages`; pinned dispatcher/deeper/query path adds write/sender availability but no account or subtype restriction.
- Decision: `deleteAllRecentMessageReactionsFromSender` supports regular user and bot with two exact kind/right alternatives. `required_supergroup_flags` remains empty; regular-only policy is rejected as hidden narrowing.
- Runtime rule: write/right/sender evidence must be current account/target/session-bound; missing, stale or gap-affected facts fail closed.
- Evidence: [capability digest](../raw/2026-07-15-tdlib-delete-recent-reactions-by-sender-capability.md), exact TDD/drift/account tests, corpus oracles and independent `APPROVED`.
- Consequences: supported 70→71, terminal 73→74, open 120→119; format stays `8`. Server acceptance/runtime evaluator remain open.
- Extends: [D-20260715-030](decisions.md); executable code contains no planning/task IDs.

## [2026-07-15] archive link map | D-20260715-031 | Rotated D022

- [Decision shard 024](archive/2026-07-15--2026-07-15-024.md) contains accepted correction D022.
- Canonical dependencies for immutable links: [D010](archive/2026-07-15--2026-07-15-009.md), [D011](archive/2026-07-15--2026-07-15-010.md), [D012](archive/2026-07-15--2026-07-15-011.md), [D015](archive/2026-07-15--2026-07-15-016.md), [D017](archive/2026-07-15--2026-07-15-019.md) and D022 in shard 024.

## [2026-07-15] accepted | D-20260715-032 | Channel gift notifications require posting rights

- Context: public schema names a channel and `can_post_messages`; pinned dispatcher rejects bots and the handler requires broadcast-channel status plus current posting right.
- Decision: `toggleChatGiftNotifications` supports exactly `RegularUser + Channel + can_post_messages`. `are_enabled` is a request value, not a capability predicate.
- Runtime rule: read/kind/right evidence must be current account/target/session-bound; missing, stale or gap-affected facts fail closed.
- Evidence: [capability digest](../raw/2026-07-15-tdlib-chat-gift-notification-capability.md), exact TDD/drift/account tests, corpus oracles and independent `APPROVED`.
- Consequences: supported 71→72, terminal 74→75, open 119→118; format stays `8`. Server acceptance/runtime evaluator remain open.
- Extends: [D-20260715-031](decisions.md); executable code contains no planning/task IDs.

## [2026-07-15] archive link map | D-20260715-032 | Rotated D023

- [Decision shard 025](archive/2026-07-15--2026-07-15-025.md) contains accepted D023; current D030 historical link resolves there.
- Canonical dependencies in the immutable shard: D007 shard 006, D009 shard 008, D012 shard 011, D015 shard 016, D016 shard 018 and D022 shard 024.

## [2026-07-15] archive link correction | D-20260715-023 | Exact rotated dependencies

- Immutable shard 025 historical active-path links resolve [D010](archive/2026-07-15--2026-07-15-009.md), [D020 base](archive/2026-07-15--2026-07-15-022.md) plus [accepted correction](archive/2026-07-15--2026-07-15-023.md), [D021](archive/2026-07-15--2026-07-15-023.md) and [D022](archive/2026-07-15--2026-07-15-024.md).

## [2026-07-15] accepted | D-20260715-033 | Chat-boost list has no proved chat-kind gate

- Context: public schema requires administrator rights in a generic chat; pinned dispatcher rejects bots, while handler/query add read access and request-value validation but no dialog-type branch.
- Decision: `getChatBoosts` supports exactly `RegularUser + ChatAdministrator(chat_id)`. No `ChatKind` is inferred from neighbouring boost methods or server expectations.
- Runtime rule: read/administrator evidence must be current account/target/session-bound; missing, stale or gap-affected facts fail closed. Offset/limit validity and server acceptance are invocation boundaries.
- Evidence: [capability digest](../raw/2026-07-15-tdlib-chat-boost-list-capability.md), exact TDD/drift/account tests, corpus oracles and independent `APPROVED`.
- Consequences: supported 72→73, terminal 75→76, open 118→117; format stays `8`. Runtime evaluator/full corpus remain open.
- Extends: [D-20260715-032](decisions.md); executable code contains no planning/task IDs.
