# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted correction | D-20260715-022 | Membership contract учитывает pinned account и supergroup subtype gates

- Context: существовавший `addChatMember` contract выводил три broad chat-kind ветки только из публичного description. Pinned handler дополнительно требует regular user и запрещает direct-messages group, которые current DNF не выражает.
- Decision: метод остаётся deferred, пока grammar и runtime evidence не представляют `AccountKind::RegularUser` на method axis и `supergroup.is_direct_messages_group == false` для chat target. Broad `ResolvedChatKind::Supergroup` не заменяет subtype fact; deeper handler review обязателен до complete membership mutation contract.
- Evidence: [addChatMember correction digest](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md), pinned exact source, red regression, updated exact oracles и independent Rust `APPROVED`.
- Alternatives: оставить member-right DNF, запретить все supergroups, считать schema description исчерпывающим или добавить свободный generic flag отклонены как fail-open, чрезмерно узкие либо не schema-bound.
- Consequences: `addChatMember` возвращён в `SchemaDrift`; supported set 66 → 65, terminal 69 → 68, open set 124 → 125. Future completion требует closed subtype vocabulary и conservative runtime freshness.
- Archive link map после ротации: [D-20260715-007 canonical entry](archive/2026-07-15--2026-07-15-006.md) и [D-20260715-015](archive/2026-07-15--2026-07-15-016.md).
- Supersedes: неполный `addChatMember` contract, реализованный до `W-20260715-026`; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md), [D-20260715-011](archive/2026-07-15--2026-07-15-010.md), [D-20260715-012](decisions.md) и [D-20260715-017](decisions.md).

## [2026-07-15] accepted | D-20260715-023 | Supergroup subtype является closed schema-bound Boolean fact

- Context: `toggleSupergroupJoinToSendMessages` имеет доказанные account/right gates, но broad `ResolvedChatKind::Supergroup` не исключает broadcast group/gigagroup/direct-messages group. Оба `addChatMember*` handlers дополнительно ветвятся по self/cardinality input.
- Decision: в domain model добавлен closed `SupergroupFlag` только для `is_broadcast_group` и `is_direct_messages_group`. Condition связывает flag/value с typed target; exact ordered `supergroup` constructor и `updateSupergroup` pin-ятся generator boundary. Единственный complete method contract живёт в semantic settings module и не ссылается на planning IDs; оба invite methods остаются deferred до typed cardinality/current-user predicates.
- Runtime rule: static condition является prerequisite, не current truth. Будущий singleton daemon принимает subtype только из complete current-session `updateSupergroup`/startup state, привязанного к account/DC generation; missing, stale, gap-affected или mismatched target evidence fail closed.
- Evidence: [supergroup subtype capability digest](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md), pinned schema/Requests/DialogParticipantManager/ChatManager, red/green domain/pinned/public tests и exact disposition oracles.
- Alternatives: свободный string flag, отдельный ordinary-supergroup kind, broad supergroup branch, запрет всех refined targets или inference из method description отклонены как не schema-bound, fail-open либо чрезмерно узкие.
- Consequences: capability format становится `8`; supported set 65 → 66, terminal set 68 → 69, open set 125 → 124. Runtime evaluator/freshness и zero-open full corpus остаются отдельными gates.
- Archive link map после ротации: [D-20260715-007](archive/2026-07-15--2026-07-15-006.md), [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md), [D-20260715-015](archive/2026-07-15--2026-07-15-016.md) и [D-20260715-016](archive/2026-07-15--2026-07-15-018.md).
- Supersedes: none; сохраняет singular fail-closed correction [D-20260715-022](decisions.md) и extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md), [D-20260715-012](archive/2026-07-15--2026-07-15-011.md), [D-20260715-020](decisions.md) и [D-20260715-021](decisions.md).

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
