# `tg-analytics` reuse audit digest

Дата: 2026-07-15. Immutable digest внешнего source/build evidence для Tasks-пункта P0 о выборочном переносе.

## Source boundary

- Repo: `/Users/nikitakocnev/RustroverProjects/tg-analytics`.
- Committed snapshot: `e35c54ce213aa170fb0b411eab614485424b3e60` (`2026-07-14T14:10:23+03:00`).
- Source working tree был dirty. Для inspection и тестов использовался только `git show`/`git archive` exact commit; source repo не изменялся.
- Рассмотрены `crates/telegram-tdlib`, `crates/telegram-agent-gateway`, их tests/README/AGENTS. Analytics apps, NATS, PostgreSQL и orchestration исключены до code transfer.

## Exact snapshot verification

Чистый временный `git archive` запускался с shared locked dependencies и отдельным temporary `CARGO_TARGET_DIR`:

- `cargo test --locked --offline -p telegram-tdlib --lib` — 24 passed, 0 failed.
- `cargo test --locked --offline -p telegram-agent-gateway` — 73 passed, 0 failed.
- Temporary snapshot и target удалены после команд; native/live Telegram не запускались.

Green evidence покрывает backend/config/rate-limit boundaries, redaction/policy, typed gateway, JSONL/compact output, history cursors и WebApp inspect safety. Оно не доказывает архитектуру текущего repo и не заменяет локальные acceptance tests будущих фаз.

## Disposition

- Уже перенесены phase-neutral patterns: repo-local wiki, Feature Logic Harness и protected local secret bootstrap.
- Behavior contracts для transport fake/auth, rate limits, policy/redaction, protocol/JSONL и WebApp split приняты как inputs соответствующих P1/P3/P5/P6/P7, но source files не скопированы преждевременно.
- Прямой TDLib owner в source CLI отвергнут: текущий contract требует единственного `telegramd` owner и lease clients.
- Полный reusable synthesis: `docs/tg-analytics-reuse.md`; durable account/session decision: `D-20260715-036`.
