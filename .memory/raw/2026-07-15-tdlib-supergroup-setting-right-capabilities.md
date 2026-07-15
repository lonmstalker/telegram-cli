# TDLib supergroup setting-right capability digest

Дата: 2026-07-15.

## Scope

Этот digest фиксирует пять безусловных методов настройки supergroup/channel с exact `can_*` right. Boost-dependent и guard-bot-input-dependent методы остаются deferred.

## Pinned schema and source evidence

Complete subset:

- `setSupergroupMainProfileTab`: channel + administrator `can_change_info`, regular user only.
- `setSupergroupUnrestrictBoostCount`: supergroup + administrator `can_restrict_members`, regular user или bot.
- `toggleSupergroupIsAllHistoryAvailable`: supergroup + member `can_change_info`, regular user only.
- `toggleSupergroupJoinToSendMessages`: supergroup + administrator `can_restrict_members`, regular user only.
- `toggleSupergroupSignMessages`: channel + member `can_change_info`, regular user only.

Из pinned source archive SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb` потоково проверен `Requests.cpp`: четыре regular-only handler имеют `CHECK_IS_USER()`, `setSupergroupUnrestrictBoostCount` не имеет account restriction. `ChatManager.cpp` для последнего метода проверяет megagroup kind, `status.can_restrict_members()` и bounded value `0..=8`.

Schema-derived setting-right family исчерпывающе разделена на пять new complete contracts, ранее complete `setSupergroupStickerSet` и три deferred methods:

- `setSupergroupCustomEmojiStickerSet` зависит также от boost level;
- `toggleSupergroupHasAutomaticTranslation` зависит от boost level и enabled value;
- `toggleSupergroupJoinByRequest` имеет conditional guard-bot administrator/right/type prerequisites.

## TDD and fail-closed evidence

- Public и pinned red дали `SchemaDrift`, пока role/right signals не имели typed disposition.
- Green использует только existing `ChatKind`, `ChatAdministratorRight` и `ChatMemberRight` atoms.
- Wrong chat kind и подмена administrator right на member permission дают `InvalidPolicy`.
- Exact source/signature drift и дополнительный right signal в argument documentation дают `SchemaDrift`.
- Supported typed set: 64 methods, SHA-256 `a6faa61e6d58a50db20f708982b47fa514d342aa4bb6ae5cbb0b933ed4f3764f`; terminal set: 67, SHA-256 `3c4b0d8c3150d4d4feb85bf6c54df38b254393e6fb98f10a1aee1c994fdd89df`; open set: 126, SHA-256 `71a75f389b248af4aeeb0e387e7be299d56d964f4969652f32bb3cfdcb47be9d`.

## Runtime boundary

Static right atom не доказывает current status. Будущий evaluator обязан использовать account/target-bound current membership/status и invalidation при permission, membership, kind или account changes. Missing, stale или incomplete evidence fail closed. Capability format остаётся `7`; serialized vocabulary не расширялся.
