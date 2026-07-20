# Chat inspection workflow

`workflows::inspect_chat` связывает resolver, authoritative cache и full-info read одним
absolute deadline. Caller передаёт Rust `ChatTarget`: ID, username (с optional `@`), public
`t.me`/`telegram.me` link. Invite-shaped link нельзя передать ни как public link, ни как
inspection target: для него существует отдельный `preview_invite_link`.

Последовательность для доступного chat:

1. schema-validated resolver call возвращает `chat` и correlation boundary;
2. core применяет все ordered updates до boundary, но сам `chat` берёт из прямого response —
   отсутствие дублирующего `updateNewChat` не создаёт false timeout;
3. response `ChatType` выбирает ровно один read: `getUserFullInfo`,
   `getBasicGroupFullInfo` или `getSupergroupFullInfo`;
4. при `open=true` scoped lease вызывает `openChat` до full-info и `closeChat` после него.

Lease имеет explicit close для видимой ошибки cleanup и `Drop` fallback для раннего выхода или
panic. Full-info error не пропускает `closeChat`. Поскольку transport timeout после dispatch не
доказывает, что `openChat` не был применён, timeout его response вызывает ровно один exact
compensating `closeChat`.

Cleanup получает deadline `max(workflow_deadline, now + 4s)`: это отдельное bounded safety window
для correlated acknowledgement после исчерпания основного workflow deadline и с запасом
укладывается в 35-second CLI socket timeout при 30-second daemon workflow boundary. Timeout самого
`closeChat` остаётся uncertain: `Drop` не повторяет уже отправленный close без observable
desired-state probe. `openChat`/`closeChat` classified как `presence/convergent`;
resolver/full-info — `read/safe_read`.

Результат содержит compact resolution, `full_info_kind`, `used_open_lease` и `complete`.
Cached chat, full-info object, description, invite links и message payload не сериализуются.
Неожиданный future constructor является typed error, а не ложным `complete`.

Invite preview не вызывает `joinChat`; membership остаётся отдельным explicit workflow.
Deterministic TDJSON backend проверяет direct-response hydration, success/error paired cleanup,
compensating close после timeout ответа `openChat`, отдельный cleanup ACK после full-info timeout,
raw canaries и отсутствие membership/presence calls в preview.
