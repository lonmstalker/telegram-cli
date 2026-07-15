# CLI login status и update events

Четвёртый P6 CLI slice добавляет две protocol-only команды:

```text
telegram-cli login
telegram-cli events watch <lease_id> [cursor]
```

`login` читает daemon-owned authorization state. Daemon преобразует уже разобранный
`AuthorizationMachine` step в закрытый `LoginState`; CLI не видит TDJSON `@type`, phone,
OTP, 2FA, email, QR link или другие challenge fields. При первом login daemon остаётся
единственным владельцем DB и держит private socket доступным в состоянии `Starting`.
До доказанного `Ready -> getMe -> expected identity` raw calls и workflows возвращают
`runtime_unavailable`. Protected ввод challenge secrets принадлежит следующему P6 slice
про secure TTY и не подменяется flags или raw call.

`events watch` требует действующий matching lease и возвращает только bounded metadata:
monotonic local `sequence`, закрытый `kind`, `next_cursor` и `gap`. Первый вызов без cursor
фиксирует текущую позицию и не выгружает прошлое. Повтор с `next_cursor` возвращает новые
records; cursor вне доступного окна явно выставляет `gap=true`.

Daemon хранит до 1024 metadata records. Raw update payload, message content, Web App data
и authorization values в event protocol не входят. Если workflow применил updates внутри
своей terminal chain, broker не реконструирует их задним числом, а публикует честный
`gap` marker. Непрерывный JSONL stream, cancellation и signal-safe release принадлежат
следующему Tasks-пункту P6; текущая команда является one-shot cursor route.
