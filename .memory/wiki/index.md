# Telegram CLI Wiki

Начинай долговечную работу с этой страницы и открывай только нужные ссылки.

## Canonical project sources

- [Product boundary](../../product.md)
- [Living plan](../../plans.md) — фазы, правила работы, зоны ответственности, acceptance
- [Feature inventory](../../HARNESS.md)
- [TDLib coverage contract](../../docs/tdlib-api-coverage.md)
- [Reviewed capability contracts](../../docs/capability-notes.md)
- [Current project state](project-state.md)

## Memory streams

- [Active work journal](../logs/work.md)
- [Active decision journal](../decisions/decisions.md)
- [Active problem journal](../problems/problems.md)

## Raw evidence

- [Bootstrap source digest](../raw/2026-07-15-project-bootstrap.md)
- [TDLib 1.8.66 schema pin digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md)
- [TDLib macOS arm64 reviewed rebuild](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — current artifact truth
- [TDLib Linux x86_64 native build](../raw/2026-07-15-tdlib-1.8.66-native-linux-x86_64.md) — current artifact truth
- [Schema parser/inventory digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md)

## Current records

- Implementation: P0 in_progress — остался выборочный перенос из `tg-analytics`; см. [project-state.md](project-state.md).
- Открытые проблемы: [P-20260715-001](../problems/problems.md) (gateway key wiring). Linux artifact закрыт в [P-20260715-003](../problems/problems.md).
- Консолидация журналов и удаление capability-движка: [D-20260715-035](../decisions/decisions.md), [W-20260715-039](../logs/work.md).
- Linux x86_64 native artifact: [W-20260715-040](../logs/work.md), [P-20260715-003](../problems/problems.md).

## Operating rules

- Wiki pages — компактный synthesis; обновляются при изменении verified state.
- Work, decisions и problems не смешиваются в одном журнале; гранулярность — пункт Tasks фазы, не отдельный метод (см. `plans.md`, «Правила работы»).
- Raw digests — только для внешних доказательств (сборка, сеть, live-сессия).
- `.env.local` используется только через protected loader; значения не читаются и не записываются в память.
