# CLI streaming, cancellation и lease cleanup

Шестой P6 slice превращает `events watch` в bounded client-side stream без второго
transport или async runtime:

```text
telegram-cli events watch <lease_id> [cursor]
telegram-cli --output jsonl events watch <lease_id> [cursor]
```

Human и JSONL modes сначала heartbeat’ят переданный matching lease, затем poll’ят
существующий cursor route. Первый record фиксирует baseline; следующие records выходят
только при новых events или `gap=true`. Каждый successful response задаёт следующий cursor.
Heartbeat interval равен трети authoritative lease TTL, полученной от daemon.

`--output json` намеренно остаётся one-shot snapshot: один JSON document не маскируется под
бесконечный stream. JSONL использует тот же envelope v1 по одной записи на event batch.

SIGINT/SIGTERM handler выполняет только atomic store. Обычный control flow замечает
cancellation, отправляет `LeaseRelease`, проверяет `lease_released` (already expired/not
found также означает отсутствие активного lease) и только затем возвращает stable client
error `cancelled`, exit 6. Broken pipe идёт через тот же cleanup, но не пытается писать в
закрытый output повторно. SIGKILL не перехватывается; daemon TTL остаётся kernel-independent
fallback.

Этот slice отменяет watch loop, а не обещает произвольную отмену уже отправленной mutation.
One-shot raw/workflow calls продолжают использовать core absolute deadlines и не получают
blind retry. Behavior test с real private Unix socket доказывает heartbeat -> cancel/pipe ->
release ordering.
