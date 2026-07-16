# CLI session commands

Первый P6 CLI slice заменяет fail-closed placeholder тонким Unix-socket client одной
daemon-owned сессии:

```text
telegram-cli session status
telegram-cli session hold [scope[,scope...]] [ttl_ms]
telegram-cli session release <lease_id>
```

`status` также имеет короткий alias `telegram-cli status`. Profile берётся из
`TELEGRAM_PROFILE` с default `default`; non-secret principal — из `TELEGRAM_PRINCIPAL` с
default `telegram-cli`. Missing scope/TTL означают `read` и `60000` ms. Неизвестный scope
отклоняется клиентом, а daemon повторно применяет owner ceiling и TTL bounds.

Status возвращает fixed-shape operational metrics: active/max leases, request outcomes и
latency, queue/rejections, retry/flood, update lag и freshness counters. Human output
показывает компактный lease/request digest; JSON сохраняет весь typed snapshot.

CLI не открывает TDLib database и не зависит от `telegram-core`: он отправляет один
`telegram-protocol::DaemonRequest` в private profile socket и печатает один JSON response.
Перед connect проверяются profile grammar, current-user directory `0700` и socket
`0600`/one-link/current-user. Auto-start, human output, heartbeat loop и signal cleanup
принадлежат следующим P6/P9 пунктам; текущий `hold` выдаёт lease на bounded TTL.
