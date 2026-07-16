# F013 Mini App browser handoff

Telegram и browser proof разделены тремя шагами:

1. `workflow run ... prepare_web_app_handoff '<input>'` выполняет typed `openWebApp` и
   возвращает `telegram_status=prepared`, `browser_status=pending`, launch ID, one-shot
   artifact handle и TTL. Root `complete=false` запрещает считать URL UI success.
2. `telegram-webapp-runner <handle> -- <adapter> [args...]` забирает URL ровно один раз
   через owner-matching private daemon socket и передаёт adapter только JSON stdin.
3. `workflow run ... close_web_app_handoff '{"launch_id":...}'` выполняется в finally
   независимо от browser pass/fail.

Artifact не является файлом: URL/init data хранится zeroizing в памяти daemon максимум
60 секунд, исчезает при expiry, take или daemon exit. CLI/MCP response содержит только
opaque handle. Повторный take, другой principal и expired handle дают
`web_app_artifact_unavailable`; новый run всегда требует fresh `openWebApp` artifact.

Browser adapter получает secret-bearing stdin:

```json
{"launch_id":11,"url":"<protected>","require_same_origin":true}
```

и обязан вернуть единственный closed evidence object без URL, DOM text или error strings:

```json
{"passed":false,"dom_assertions":2,"bridge_assertions":1,"network_assertions":1,"js_errors":1}
```

Runner ограничивает response 16 KiB и deadline 30 секунд, глушит adapter stderr,
zeroize-ит request/output buffers и публикует только launch ID, отдельный
`telegram_prepared`, assertion counters, JS error count и `artifact_consumed`. Нулевое
число assertions не является browser proof. Failed evidence даёт exit 4, invalid/oversize
evidence не отражается в stdout.

Synthetic adapter test моделирует SC001: Telegram prepared, browser failed с JS error;
canary отсутствует в args/report. Live Mini App и DOM не запускались — это P10. Remote
MCP topology Q001 не решена: runner-only artifact-take route не публикуется как MCP tool.
