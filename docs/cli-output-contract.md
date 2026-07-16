# CLI output и exit-code contract

Пятый P6 slice задаёт один formatter поверх `telegram-protocol`:

```text
telegram-cli [--output human|json|jsonl] <command...>
```

Default — `human`; его формулировки не являются API. `TELEGRAM_OUTPUT` задаёт тот же
default без дублирующих flags. `json` и `jsonl` для one-shot command выводят одну и ту же
compact newline-terminated запись. `events watch` продолжает JSONL несколькими records,
не меняя envelope; explicit JSON делает один snapshot.

Machine envelope version 2 добавляет fixed `metrics` object к session status:

```json
{"version":2,"status":"ok","data":{"type":"session_status","metrics":{"requests":0,"succeeded":0,"failed":0,"uncertain":0,"denied":0,"request_latency_ms_total":0,"request_latency_ms_max":0,"queue_depth":0,"queue_depth_max":0,"queue_rejections":0,"retries":0,"flood_waits":0,"flood_delay_ms_total":0,"update_lag_events":0,"update_lag_ms_max":0,"fresh_results":0,"cached_results":0,"stale_results":0,"partial_results":0,"active_leases":0,"active_leases_max":0}}}
```

Root `status` принимает только `ok`, `partial`, `error`. Daemon добавляет authoritative
`complete` к каждому workflow response из typed core outcome; `complete=false`, raw
`reconciliation_required=true` и event `gap=true` становятся `partial`. Raw result также
содержит фактический bounded `retries` counter. Поэтому agent не выводит terminal state из
human prose или формы конкретного payload.

`error` содержит только закрытый domain/code:

```json
{"version":2,"status":"error","error":{"domain":"client","code":"invalid_arguments"}}
```

Domains: `command` для core/workflow rejection, `lease` для session policy и `client` для
local CLI boundary. Arbitrary IO/error text в machine output не попадает.

Stable exit codes:

| Code | Meaning |
|---:|---|
| 0 | Protocol request выполнен; machine `status` всё ещё нужно проверить на `partial` |
| 2 | Invalid command/input/output format/profile |
| 3 | Socket unavailable/unsafe или transport failure |
| 4 | Structured daemon rejection |
| 5 | Invalid daemon response или output failure |
| 6 | Stream cancelled после lease cleanup; при closed pipe повторный output не пишется |

Human errors идут в stderr, human success — в stdout. Machine envelope всегда идёт в
stdout, включая structured errors; stderr не нужен для machine decisions.
