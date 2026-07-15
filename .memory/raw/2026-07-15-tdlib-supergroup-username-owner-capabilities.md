# TDLib supergroup username owner capability digest

Дата: 2026-07-15.

## Scope

Этот digest фиксирует exact reviewed subset методов управления username супергруппы или канала. Он не утверждает, что текущий аккаунт уже является владельцем, и не заменяет runtime freshness/evaluator.

## Pinned schema evidence

В `vendor/tdlib/td_api.tl` четыре метода имеют самостоятельное method-level требование `requires owner privileges in the supergroup or channel`:

- `disableAllSupergroupUsernames supergroup_id:int53 = Ok;`
- `reorderSupergroupActiveUsernames supergroup_id:int53 usernames:vector<string> = Ok;`
- `setSupergroupUsername supergroup_id:int53 username:string = Ok;`
- `toggleSupergroupUsernameIsActive supergroup_id:int53 username:string is_active:Bool = Ok;`

Их target всегда `supergroup_id:int53`, а допустимый runtime kind — `supergroup` или `channel`. Поэтому exact prerequisite является DNF из двух веток: `ChatKind(supergroup) AND ChatOwner` либо `ChatKind(channel) AND ChatOwner`.

Schema-derived family `RequiresOwnerPrivileges` исчерпывающе разделена на эти четыре complete contracts, ранее complete `upgradeBasicGroupChatToSupergroupChat` и 13 mixed/deferred methods. Mixed owner/filter/value/password/input semantics не поглощаются этим subset.

## TDD and fail-closed evidence

- Public generator red: без exact contract `setSupergroupUsername` отклонялся как `InvalidPolicy` из-за противоречия method capability и description.
- Pinned-family red: `disableAllSupergroupUsernames` оставался без terminal typed disposition и давал `SchemaDrift`.
- Green: exact signatures и normalized description sources закреплены для всех четырёх methods; source drift, signature drift и дополнительный owner signal в argument documentation снова дают `SchemaDrift`.
- Policy с пропущенным `ChatOwner` atom или `ready_accounts=[bot]` отклоняется.
- Exact family hashes: complete subset `b093777061da4be0622087c24d81db5116ff19c06d2028d228d369adf916d185`, deferred subset `f94f95c9e369c9e00b48492827522039251657a4a64ae9f33332d843449d461e`, consumed rows `65656db662de8295423d199a06c785503c86c5d51f591aaa2321731f64dbc566`.
- Global typed supported set: 57 methods, SHA-256 `4454d9ff7d0b979f60cd10639507cd73fab248f9bcdcaf77226ed473e868b7e8`; terminal set: 60; exact open set: 133, SHA-256 `cd2b13cc68f18956f113592b505ec4469c564e3f7ce4298e7e4093b172e5a914`.

## Runtime boundary

Static policy описывает требуемый факт, но не доказывает его истинность. Будущий evaluator обязан получать current membership/status для того же `supergroup_id`, различать owner от administrator/member и считать missing, stale, mismatched-account или incomplete evidence ложным. Capability format остаётся `7`: новый serialized vocabulary не добавлялся.
