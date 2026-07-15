# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-018 | Username management требует exact chat kind и current owner evidence

- Context: wording `requires owner privileges in the supergroup or channel` задаёт два допустимых chat kinds и более сильное право, чем administrator. Generic consumption owner phrase без exact method/source binding могло бы ошибочно закрыть mixed owner/filter/value methods.
- Decision: четыре exact username-management methods живут в semantic module `capability/supergroup_usernames.rs`. Их signature и normalized description pin-ятся name-first; prerequisite — DNF `ChatKind(supergroup) AND ChatOwner` либо `ChatKind(channel) AND ChatOwner`. Methods regular-user-only; остальные owner-signal methods остаются в explicit complete/deferred partition.
- Runtime rule: static `ChatOwner` является требованием, не доказательством. Будущий evaluator принимает только current, target-bound, account-bound membership/status evidence; missing, stale, incomplete или mismatched evidence fail closed.
- Evidence: [supergroup username owner digest](../raw/2026-07-15-tdlib-supergroup-username-owner-capabilities.md), public generator test, exhaustive pinned family test и source/signature/additional-signal negative controls.
- Alternatives: free-form owner predicate, administrator-as-owner, one branch без explicit kind и generic phrase matcher отклонены как семантически неверные или fail-open.
- Consequences: supported typed set растёт 53 → 57, terminal complete 56 → 60, exact open set уменьшается 137 → 133. Capability format остаётся `7`; runtime evaluator и freshness store не реализованы.
- Supersedes: none; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md) и [D-20260715-017](decisions.md).

## [2026-07-15] accepted | D-20260715-019 | Invite-link creation требует administrator can-invite right в explicit chat kind

- Context: create/replace primary invite link имеют безусловный administrator+`can_invite_users` contract для трёх chat kinds, тогда как соседние методы выбирают admin или owner prerequisite по creator/invite-link input.
- Decision: только exact `createChatInviteLink` и `replacePrimaryChatInviteLink` входят в semantic module `capability/chat_invite_links.rs`. Их DNF содержит отдельные basic-group/supergroup/channel branches с `ChatAdministratorRight(CanInviteUsers)`; regular user и bot допустимы. Девять own/other-link methods остаются deferred.
- Runtime rule: static requirement не является current truth. Evaluator использует account/target-bound current status, учитывает active basic group и invalidates evidence при membership/right/account changes; unknown/stale fail closed.
- Evidence: [chat invite-link creation digest](../raw/2026-07-15-tdlib-chat-invite-link-creation-capabilities.md), pinned schema/source archive, public generator test, exhaustive triple-signal partition и drift controls.
- Alternatives: member permission вместо administrator right, generic invite-link matcher, consumption own/other branches и implicit catch-all chat kind отклонены как семантически неверные или fail-open.
- Consequences: supported typed set растёт 57 → 59, terminal complete 60 → 62, exact open set уменьшается 133 → 131. Capability format остаётся `7`; invocation predicates/runtime evaluator не реализованы.
- Supersedes: none; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md) и [D-20260715-018](decisions.md).

## [2026-07-15] accepted | D-20260715-020 | Supergroup setting сохраняет exact kind, role-right и account boundary

- Context: пять methods имеют безусловный setting right, но различают channel/supergroup, administrator/member wording и account availability. Три соседних methods дополнительно зависят от boost или guard-bot input.
- Decision: exact five-method subset живёт в `capability/supergroup_settings.rs`; contract pin-ит method/signature/source, `ResolvedChatKind`, `Administrator/Member` right и regular-only flag. Используются существующие runtime atoms; generic free-form predicate не добавляется. Prior sticker-set contract остаётся отдельным, mixed methods deferred.
- Runtime rule: static atom является prerequisite, не current proof. Evaluator обязан получать account/target-bound current status и invalidates evidence при right/membership/kind/account changes; unknown/stale fail closed.
- Evidence: [supergroup setting-right digest](../raw/2026-07-15-tdlib-supergroup-setting-right-capabilities.md), pinned schema/source archive, public generator test, exhaustive family partition и drift controls.
- Alternatives: сгладить member/admin distinction, считать все methods regular-only, поглотить boost/guard prerequisites или добавить free-form setting DSL отклонены как неточные либо fail-open.
- Consequences: supported typed set растёт 59 → 64, terminal complete 62 → 67, exact open set уменьшается 131 → 126. Capability format остаётся `7`; runtime evaluator не реализован.
- Supersedes: none; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md) и [D-20260715-019](decisions.md).

## [2026-07-15] accepted correction | D-20260715-020 | Ordinary discussion supergroup нельзя доказать broad kind

- Corrects: initial five-method subset и counts в предыдущей `D-20260715-020` entry.
- Evidence: [ordinary-supergroup correction digest](../raw/2026-07-15-tdlib-supergroup-setting-ordinary-kind-correction.md); pinned schema и `ChatManager.cpp` исключают broadcast, gigagroup и monoforum для `toggleSupergroupJoinToSendMessages`.
- Decision: method возвращён в deferred до появления closed ordinary-supergroup predicate и runtime evidence. Current subset: 4 new complete, 1 prior complete, 4 deferred; supported 63, terminal 66, open 127.
- Consequences: broad `ResolvedChatKind::Supergroup` не используется как proof ordinary discussion group. Остальные четыре exact contracts и account/right boundaries не меняются.
- Status: `D-20260715-020` остаётся accepted только с этой correction.

## [2026-07-15] accepted | D-20260715-021 | Chat setting contract сохраняет kind, role-right и account-conditioned guards

- Context: `setChat*` right family смешивает безусловные kind/right checks с boost, input, value, target и account-conditioned branches. Отсутствие `CHECK_IS_USER` в `Requests.cpp` не доказывает отсутствие более глубокого bot-specific guard.
- Decision: `capability/chat_settings.rs` объединяет прежние four supergroup setting contracts и три exact complete chat contracts: permissions, description и slow mode. `setChatTitle`/`setChatPhoto` остаются deferred, потому что bot в basic group дополнительно требует appointed-administrator status; остальные mixed methods также не поглощаются.
- Runtime rule: static right является prerequisite, не current proof. Account/kind-conditioned implication нельзя заменять широкой member-right DNF; missing/stale account, kind или status evidence fail closed.
- Evidence: [chat setting-right digest](../raw/2026-07-15-tdlib-chat-setting-right-capabilities.md), pinned schema/C++ archive, exhaustive family partition, TDD/negative controls и post-fix independent `APPROVED`.
- Alternatives: считать dispatcher единственным account gate, запретить bots целиком, добавить неиспользуемый generic predicate или оставить title/photo member-only отклонены как неточные либо fail-open.
- Consequences: supported typed set растёт 63 → 66, terminal complete 66 → 69, exact open set уменьшается 127 → 124. Capability format остаётся `7`; account-conditioned runtime grammar и evaluator не реализованы.
- Archive link map после ротации: [D-20260715-006](archive/2026-07-15--2026-07-15-005.md), [D-20260715-014 base](archive/2026-07-15--2026-07-15-014.md) и [D-20260715-014 correction](archive/2026-07-15--2026-07-15-015.md).
- Supersedes: none; extends [D-20260715-009](archive/2026-07-15--2026-07-15-008.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md), [D-20260715-017](decisions.md) и [D-20260715-020](decisions.md).

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
