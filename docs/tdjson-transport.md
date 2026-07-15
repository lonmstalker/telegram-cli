# TDJSON transport contract

Текущий P1 slice реализован в `telegram-core::transport` и `telegram-core::NativeTdJson`. Он отвечает только за прямой TDJSON send/receive и correlation; authorization, reducer/cache, retries, daemon ownership и policy принадлежат следующим Tasks-пунктам.

## Request path

1. Caller передаёт JSON object без `@extra`.
2. Transport атомарно выделяет `u64` correlation ID, добавляет его как transport-owned `@extra` и отправляет command единственному receive thread.
3. Receive thread регистрирует pending response до backend `send`.
4. Тот же thread один вызывает backend `receive`, разбирает JSON и удаляет `@extra` из результата.
5. Изменённый порядок responses не влияет на callers: response направляется exact pending request по echoed ID.

Для ожидания доступен только timeout-bound `PendingResponse::wait_timeout` либо `TdJsonTransport::call(request, timeout)`. Caller-provided `@extra`, non-object requests, malformed backend JSON и backend failure fail closed.

## Event path

- Значение без `@extra` публикуется как `TdJsonEvent::Update` в receive order.
- Значение с неизвестным/чужим `@extra` не выдаётся за update или response: публикуется `UnmatchedResponse` с raw JSON.
- Malformed JSON/backend failure завершает loop, уведомляет все pending requests и публикует `Fatal`.

Event receiver один: ordered reducer следующего Tasks-пункта становится его единственным consumer, а не запускает второй native receive.

## Native boundary

`NativeTdJson::load` динамически открывает exact macOS/Linux artifact и использует четыре из пяти pinned exports: `td_json_client_create`, `td_json_client_send`, `td_json_client_receive`, `td_json_client_destroy`. `td_json_client_execute` остаётся закреплён artifact gate, но этому transport slice не нужен; runtime `getOption version` идёт через обычный correlated request. Loader является `unsafe`: caller обязан передать hash-verified library с exact TDJSON ABI.

`TdJsonTransport::shutdown` завершает только transport/client resource. Он не отправляет TDLib `close`, `logOut` или `destroy` method; graceful account/session lifecycle принадлежит P2 и обязан дождаться `authorizationStateClosed` до остановки transport.

## Verification

- Pure tests разворачивают две параллельные requests и намеренно возвращают responses в обратном порядке; correlation и единственный receive thread проверяются независимо от native library.
- Updates и unmatched response проверяются в exact receive order; malformed JSON проверяет fail-closed pending behavior.
- Ignored native test с `TDJSON_LIBRARY_PATH` запускается отдельно на pinned artifact и подтверждает `getOption version == 1.8.66` без настройки DB/auth.
