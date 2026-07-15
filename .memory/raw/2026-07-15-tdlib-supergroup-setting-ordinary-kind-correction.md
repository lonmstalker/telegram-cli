# TDLib supergroup setting ordinary-kind correction

Дата: 2026-07-15.

## Corrects

Этот immutable digest исправляет `2026-07-15-tdlib-supergroup-setting-right-capabilities.md`, где `toggleSupergroupJoinToSendMessages` был преждевременно включён в complete subset через broad `ResolvedChatKind::Supergroup`.

## Evidence

Pinned schema сужает target до `discussion supergroup`; `@supergroup_id` говорит `supergroup that isn't a broadcast group`. Pinned `ChatManager.cpp` дополнительно отклоняет `ChannelType::Broadcast`, `is_gigagroup` и `is_monoforum` с boundary `only for ordinary supergroups`.

Текущий closed `ResolvedChatKind::Supergroup` различает channel и non-channel, но не ordinary/gigagroup/monoforum refinements. Поэтому conjunction `Supergroup AND CanRestrictMembers` недостаточен и мог бы дать ложный positive runtime verdict.

## Correction

- `toggleSupergroupJoinToSendMessages` удалён из `supergroup_settings` complete contracts и возвращён в deferred set.
- Current family partition: 4 new complete, 1 prior complete, 4 deferred.
- Current supported typed set: 63 methods, SHA-256 `f8b3f3dca23edc804aab118ccecfab4e8d8f093f14ea38eff817ec8d7645eabd`.
- Current terminal set: 66 methods, SHA-256 `f9acaf38b390d6698293de72ca67050c3e9dfec46d2e1a3af4754326cd468b0c`.
- Current semantic disposition SHA-256: `84deadee028781d867d5f5f9fde93c5800a56ceae9e8c5edb985864855de16d9`.
- Current open set: 127 methods, SHA-256 `b872e1f38e72845cd22f4a14460655508775545f5301882b8edbc6189265aa8d`.

Future completion требует closed ordinary-supergroup predicate с runtime evidence/invalidation либо иного exact proof. До этого method остаётся `SchemaDrift`.
