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
- [TDLib 1.8.66 macOS arm64 first-build digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64.md) — historical pre-review evidence.
- [TDLib 1.8.66 macOS arm64 reviewed rebuild correction](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — current artifact/resource truth.
- [TDLib strict schema parser/inventory digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md) — reviewed P0.3 parser facts and boundaries.

## Current records

- Implementation: [P0 in progress](project-state.md) — workspace, exact schema, strict parser/inventory и reviewed macOS native pin закрыты через `W-20260715-009`; owner manifest/registry/runtime ещё не реализованы.
- Native pin: [reviewed rebuild correction](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — exact source/schema и crash-safe macOS arm64 artifact закреплены; Linux/reproducibility остаются open.
- Decision: [D-20260715-001](../decisions/decisions.md) — раздельная memory model, rotation и secret boundary.
- Decision: [D-20260715-002](../decisions/decisions.md) — публичный GitHub remote принят как canonical `origin`.
- Decision: [D-20260715-003](../decisions/decisions.md) — initial production schema pin использует exact TDLib commit, не moving branch.
- Decision: [D-20260715-004](../decisions/decisions.md) — binary остаётся в content-addressed local cache, Git хранит exact policy/recipe/provenance.
- Decision: [D-20260715-005](../decisions/decisions.md) — inherited global lease, gated target и proof-backed recovery определяют crash ownership.
- Decision: [D-20260715-006](../decisions/decisions.md) — schema parser остаётся pure strict TDLib subset в `telegram-core`, а policy classification отделена от AST.
- Open problem: [P-20260715-001](../problems/problems.md) — database key ещё не подключён к штатному gateway.
- Open problem: [P-20260715-003](../problems/problems.md) — Linux x86_64 native artifact ещё не закреплён.

## Operating rules

- Raw digests и archive shards immutable.
- Wiki pages являются компактным synthesis и обновляются при изменении verified state.
- Work, decisions и problems никогда не смешиваются в одном журнале.
- `.env.local` используется только через protected loader; значения не читаются и не записываются в memory.
