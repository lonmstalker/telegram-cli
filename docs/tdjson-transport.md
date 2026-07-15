# TDJSON transport contract

Принятый P1 transport реализован в `telegram-core::transport` и `telegram-core::NativeTdJson`. Он отвечает за прямой TDJSON send/receive, correlation, абсолютные deadlines и удаление отменённых pending requests; daemon ownership, retry policy и account lifecycle принадлежат следующим фазам.

## Request path

1. Caller передаёт JSON object без `@extra`.
2. Transport атомарно выделяет `u64` correlation ID, добавляет его как transport-owned `@extra` и отправляет command единственному receive thread.
3. Receive thread регистрирует pending response до backend `send`.
4. Тот же thread один вызывает backend `receive`, разбирает JSON и удаляет `@extra` из результата.
5. Изменённый порядок responses не влияет на callers: response направляется exact pending request по echoed ID.

`PendingResponse::wait_until` и `TdJsonTransport::call_until` принимают один абсолютный `Instant`, поэтому последовательная цепочка не получает новый timeout на каждом шаге. Относительные `wait_timeout`/`call` являются короткими adapters. Явная `cancel` и drop pending handle удаляют correlation из receive-loop registry; поздний response становится `UnmatchedResponse`, а не может попасть другому caller. Caller-provided `@extra`, non-object requests, malformed backend JSON и backend failure fail closed.

## Event path

- Значение без `@extra` публикуется как `TdJsonEvent::Update` в receive order.
- Перед matched response transport публикует `ResponseBoundary` с тем же correlation ID. Core runtime использует boundary ответа `getCurrentState`, чтобы отбросить только pre-snapshot events и сохранить последующие updates.
- Значение с неизвестным/чужим `@extra` не выдаётся за update или response: публикуется `UnmatchedResponse` с raw JSON.
- Malformed JSON/backend failure завершает loop, уведомляет все pending requests и публикует `Fatal`.

Event receiver один: `CoreRuntime` является его consumer, применяет updates к ordered reducer и не запускает второй native receive.

## Native boundary

`NativeTdJson::load` динамически открывает exact macOS/Linux artifact и использует четыре из пяти pinned exports: `td_json_client_create`, `td_json_client_send`, `td_json_client_receive`, `td_json_client_destroy`. `td_json_client_execute` остаётся закреплён artifact gate, но transport/runtime он не нужен; runtime options идут через обычные correlated requests. Loader является `unsafe`: caller обязан передать hash-verified library с exact TDJSON ABI.

`TdJsonTransport::shutdown` завершает только transport/client resource. Он не отправляет TDLib `close`, `logOut` или `destroy` method; graceful account/session lifecycle принадлежит P2 и обязан дождаться `authorizationStateClosed` до остановки transport.

## Verification

- Pure tests разворачивают две параллельные requests и намеренно возвращают responses в обратном порядке; correlation и единственный receive thread проверяются независимо от native library.
- Deadline и explicit cancellation проверяют удаление pending correlation; late response не переиспользуется.
- Updates и unmatched response проверяются в exact receive order; malformed JSON проверяет fail-closed pending behavior.
- Ignored native tests с `TDJSON_LIBRARY_PATH` запускаются отдельно на pinned artifact и подтверждают handshake/current state, secret-output canary, wrong/missing-key fail-closed и returning-session live path.
