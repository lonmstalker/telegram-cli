# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap создан: product, living plan, HARNESS, TDLib coverage contract и F001–F022.
- Pinned planning baseline описывает TDLib 1.8.66: 1010 functions, 2168 definitions, 184 updates и 13 authorization states.
- Existing encrypted TDLib session ранее достигла Ready/getMe и была закрыта через authorizationStateClosed.
- SSH-доступ и серверный database-key path проверены без вывода значения.
- `.env.local` создан как ignored mode-`0600` source; env contract опубликован без значений, loader проверен.
- Karpathy Wiki использует отдельные work/decision/problem journals и checksum-backed rotation.
- Canonical GitHub remote: `https://github.com/lonmstalker/telegram-cli.git`; public visibility явно принята пользователем.
- P0 начат: Cargo workspace содержит шесть целевых packages, а dependency/target/default-member boundaries защищены executable contract с negative controls.
- До появления runtime все четыре binary entrypoint fail closed; process guard ограничен timeout и очищает всю отдельную process group.
- Initial production schema pin: TDLib `1.8.66`, exact commit `07d3a097...`; vendored schema hash/counts проверяются offline с negative controls.
- Exact macOS arm64 `tdjson` подтверждён crash-safe reviewed rebuild: artifact SHA-256 `5dbd3009...6852e7e`, 27 654 296 bytes; Mach-O/dependencies/exports/version/commit и no-DB smoke проверены.
- Global build lock наследуется всеми watchdog paths; gated handshake, recursive stale recovery и proof-backed finalization проверены parent/inspection `SIGKILL` controls. RSS/tree limits являются sampled thresholds, не kernel hard caps.
- Native binary хранится в ignored content-addressed cache; Git хранит exact policy/recipe/provenance. Одна сборка помечена `reproducibility=not_verified`.
- Strict Rust parser в `telegram-core` разбирает полный pinned corpus без сторонних dependencies: 2168 definitions = 9 builtins + 2159 object constructors, 1010 methods, 745 type families, 184 updates и 13 authorization states. Documentation сохраняется raw и structured, signatures canonical; input cap 2 MiB, type depth cap 32. Independent re-review — Approved.
- Non-default `tdlib-registry-gen` отделён от product packages и реализует bounded deterministic owner rule engine: one-rule-per-feature candidate sets, reviewed set/signature hashes, exact overlap overrides и fail-closed coverage. `check` read-only; `generate` получает один fixed-temp lease до input snapshot, проверяет path/inode identity и публикует atomic rename. Independent re-review — Approved.
- Exact owner corpus закреплён для 1010/1010 methods: 17 rules, 252 atoms, 372 reviewed overlap overrides; schema-derived signatures, 22 per-feature hashes, independent exact owner digest и adversarial semantic boundaries проверяются corpus gate. Canonical artifact остаётся owner-only; independent final review — Approved.
- P0.5a capability foundation закрепляет closed account/auth/entitlement/application/DC vocabularies, additive synchronous path, typed bounded runtime DNF и parameter notices. Pure generator требует exact schema/owner/method/signature/documentation evidence, отклоняет распознанные unsupported capability/runtime gate signals и скрытое policy-сужение; два independent reviews — Approved.

## Not implemented

- Linux x86_64 TDLib artifact, reviewed 1010-method capability corpus, risk/prerequisite/retry classification, generated full schema registry, singleton daemon, рабочий product CLI и MCP ещё не созданы; текущие product binaries являются только fail-closed skeleton.
- Stateful request-chain engine, retry/reconciliation, policy, metrics и agent skill остаются планом.

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты находятся вне model-visible interfaces.
- Gateway key wiring остаётся [P-20260715-001](../problems/problems.md).
- Linux target proof остаётся [P-20260715-003](../problems/problems.md); macOS artifact нельзя считать доказательством Linux или bit-for-bit reproducibility.

## Evidence

- [Bootstrap digest](../raw/2026-07-15-project-bootstrap.md)
- [D-20260715-001](../decisions/decisions.md)
- [D-20260715-002](../decisions/decisions.md)
- [W-20260715-005](../logs/work.md)
- [D-20260715-003](../decisions/decisions.md)
- [W-20260715-006](../logs/work.md)
- [Reviewed native macOS arm64 correction digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md)
- [D-20260715-004](../decisions/decisions.md)
- [D-20260715-005](../decisions/decisions.md)
- [W-20260715-008](../logs/work.md)
- [Strict schema parser/inventory digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md)
- [D-20260715-006](../decisions/decisions.md)
- [W-20260715-009](../logs/work.md)
- [TDLib owner generator digest](../raw/2026-07-15-tdlib-feature-owner-generator.md)
- [D-20260715-007](../decisions/decisions.md)
- [W-20260715-010](../logs/work.md)
- [TDLib owner corpus digest](../raw/2026-07-15-tdlib-feature-owner-corpus.md)
- [D-20260715-008](../decisions/decisions.md)
- [W-20260715-011](../logs/work.md)
- [TDLib capability generator foundation digest](../raw/2026-07-15-tdlib-capability-generator-foundation.md)
- [D-20260715-009](../decisions/decisions.md)
- [W-20260715-012](../logs/work.md)
