# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] archive pointer | D-20260715-008 | Exact owner corpus закрепляет domain ownership, а не runtime parity

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-007.md). Pointer only; решение не изменено.

## [2026-07-15] archive pointer | D-20260715-009 | Static capability requirements отделены от runtime truth и policy permission

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-008.md). Pointer only; решение не изменено.

## [2026-07-15] accepted | D-20260715-016 | Runtime boolean option является generation-bound typed evidence, а не Premium entitlement

- Context: documentation `getOption("...")` смешивает method availability, argument transformation и multi-owner DNF. Free-form option name или отождествление option с Premium entitlement дали бы ложную capability.
- Decision: pin exact four-constructor `OptionValue`, `getOption`/`updateOption` ingress и closed three-name vocabulary. Только exact `setNewChatPrivacySettings` source/signature получает `BooleanOptionEnabled(can_set_new_chat_privacy_settings)`; method regular-user-only по `CHECK_IS_USER()`, atom account-neutral. `postStory` и withdrawal остаются deferred. Positive paid-message payload сохраняет отдельный `can_enable_paid_messages` prerequisite.
- Freshness rule: W020 не реализует runtime evaluator. Будущий store принимает только `optionValueBoolean(true)` из complete ordered stream текущей TDLib session/account/DC generation. Missing, empty, false, wrong-typed, reset, incomplete или gap-affected evidence всегда false; смена generation invalidates state. Для complete gapless stream не вводится искусственный TTL.
- Evidence: [runtime boolean option digest](../raw/2026-07-15-tdlib-runtime-boolean-options.md), exact family3/safe1/deferred2 partition, pinned source hashes, public generator/negative tests и три independent approved reviews. Reviewer P2 exact-name oracle исправлен до repeat approval.
- Alternatives: generic string option atom, Premium-only gate, consumption `postStory`/withdrawal по одному option и трактовка static policy как current runtime truth отклонены как fail-open или семантически неверные.
- Consequences: capability format становится `6`; supported typed set растёт 52 → 53, terminal complete 55 → 56, exact open set уменьшается 138 → 137. Runtime evaluator, option store, prerequisite/risk/retry и zero-open 1010-method artifact остаются отдельными обязательными слоями.
- Supersedes: none; extends [D-20260715-009](decisions.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md) и [D-20260715-015](decisions.md).

## [2026-07-15] archive pointer | D-20260715-010 | Real-schema capability grammar закрывается по exact open set

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-009.md). Pointer only; решение не изменено.

## [2026-07-15] accepted correction | D-20260715-017 | Planning feature IDs не входят в executable architecture

- Context: `F001`…`F022` из `HARNESS.md` были ошибочно превращены в `FeatureId`, owner rule engine, policy/artifact и capability field. Эта taxonomy отражала структуру плана, а не Telegram/TDLib domain.
- Decision: planning IDs используются только для документационной навигации. Runtime/tooling code организуется семантическими модулями; raw API keyed by exact TDLib method/constructor name, canonical signature и schema evidence. Machine-readable policy/registry не содержит planning owner mapping.
- Evidence: [planning-taxonomy removal correction](../raw/2026-07-15-planning-taxonomy-removal.md), `scripts/check-planning-boundary.py`, direct schema+policy capability tests и independent review.
- Alternatives: сохранить numeric enum только в core, заменить номера slug-owner taxonomy или оставить owner artifact как дополнительную metadata отклонены: все варианты продолжают связывать executable contracts со структурой planning inventory без runtime-семантики.
- Consequences: owner engine/CLI/policy/artifact удалены; `tdlib-registry-gen` library-only; capability format `7`, input schema+policy, rows schema-bound. Full registry/risk/retry/capability coverage остаются отдельными open gates.
- Supersedes: [D-20260715-007](decisions.md) и [D-20260715-008](archive/2026-07-15--2026-07-15-007.md); extends [D-20260715-006](decisions.md) и не меняет runtime-capability semantics `D-20260715-009`…`D-20260715-016`.

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
