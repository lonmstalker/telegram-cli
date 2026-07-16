# Durable idempotency journal

`telegram-core::idempotency::IdempotencyJournal` хранит operation state в owner-only
`.telegramd-idempotency.jsonl` внутри canonical profile database directory. `telegramd`
открывает journal после OS owner lock и до runtime dispatch.

## Запись и восстановление

- `OperationFingerprint` — SHA-256 от domain-separated canonical validated TDJSON request.
  Method/payload/Telegram identifiers в journal не записываются.
- `begin` сначала append-ит `pending` и вызывает `sync_data`; только возвращённый
  `Dispatch` разрешает side effect.
- Доказанный terminal result переводит `pending` в `succeeded` или `failed`; timeout — в
  `uncertain`. Каждая transition durable до возврата caller-у.
- Полный invalid JSONL record или запрещённая transition блокирует startup. Только torn
  final record без newline отбрасывается: предыдущее `pending` затем durable переводится в
  `uncertain`.

После restart или timeout `begin` возвращает `ReconcileRequired`, а не повторяет operation.
`reconcile` принимает typed `Applied`/`NotApplied`/`Unknown` и сохраняет только SHA-256
evidence digest. Только доказанный `NotApplied` переводит record в `failed`, после чего
новый explicit `begin` может разрешить повтор exact fingerprint. `Applied` становится
`succeeded`; `Unknown` остаётся `uncertain`.

## Daemon dispatch

Universal raw call с любым generated retry class кроме `safe_read` проходит `begin` после
lease/policy/approval и scheduler admission, но до transport. Generic mutation result не
является domain terminal proof, поэтому raw response возвращается с
`reconciliation_required=true`, а record остаётся `uncertain`; TDLib error закрывает его как
`failed`. Повтор exact fingerprint получает stable `reconciliation_required`, а не новый
dispatch.

Curated workflows, для которых повтор может создать новый remote artifact, перечислены в
одной `JOURNALED_WORKFLOWS` data table. Их fingerprint состоит из fixed workflow name и
canonical input; `complete=false` сохраняется как `uncertain`. Convergent desired-state
workflows не дублируют journal: они уже перечитывают current state перед mutation.

## Filesystem boundary

Journal открывается без symlink following, обязан быть regular current-user file с одним
hard link и exact mode `0600`. Создание fsync-ит file и parent directory. Единственный
daemon owner обеспечивает одного writer; module требует `&mut self` для transitions.
