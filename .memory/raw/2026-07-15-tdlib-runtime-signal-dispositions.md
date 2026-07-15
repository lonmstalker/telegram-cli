# TDLib per-signal runtime disposition oracle

Дата: 2026-07-15.

## Scope

Этот checkpoint заменяет method-level boolean scan на exact source/family inventory и terminal disposition invariant. Он не реализует deferred runtime predicates и не является полным capability artifact.

## Exact inventory

- Pinned schema по-прежнему содержит 193 methods с runtime-like documentation signals, method-set SHA-256 `cbe074623352b1b4e970af939aed6297e7ce37366d7fd5ad7cedcf1a36848706`.
- Structured scan сохраняет 208 signal sources: 162 `@description` и 46 argument tags. Один method имеет не более трёх signal sources.
- Sources разворачиваются в 398 unique `(method, source, family)` keys; максимум четыре lexical families на source. Key oracle SHA-256 — `b0b95745adac694757ae7a46dcbb4dce048129379c3aefa62da62f04a2476545`.
- Semantic disposition oracle SHA-256 — `bd1ccc88e83cea58e7a3ec161032dfe54e53f244ca89a51db568250f059bca65`.
- Duplicate source tag и signal на tag, не связанный с method argument, дают `SchemaDrift`.

## Disposition invariant

- Closed source model различает `Description` и typed `Argument(ArgumentRef)`; closed family model содержит 22 ветви и сохраняет overlapping lexical evidence.
- Reviewed runtime contract объявляет exact `consumed_signal_keys`. Method считается complete только при полном equality между declared и фактически consumed keys; любой deferred key сохраняет fail-closed `SchemaDrift`.
- Неразобранный description честно остаётся `Deferred(UnclassifiedDescription)`, argument signal — `Deferred(InputPrerequisite)`, два exact resend contracts — `Deferred(RetryCondition)`.
- `getChatBoostFeatures` и `getChatBoostLevelFeatures` признаны terminal lexical non-gates только при exact normalized source wording. Same-family semantic drift не наследует исключение.
- Terminal complete set теперь содержит шесть methods с typed runtime requirements и два exact lexical non-gates; combined set SHA-256 `46b15f70d729b17aa8eec4dd27805d1e18edb39bc2e1df5e94062aa793446a7f`.
- Exact fail-closed open set уменьшен с 187 до 185 methods, SHA-256 `b4b68de316938e83a661cfb9cde4dd3ca33336c6544f37ab0d39aa4b7f3009c8`. Эти 185 methods не считаются capability coverage.

## TDD, review and verification

- Red: новые tests сначала не компилировались без source/family/disposition model; затем corpus выявил broad right-token classification, partial consumption и wording-insensitive non-gate risks.
- Green/refactor: scanner сохраняет overlaps без расширения исходного 193-method recognizer; explicit consumed sets, exact normalized lexical exceptions и negative controls закрывают найденные fail-open paths.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test --locked --offline --workspace --all-targets --jobs 2 -- --test-threads=2`: generator 48/48, core 21/21, остальные targets green.
- Whole-workspace Clippy `-D warnings`, fmt, `git diff --check`, workspace boundary и exact TDLib pin green. Два independent final reviewers дали `Approved`; два P1 review findings были исправлены до acceptance.
- Новых dependencies, threads, subprocesses, network/runtime resources или temp instances нет. `target` — 145 MiB; background Cargo/Rust/product processes после gates отсутствуют.
- Source SHA-256: `capability.rs` — `4ec7ff3f3e3fff1fff318b5362aa18800dbd385acef8c92931e7ee7d72ff59d2`; `capability/tests.rs` — `b2ea04d2e6791ce39119f36c81e9cf3873cf37c83174a7e48babc530bcd1e437`.

## Boundary

- Это in-code exact oracle, не отдельный generated/policy artifact и не новый capability format.
- Typed `MessageProperties`, remaining object/option/group-call evidence и 185 open dispositions остаются следующими source-family tasks.
- Canonical 1010-method capability artifact, risk/prerequisite/retry fields, runtime evaluator, daemon/CLI/MCP и live acceptance не реализованы.
