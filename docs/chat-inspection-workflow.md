# Chat inspection workflow

`workflows::inspect_chat` связывает resolver, authoritative cache и full-info read одним
absolute deadline. Caller передаёт Rust `ChatTarget`: ID, username (с optional `@`), public
`t.me`/`telegram.me` link или explicit invite link. Invite-shaped link нельзя передать как
public link; это не позволяет случайно сменить read-only ветку.

Последовательность для доступного chat:

1. schema-validated resolver call возвращает correlation boundary;
2. core применяет все ordered updates до boundary и ждёт `updateNewChat`, если chat ещё не
   появился в reducer cache;
3. cached `ChatType` выбирает ровно один read: `getUserFullInfo`,
   `getBasicGroupFullInfo` или `getSupergroupFullInfo`;
4. при `open=true` scoped lease вызывает `openChat` до full-info и `closeChat` после него.

Lease имеет explicit close для видимой ошибки cleanup и `Drop` fallback для раннего выхода или
panic. Full-info error не пропускает `closeChat`. `openChat`/`closeChat` classified как
`presence/convergent`; resolver/full-info — `read/safe_read`.

`checkChatInviteLink` с недоступным `chat_id` возвращает `MembershipRequired`, `complete=false`
и raw preview. Workflow не вызывает `joinChat`; membership остаётся отдельным explicit
workflow. Неизвестный future resolver result сохраняется со status `Unknown`.

Live join/open не выполнялся: текущая evidence — deterministic TDJSON backend с ordered
cache update, paired cleanup и negative invite branch.
