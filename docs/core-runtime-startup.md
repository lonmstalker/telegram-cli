# Core runtime startup contract

`telegram-core::runtime::CoreRuntime` завершает P1 boundary между прямым TDJSON transport и daemon-owner. Runtime не открывает profile/DB сам и не подменяет P2 lifecycle; его единственный production consumer теперь [`telegramd` session lifecycle](daemon-session-lifecycle.md).

## Startup order

Все шаги используют один абсолютный deadline:

1. Запускается единственный TDJSON receive loop.
2. Первым request отправляется `setLogStream(logStreamEmpty)`, до любых secret-shaped payloads.
3. `getOption version` и `getOption commit_hash` сравниваются с `vendor/tdlib/manifest.json`. Mismatch завершает startup до DB parameters и рабочих calls.
4. `getCurrentState` создаёт snapshot. Transport response boundary отделяет события, наблюдавшиеся до snapshot, от более поздних updates.
5. Pre-snapshot events отбрасываются, snapshot применяется к пустому ordered reducer, post-boundary events остаются в очереди и затем применяются строго в receive order.

`CoreRuntime::next_event_until` продолжает один reducer sequence. Unmatched response не выдаётся за update. `CoreRuntime::shutdown` освобождает только transport resource: штатный account stop в P2 обязан отправить `close` и дождаться `authorizationStateClosed`.

## Acceptance evidence

- Pure backend доказывает exact startup order, mismatch до `getCurrentState` и snapshot boundary semantics.
- Pinned native artifact подтверждает version/commit handshake и начальный `authorizationStateWaitTdlibParameters`.
- Synthetic test-DC DB подтверждает: correct key создаёт DB, normal `close` достигает Closed, wrong key возвращает 401 без перехода к phone authorization и не меняет DB bytes; missing key отклоняется до TDLib.
- Protected live gate подтверждает returning production session: параметры подаются через `.env.local` loader, состояние достигает Ready, `getMe` подтверждает user object, `close` достигает Closed без phone/OTP/2FA input.
- `scripts/check-core-secret-output.py` запускает native secret canary после отключения TDLib logs и содержит внутренний negative control scanner. Runtime P1 не имеет metrics surface; Rust `Debug`/error contracts отдельно проверяют redaction.

Sanitized command/result digest: [P1 runtime acceptance](../.memory/raw/2026-07-15-p1-runtime-acceptance.md).
