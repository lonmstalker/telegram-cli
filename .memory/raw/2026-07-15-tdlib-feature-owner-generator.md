# Immutable digest: P0.4a TDLib feature-owner generator

Date: 2026-07-15

## Scope

Этот digest фиксирует только bounded generator/rule/publication infrastructure. Reviewed corpus policy `policy/tdlib-feature-owners.json` и generated 1010-row artifact в scope не входили и на момент checkpoint отсутствуют. Capability, risk, prerequisite, retry, codec, router и runtime parity не доказаны.

## Source evidence

- `crates/telegram-core/src/feature.rs`: закрытый inventory `FeatureId::F001..F022`, canonical string/parser contract.
- `tools/tdlib-registry-gen`: отдельный non-default workspace binary, local dependency только на `telegram-core`.
- `tools/tdlib-registry-gen/src/engine.rs`: strict JSON DTOs, schema binding, unordered one-rule-per-feature candidate engine, rule-set/signature SHA-256, exact overlap overrides, canonical full-output-only JSON.
- `tools/tdlib-registry-gen/src/app.rs`: fixed inputs/output, read-only `check`, single fixed-temp `generate`, bounded regular-file snapshots, open-handle identity checks, atomic rename и parent sync.
- `scripts/check-workspace-boundaries.py`: product/tool topology и четыре executable negative controls.
- `plans.md`, `docs/tdlib-api-coverage.md`, `D-20260715-007` и `W-20260715-010`: accepted scope и explicit deferred boundaries.

## TDD evidence

Initial red phases были наблюдаемыми compile failures: отсутствовали `FeatureId`, затем `generate`/`GenerationErrorKind`. Engine green fixtures закрепили rule-order/atom-order invariance, same-count hash drift, unowned/ambiguous failure, exact overlap override, stale candidates/signature, duplicate/dead/generic policy, unknown fields/features, redundant override и direct input caps.

File/publication red controls сначала доказали реальные дефекты:

- deterministic writer test не компилировался до lease-aware generate path;
- reviewer P1: input snapshot строился до writer lease и мог быть поздно опубликован поверх нового output;
- symlink input принимался как обычный file (`OutOfDate` вместо path error);
- заменённый temp inode успешно rename-публиковался вместо отказа.

После correction `generate` получает fixed-temp lease до чтения inputs. Barrier test удерживает writer A после lease, меняет policy, доказывает fail-fast второго writer и публикацию A только по новому snapshot. Leaf symlink inputs/output и symlink output directory fail closed. Temp/file/directory identity сверяется по open handle и Unix device/inode до/после critical operations; handle остаётся открыт через rename, а cleanup не удаляет foreign successor.

## Determinism and fail-closed contract

- Rules, atoms, examples, overrides и candidate sets canonicalized независимо от input JSON order/whitespace.
- Rule не выбирается по priority: каждый метод получает полный `BTreeSet<FeatureId>` candidates.
- `0` candidates или `>1` без override прекращают generation без partial manifest.
- Override допустим только для actual overlap, owner обязан входить в pinned candidates; signature hash и rationale обязательны. Redundant/unknown/stale override отклоняется.
- Raw match set каждого rule закреплён count и SHA-256 по sorted `method + NUL + canonical_signature_sha256 + LF`.
- Output содержит exact schema/vendor/policy/generator evidence, 22 per-feature summaries и sorted method rows. Нет timestamps, absolute paths или host state.
- F020 не имеет fallback/else semantics; generic verb prefixes отклоняются, а любой matched-set drift ломает reviewed hash.

## Resource and ownership boundary

- Input/output caps: vendor manifest 64 KiB, TDLib schema 2 MiB, policy 1 MiB, generated output 4 MiB.
- Semantic caps: 2048 methods, 22 rules, 512 atoms, 1024 overrides, 64-byte atoms/method evidence и 1024-byte rationale.
- Tool однопоточный, offline, без subprocess/network/daemon/runtime DB.
- `check` не создаёт temp и не пишет output.
- `generate` использует ровно один fixed sibling temp; concurrent/stale temp блокирует новый writer, а не создаёт новые instances.
- Normal/error cleanup оставляет owned temp отсутствующим; foreign replacement не удаляется.
- Same-user hostile mutation не считается sandboxed: процесс с правом менять checkout уже может менять tracked files напрямую. Implemented identity/symlink checks защищают cooperative build boundary и обнаруживают observed substitution, но не заявляются OS isolation.

## Verification and review

Fresh bounded commands used for the checkpoint:

```text
cargo test --offline -p tdlib-registry-gen -p telegram-core --jobs 2
cargo clippy --offline -p tdlib-registry-gen -p telegram-core --all-targets --jobs 2 -- -D warnings
python3 scripts/check-workspace-boundaries.py
cargo fmt --all -- --check
git diff --check
python3 scripts/rotate-wiki-journal.py --all --check
```

Package evidence before final whole-workspace gate: 16 `tdlib-registry-gen` tests и 14 `telegram-core` tests passed; workspace contract reported four negative controls. Independent senior Rust reviewer initially found one P1 stale-snapshot race and two P2 path/temp edges. После TDD corrections повторное review вернуло `Approved`, findings отсутствуют.

## Explicit next gate

Следующий отдельный commit обязан добавить reviewed schema-bound rules/overrides и canonical exact owner rows для всех 1010 parsed methods. Обязательные adversarial boundaries включают `Start` vs `Star`, `Callback` vs `Call`, `testCall*`, Business start page и login-email/log overlap. До green corpus check parent P0 owner acceptance остаётся open.
