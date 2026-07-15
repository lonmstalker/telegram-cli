# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap: `product.md`, `plans.md`, `HARNESS.md` (F001–F022), harness-файлы, `docs/tdlib-api-coverage.md`.
- Cargo workspace из шести пакетов; границы под gate `scripts/check-workspace-boundaries.py`; product binaries — fail-closed заглушки.
- Pinned schema: TDLib `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`; 1010 functions, 2168 definitions, 184 updates, 13 auth states; gate `scripts/check-tdlib-pin.py`.
- Strict schema parser в `telegram-core::schema` (12 тестов, без внешних dependencies).
- macOS arm64 `tdjson` c provenance в content-addressed cache; gate `scripts/check-tdlib-native-pin.py`.
- Ручное capability-ревью: 74 supported contract и 116 deferred методов сохранены в `docs/capability-notes.md`. Recognizer engine удалён ([D-20260715-035](../decisions/decisions.md)); классификация — данные с default-deny.
- Существующая зашифрованная сессия ранее достигала Ready/getMe; database key получен; `.env.local` contract (mode `0600`, protected loader) настроен.
- Canonical GitHub remote: `https://github.com/lonmstalker/telegram-cli.git` (public, принято пользователем).

## Not implemented

- Весь runtime P1–P10: TDJSON transport, авторизация, daemon, generated registry, capability-таблица, workflows, policy, CLI, MCP, packaging.
- Linux x86_64 native artifact ([P-20260715-003](../problems/problems.md)).

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты — вне model-visible interfaces.
- Gateway key wiring — [P-20260715-001](../problems/problems.md).
- Неотревьюенные методы — default-deny; это валидное состояние, не блокер (см. `plans.md`, «Правила работы»).
