# Telegram CLI Wiki

Начинай долговечную работу с этой страницы и открывай только нужные ссылки.

## Canonical project sources

- [Product boundary](../../product.md)
- [Living plan](../../plans.md)
- [Feature inventory](../../HARNESS.md)
- [TDLib coverage contract](../../docs/tdlib-api-coverage.md)
- [Current project state](project-state.md)

## Memory streams

- [Active work journal](../logs/work.md)
- [Active decision journal](../decisions/decisions.md)
- [Active problem journal](../problems/problems.md)
- [Work archive](../logs/archive/index.md)
- [Decision archive](../decisions/archive/index.md)
- [Problem archive](../problems/archive/index.md)
- [Bootstrap source digest](../raw/2026-07-15-project-bootstrap.md)
- [TDLib 1.8.66 schema pin digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md)

## Current records

- Implementation: [P0 in progress](project-state.md) — Cargo workspace boundary закрыт checkpoint `W-20260715-005`, runtime ещё не реализован.
- Schema pin: [TDLib 1.8.66 source digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md) — exact source/schema закреплены, native artifact ещё не доказан.
- Decision: [D-20260715-001](../decisions/decisions.md) — раздельная memory model, rotation и secret boundary.
- Decision: [D-20260715-002](../decisions/decisions.md) — публичный GitHub remote принят как canonical `origin`.
- Decision: [D-20260715-003](../decisions/decisions.md) — initial production schema pin использует exact TDLib commit, не moving branch.
- Open problem: [P-20260715-001](../problems/problems.md) — database key ещё не подключён к штатному gateway.
- Open problem: [P-20260715-002](../problems/problems.md) — exact target-specific TDLib native artifact ещё не закреплён.

## Operating rules

- Raw digests и archive shards immutable.
- Wiki pages являются компактным synthesis и обновляются при изменении verified state.
- Work, decisions и problems никогда не смешиваются в одном журнале.
- `.env.local` используется только через protected loader; значения не читаются и не записываются в memory.
