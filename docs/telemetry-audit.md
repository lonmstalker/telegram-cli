# Operational telemetry и redacted audit

`telegramd::telemetry` даёт один fixed-shape in-process snapshot для latency/outcome,
queue/rejections, retry/flood delay, update lag, freshness и active leases. Metric labels и
произвольные maps отсутствуют, поэтому Telegram identifiers, request fields и message text
не имеют пути в metrics.

Scheduler обновляет queue depth/rejections и flood counters; lease manager — active/max
leases. Raw dispatch и workflow consumer добавляют latency/closed outcome, а bounded raw
retry — retry count. `telegram-cli status` читает тот же shared snapshot через protocol;
отдельный exporter/framework не создаётся.

## Audit boundary

`AuditEvent::operation` принимает schema-validated request, но извлекает только generated
method/risk. Persisted JSONL schema содержит timestamp, method, risk, closed outcome,
latency/queue duration, retry count и reconciliation flag. Payload, Telegram identifiers,
errors и arbitrary context в schema отсутствуют.

Daemon открывает `.telegramd-audit.jsonl` после canonical profile owner lock и передаёт его
raw dispatch path. Файл обязан
быть current-user regular file с одним hard link и exact mode `0600`; symlink и unsafe
metadata fail closed. Каждая append завершается `sync_data`.

## Verification

Behavior test строит valid `setChatDescription` с identifier и secret-shaped canary,
записывает audit event и доказывает присутствие method при отсутствии обоих sensitive
values. Metric test проходит все planned dimensions, а existing secret-output gate
проверяет process output независимо от audit schema.
