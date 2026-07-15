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
- [TDLib feature-owner generator digest](../raw/2026-07-15-tdlib-feature-owner-generator.md) — reviewed P0.4a engine/publication facts and explicit 1010-policy boundary.
- [TDLib feature-owner corpus digest](../raw/2026-07-15-tdlib-feature-owner-corpus.md) — reviewed exact owner mapping and explicit policy/runtime boundary.
- [TDLib capability generator foundation digest](../raw/2026-07-15-tdlib-capability-generator-foundation.md) — closed bounded static model и fail-closed generator до полного 1010-method corpus.
- [TDLib capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md) — exact 193-method signal set, 188-method open set и all-tag authorization correction.
- [TDLib ChatKind capability semantics](../raw/2026-07-15-tdlib-chat-kind-capability.md) — exact `ChatType` pin, six reviewed conditional contracts и 187-method open set.
- [TDLib per-signal runtime disposition oracle](../raw/2026-07-15-tdlib-runtime-signal-dispositions.md) — exact 208 sources/398 keys, partial-consumption invariant и 185-method open set.
- [TDLib MessageProperties capability semantics](../raw/2026-07-15-tdlib-message-properties-capabilities.md) — exact 39-field vocabulary, 29 typed contracts, four deferred mixed methods и 156-method open set.

## Current records

- Implementation: [P0 in progress](project-state.md) — workspace, exact schema, strict parser/inventory, bounded owner generator, reviewed 1010-method owner corpus, capability foundation/ChatKind/per-signal/MessageProperties semantics и macOS native pin закрыты через `W-20260715-016`; 156 typed dispositions, 1010-method capability corpus, risk/retry, full registry и runtime ещё не реализованы.
- Native pin: [reviewed rebuild correction](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — exact source/schema и crash-safe macOS arm64 artifact закреплены; Linux/reproducibility остаются open.
- Decision: [D-20260715-001](../decisions/archive/2026-07-15--2026-07-15-001.md) — раздельная memory model, rotation и secret boundary.
- Decision: [D-20260715-002](../decisions/archive/2026-07-15--2026-07-15-002.md) — публичный GitHub remote принят как canonical `origin`.
- Decision: [D-20260715-003](../decisions/archive/2026-07-15--2026-07-15-002.md) — initial production schema pin использует exact TDLib commit, не moving branch.
- Decision: [D-20260715-004](../decisions/decisions.md) — binary остаётся в content-addressed local cache, Git хранит exact policy/recipe/provenance.
- Decision: [D-20260715-005](../decisions/decisions.md) — inherited global lease, gated target и proof-backed recovery определяют crash ownership.
- Decision: [D-20260715-006](../decisions/decisions.md) — schema parser остаётся pure strict TDLib subset в `telegram-core`, а policy classification отделена от AST.
- Decision: [D-20260715-007](../decisions/decisions.md) — owner classification живёт в isolated non-default tool; rules строят candidates, exact overrides только разрешают reviewed overlaps.
- Decision: [D-20260715-008](../decisions/decisions.md) — exact owner mapping принят только с schema-derived oracles и semantic review; owner-only artifact не доказывает runtime parity.
- Decision: [D-20260715-009](../decisions/decisions.md) — static capability requirements имеют closed bounded model; распознанные unsupported gate signals и лишнее policy-сужение fail closed, runtime truth остаётся отдельным слоем.
- Decision: [D-20260715-010](../decisions/decisions.md) — capability grammar закрывается малыми reviewed source-family tasks по exact open set; full artifact требует zero-open gate.
- Decision: [D-20260715-011](../decisions/decisions.md) — chat kind является closed typed evidence; channel — refinement `chatTypeSupergroup.is_channel`, не отдельный constructor.
- Decision: [D-20260715-012](../decisions/decisions.md) — method complete только при terminal disposition каждого exact signal key.
- Decision: [D-20260715-013](../decisions/decisions.md) — message-property capability требует exact source, identifier space и scalar/universal cardinality; mixed invocation semantics остаются deferred.
- Open problem: [P-20260715-001](../problems/problems.md) — database key ещё не подключён к штатному gateway.
- Open problem: [P-20260715-003](../problems/problems.md) — Linux x86_64 native artifact ещё не закреплён.
- Open problem: [P-20260715-005](../problems/problems.md) — 156 pinned runtime-signal methods ещё не имеют typed disposition.

## Operating rules

- Raw digests и archive shards immutable.
- Wiki pages являются компактным synthesis и обновляются при изменении verified state.
- Work, decisions и problems никогда не смешиваются в одном журнале.
- `.env.local` используется только через protected loader; значения не читаются и не записываются в memory.
