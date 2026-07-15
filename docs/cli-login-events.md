# CLI login status и update events

Четвёртый P6 CLI slice добавляет две protocol-only команды:

```text
telegram-cli login
telegram-cli events watch <lease_id> [cursor]
```

`login` читает daemon-owned authorization state. Daemon преобразует уже разобранный
`AuthorizationMachine` step в закрытый `LoginState`; status не содержит phone, OTP, 2FA,
email, QR link или другие challenge values. `login tty` вводит owner secret только через
`/dev/tty` с отключённым echo и передаёт typed input для exact challenge ID; flags, stdin и
caller-authored TDJSON для login запрещены. При первом login daemon остаётся единственным
владельцем DB и держит private socket доступным в состоянии `Starting`. До доказанного
`Ready -> getMe -> expected identity` raw calls и workflows возвращают
`runtime_unavailable`. Полный contract: [`cli-secure-login.md`](cli-secure-login.md).

`events watch` требует действующий matching lease и возвращает только bounded metadata:
monotonic local `sequence`, закрытый `kind`, `next_cursor` и `gap`. Первый вызов без cursor
фиксирует текущую позицию и не выгружает прошлое. Повтор с `next_cursor` возвращает новые
records; cursor вне доступного окна явно выставляет `gap=true`.

Daemon хранит до 1024 metadata records. Raw update payload, message content, Web App data
и authorization values в event protocol не входят. Если workflow применил updates внутри
своей terminal chain, broker не реконструирует их задним числом, а публикует честный
`gap` marker. CLI human/JSONL watch теперь повторяет этот one-shot cursor route с heartbeat,
cancellation и signal-safe release; explicit JSON сохраняет one-shot semantics.
