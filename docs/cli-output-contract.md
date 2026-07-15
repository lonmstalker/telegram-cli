# CLI output и exit-code contract

Пятый P6 slice задаёт один formatter поверх `telegram-protocol`:

```text
telegram-cli [--output human|json|jsonl] <command...>
```

Default — `human`; его формулировки не являются API. `TELEGRAM_OUTPUT` задаёт тот же
default без дублирующих flags. `json` и `jsonl` для one-shot command выводят одну и ту же
compact newline-terminated запись. Streaming slice продолжит `jsonl` несколькими records,
не меняя envelope.

Machine envelope version 1:

```json
{"version":1,"status":"ok","data":{"type":"session_status","active_leases":0}}
```

Root `status` принимает только `ok`, `partial`, `error`. Daemon добавляет authoritative
`complete` к каждому workflow response из typed core outcome; `complete=false` и event
`gap=true` становятся `partial`. Поэтому agent не выводит terminal state из human prose
или формы конкретного workflow payload.

`error` содержит только закрытый domain/code:

```json
{"version":1,"status":"error","error":{"domain":"client","code":"invalid_arguments"}}
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

Human errors идут в stderr, human success — в stdout. Machine envelope всегда идёт в
stdout, включая structured errors; stderr не нужен для machine decisions.
