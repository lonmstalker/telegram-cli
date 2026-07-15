# TDLib supergroup subtype capability contracts

Date: 2026-07-15

## Scope

Digest фиксирует schema-bound Boolean subtype facts, один exact static capability contract и reviewer-enforced deferred invite family. Он не доказывает runtime freshness, не реализует daemon и не утверждает live readiness.

## Pinned source

- TDLib commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `vendor/tdlib/td_api.tl` содержит exact ordered `supergroup` constructor с `is_broadcast_group` и `is_direct_messages_group`, а также ingress `updateSupergroup supergroup:supergroup = Update;`.
- [`Requests.cpp` lines 6082-6094](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L6082-L6094): `addChatMember` и `addChatMembers` проходят через `CHECK_IS_USER()`.
- [`DialogParticipantManager.cpp` lines 2471-2496](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogParticipantManager.cpp#L2471-L2496) и [2671-2708](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogParticipantManager.cpp#L2671-L2708): singular basic/channel path отделяет self-join до обычной invite-right/subtype проверки.
- [`DialogParticipantManager.cpp` lines 2308-2324](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogParticipantManager.cpp#L2308-L2324): plural method при одном `user_id` допускает basic-group path и делегирует в singular handler.
- [`DialogParticipantManager.cpp` lines 2771-2800](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogParticipantManager.cpp#L2771-L2800): plural channel path запрещает bot/direct-messages group и требует `can_invite_users` до отправки request.
- [`Requests.cpp` lines 7220-7225](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L7220-L7225): `toggleSupergroupJoinToSendMessages` regular-user-only.
- [`ChatManager.cpp` lines 3460-3469](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/ChatManager.cpp#L3460-L3469): target обязан быть non-broadcast, non-gigagroup, non-monoforum и иметь `can_restrict_members`.
- [`ChatManager.cpp` lines 9850-9862](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/ChatManager.cpp#L9850-L9862): `is_gigagroup` отображается в `supergroup.is_broadcast_group`, `is_monoforum` — в `supergroup.is_direct_messages_group`.

## Model

- Closed `SupergroupFlag` vocabulary содержит только два доказанных поля: `is_broadcast_group` и `is_direct_messages_group`.
- `SupergroupFlagCondition` связывает flag, ожидаемый Boolean и typed chat target; contradictory values и flag на private/basic/secret kind rejected при построении DNF.
- Generator принимает этот atom только при exact pinned `supergroup`/`updateSupergroup` vocabulary. Capability artifact format увеличен с `7` до `8`.
- `toggleSupergroupJoinToSendMessages`: regular user; `supergroup`, `is_broadcast_group == false`, `is_direct_messages_group == false`, `can_restrict_members` administrator right.
- `addChatMember` остаётся deferred: pinned basic-group/channel handlers разделяют self-join и добавление другого пользователя. Без typed `user_id == current_account_id` predicate единый `can_invite_users` contract не является exact.
- `addChatMembers` также остаётся deferred: при `user_ids.len() == 1` pinned handler допускает basic-group path и делегирует в тот же self/non-self flow. Public description уже фактического parameter-dependent handler; нужны typed cardinality и current-user predicates.

## TDD and exact oracles

- Red model test preceded the new domain atom; pinned ordinary-setting test сначала получил `SchemaDrift`. Reviewer regressions затем доказали self/cardinality branches обоих invite methods, после чего speculative invite module удалён. DTO/canonical tests закрепляют schema parsing и output shape нового atom.
- Supported typed set: 66, SHA-256 `502aa34506879a1c37f8fbbb1e3e0dea0617a344e75158c430e386a1faa51fb5`.
- Terminal set: 69, SHA-256 `f85d132a7d4e1bfe5a2997ddd29bb1cac41c2c985f16eab1f8c4a54d0d0be731`.
- Open set: 124, SHA-256 `437c17ed2ccb09f23aa7eba6b04223e0b05a97ae55493d280fa18f28fe7ce796`.
- Semantic disposition rows: SHA-256 `49a6419cd0ddaca17dfd856d54c1eb2ec4120a176b583641d572e4dd99aac051`.
- Final verification: 58 generator + 24 core = 82 workspace tests; Clippy `-D warnings`, rustfmt, planning/workspace/schema/native/skeleton/process/rotation/diff gates green with `jobs=2`. `target` — 151 MiB, project process leftovers — 0.

## Boundary

Static subtype facts должны поступать из daemon-owned, актуального `supergroup` snapshot. Пока runtime ingestion/invalidation не реализованы, contract остаётся schema semantics, а не утверждением о текущем Telegram state.

`addChatMembers` wording про channel member count больше 200 описывает dynamic operation outcome/resource prerequisite и возвращается через `FailedToAddMembers`; это не причина deferred state. `discussion supergroup`/`has_linked_chat` влияет на effective join-to-send value, но pinned toggle handler не использует linked-chat state как availability gate. Оба аспекта остаются для будущего prerequisite/postcondition layer и не входят в текущую static caller-capability DNF.
