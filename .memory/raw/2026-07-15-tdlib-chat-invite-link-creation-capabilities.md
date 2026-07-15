# TDLib chat invite-link creation capability digest

Дата: 2026-07-15.

## Scope

Этот digest фиксирует только два безусловных метода создания chat invite link. Методы управления собственной или чужой ссылкой зависят от invocation data/creator identity и остаются deferred.

## Pinned schema and source evidence

Exact pinned contracts:

- `createChatInviteLink chat_id:int53 name:string expiration_date:int32 member_limit:int32 creates_join_request:Bool = ChatInviteLink;`
- `replacePrimaryChatInviteLink chat_id:int53 = ChatInviteLink;`

Обе descriptions разрешают basic groups, supergroups и channels и требуют administrator privileges вместе с `can_invite_users` right. Поэтому static prerequisite является трёхветочным DNF: соответствующий `ChatKind` в conjunction с `ChatAdministratorRight(can_invite_users)`.

Из закреплённого source archive SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb` потоково проверены `Requests.cpp` и `DialogInviteLinkManager.cpp`. Оба request handler вызывают `export_dialog_invite_link`; перед network query `can_manage_dialog_invite_links` проверяет write access, отклоняет private/secret chats, требует active basic group и `status.can_manage_invite_links()` для basic group/supergroup/channel. `CHECK_IS_USER()` отсутствует, поэтому regular user и bot остаются допустимыми account kinds.

## Exact partition and TDD

Triple-signal family `RequiresAdministrator + RequiresRightPhrase + NamedRight(can_invite_users)` исчерпывающе разделена на два complete methods и девять mixed methods: edit/get/list/members/revoke/delete/join-request paths. Mixed rows содержат branch `own link -> admin+right; other link -> owner` и не поглощаются без typed invocation predicate.

- Public red и pinned red дали `SchemaDrift`: все три signal keys оставались без typed disposition.
- Green contract сериализует три explicit branches и сохраняет `ready_accounts=[regular_user, bot]`.
- Missing basic-group branch и подмена administrator right на member permission дают `InvalidPolicy`.
- Exact source/signature drift и дополнительный right signal в argument documentation дают `SchemaDrift`.
- Safe-set SHA-256: `91ddac463f4dcc4d43579e97bd8fdb2e831e0e8413cf288890b91ba73e049f27`; mixed-set SHA-256: `8309b84a00eb0d95f99593c2c5cfd01dfe21ccdf7b2ca89f956bc6094b49545f`.
- Global supported set: 59 methods, SHA-256 `564475074ddd25c973a007b3c719a1a66cf36a3b9e39b4bca26fb629f450252f`; terminal set: 62 methods; open set: 131, SHA-256 `49480a48f3c072d8b3621c5d8e64ada2f1eacb13c697feed31279490e8886fbf`.

## Runtime boundary

Static `ChatAdministratorRight` не доказывает current permission. Будущий evaluator обязан использовать current, account-bound, target-bound status, учитывать active/deactivated group state и invalidation при membership/right/account changes. Missing, stale или incomplete evidence fail closed. Capability format остаётся `7`: serialized vocabulary не расширялся.
