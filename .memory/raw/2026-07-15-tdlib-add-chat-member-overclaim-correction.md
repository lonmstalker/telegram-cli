# TDLib `addChatMember` capability overclaim correction

Date: 2026-07-15

## Scope

Этот digest фиксирует source-backed correction существовавшего static capability contract для `addChatMember`. Он не утверждает runtime readiness и не реализует новый predicate.

## Pinned schema

- TDLib commit: `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Exact signature: `addChatMember chat_id:int53 user_id:int53 forward_limit:int32 = FailedToAddMembers;`.
- Exact description: `Adds a new member to a chat; requires can_invite_users member right. Members can't be added to private or secret chats. Returns information about members that weren't added`.

## Pinned implementation evidence

- [`Requests.cpp` lines 6082-6087](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L6082-L6087): dispatcher выполняет `CHECK_IS_USER()` до `add_dialog_participant`; bot account недопустим.
- [`DialogParticipantManager.cpp` lines 2671-2701](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogParticipantManager.cpp#L2671-L2701): channel/supergroup path отдельно отклоняет bot и `is_monoforum_channel`, затем проверяет `can_invite_users`.
- [`ChatManager.cpp` lines 9855-9862](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/ChatManager.cpp#L9855-L9862) передаёт `c->is_monoforum` в позицию `supergroup.is_direct_messages_group`; exact ordered constructor закреплён в `vendor/tdlib/td_api.tl`.

## Defect

Удалённый contract строил три ветки `basic_group|supergroup|channel AND can_invite_users` и считал account scope обычным regular-user+bot. `ResolvedChatKind::Supergroup` не отличает direct-messages group, поэтому contract мог дать supported verdict для двух запрещённых состояний: bot account и direct-messages supergroup.

## Correction

- Одноразовый generic `ReviewedRuntimeContract::MemberRightInKinds` удалён; он использовался только `addChatMember`.
- `addChatMember` снова получает deferred dispositions и `SchemaDrift`, пока grammar не выражает regular-user gate и `is_direct_messages_group == false`.
- Exact supported set: 65, SHA-256 `7f251ea70bf74151d6c7d88cbd61fd8ff9480f7174de17cc59970db531b47cda`.
- Terminal set: 68, SHA-256 `6db6e9c9b3912a99885be768645aab25950ba156fc5b4984c8d144bf436c5430`.
- Open set: 125, SHA-256 `ff2f1639bd2947b460ebac2d7a733e71556619db8804ebe49f7410e73cd13af6`.
- Semantic disposition rows: SHA-256 `9bc6e056433a91a35e95a6278852286817e68266b41f9b53eb1ebbc41aa012dc`.

## Boundary

Correction только удаляет fail-open claim. Будущий complete contract требует closed schema-bound supergroup subtype fact, account constraint и fail-closed runtime freshness; broad `ChatKind::Supergroup` повторно использовать нельзя.
