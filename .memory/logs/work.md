# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] work | W-20260715-008 | P0.2b crash-safe reviewed rebuild

- Цель: закрыть post-review crash/resource gaps первой native build и заменить current artifact facts, не переписывая immutable `W-20260715-007` и первый raw digest.
- Sources: [reviewed rebuild correction digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md), exact build policy/provenance и два independent post-build review passes.
- Actions: принят `D-20260715-005`; global lock превращён в inherited watchdog lease, добавлены gated target handshake, recursive state-aware startup recovery, proof-backed reap finalization и immutable archive/OpenSSL snapshots. `P-20260715-004` прошёл open → resolved.
- Verification: offline `jobs=2` rebuild дал SHA-256 `5dbd3009...6852e7e`, 27 654 296 bytes; build 330.225 s, peak sampled RSS 2 064 613 376 bytes, tree 310 298 581 bytes, processes 13. Оба checker mode и 17 provenance controls green; шесть crash/build suites, schema gates, Mach-O/dependency/export/version/commit/no-DB smoke green; cache `1`, leftovers `0`, `target` 42 MiB.
- Corrections: SHA `99e7cdb1...fbb6b49`, size 27 637 784, первые metrics и `16` provenance controls из `W-20260715-007` являются historical pre-review facts; sampled thresholds не называются kernel hard caps.
- Decisions: [D-20260715-005](../decisions/decisions.md).
- Problems: [P-20260715-004](../problems/problems.md) resolved; [P-20260715-003](../problems/problems.md) остаётся open для Linux x86_64.
- Next: independent final docs/diff review и отдельный commit; затем P0 schema parser/inventory/feature-owner manifest через TDD.

## [2026-07-15] correction | W-20260715-008 | Exact input isolation terminology

- Corrects: фраза `immutable archive/OpenSSL snapshots` в Actions основной entry была шире implementation.
- Exact behavior: source archive копируется через один `O_NOFOLLOW` fd в private mode-`0400` snapshot и только он передаётся extractor; OpenSSL остаётся exact resolved Cellar input `/opt/homebrew/Cellar/openssl@3/3.6.2`, а `libssl.a`/`libcrypto.a` проверяются по bytes/SHA-256 до configure и после build.
- Boundary: OpenSSL archives не копируются в immutable private snapshot; current artifact/provenance и verification result от этой терминологической correction не меняются.
- Evidence: [reviewed rebuild correction digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md), `stage_verified_file` и `verify_static_openssl_archives` в `scripts/build-tdlib-native.py`.

## [2026-07-15] work | W-20260715-009 | P0.3 strict schema parser и deterministic inventory

- Цель: разобрать exact pinned `td_api.tl` в проверяемую доменную модель до feature/policy classification, не расширяя wire surface и build footprint.
- Sources: [strict parser digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md), exact schema pin, `plans.md`, F003 harness и independent grammar/review audit.
- Actions: принят `D-20260715-006`; через red-green-refactor реализованы pure parser, role-specific lexical/type/arity validation, raw+structured documentation, canonical signatures, type-reference validation и sorted inventory. Девять builtins отделены от 2159 object constructors; protocol boundary и single-lib-target contract сохранены.
- Verification: full corpus подтвердил 2168 definitions, 1010 methods, 745 type families, 184 updates и 13 authorization states; 12 tests покрывают malformed delimiter/signature/field/type/casing/reserved/generic cases, depth 32 и input cap 2 MiB. Clippy/fmt/workspace boundary green; reviewer после исправления двух P1 и одного P2 дал `Approved` без новых findings.
- Decisions: [D-20260715-006](../decisions/decisions.md).
- Problems: новых нет; [P-20260715-003](../problems/problems.md) не затронута.
- Next: отдельным TDD-коммитом реализовать bounded offline generator и reviewed owner rules/overrides для 1010/1010 methods; не заявлять registry/codec/runtime parity раньше соответствующих gates.

## [2026-07-15] work | W-20260715-010 | P0.4a bounded feature-owner generator

- Цель: создать offline classification/publication boundary до ручного review полного 1010-method owner corpus, не превращая product CLI или schema parser в build tool.
- Sources: [owner generator digest](../raw/2026-07-15-tdlib-feature-owner-generator.md), exact schema/parser evidence, `HARNESS.md`, `docs/tdlib-api-coverage.md` и independent architecture/rule/owner audits.
- Actions: принято `D-20260715-007`; через red-green-refactor добавлены canonical `FeatureId`, isolated non-default `tdlib-registry-gen`, schema-bound rule/override validation, SHA-256 evidence, full-output-only coverage semantics, read-only check и bounded atomic generate. Reviewer P1 stale-snapshot race закрыт lease-before-input; path/temp P2 закрыты symlink/inode negative controls и owned-temp cleanup.
- Verification: 16 generator tests и 14 core tests green; Clippy `-D warnings`, fmt, workspace contract с 4 negative controls и `git diff --check` green. Independent reviewer после corrections: `Approved`, findings отсутствуют. Tool однопоточный; scratch/process leftovers отсутствуют, build footprint остаётся bounded.
- Decisions: [D-20260715-007](../decisions/decisions.md).
- Problems: новых durable problems нет; [P-20260715-003](../problems/problems.md) не затронута.
- Boundary: corpus policy/generated 1010 rows, capability/risk/retry и registry/codec/router/runtime parity не реализованы; parent P0 task и acceptance остаются open.
- Next: отдельным reviewed TDD-коммитом добавить exact schema-bound owner policy, adversarial camel-name controls и canonical 1010/1010 artifact.

## [2026-07-15] work | W-20260715-011 | P0.4b reviewed 1010-method owner corpus

- Цель: закрыть exact feature ownership всех pinned methods отдельным TDD-коммитом, не приписывая owner artifact ещё не реализованные policy/runtime поля.
- Sources: [owner corpus digest](../raw/2026-07-15-tdlib-feature-owner-corpus.md), pinned schema/parser, `HARNESS.md`, feature harnesses и два независимых semantic/review passes.
- Actions: добавлены schema-bound policy из 17 rules/252 atoms/372 exact overrides и canonical 1010-row artifact; принят `D-20260715-008`. Initial mechanically complete draft был заблокирован semantic audit, cross-domain owners исправлены до generation. Corpus tests закрепили exact schema/mapping/per-feature oracles, adversarial boundaries, owner-only shape и bounded real check/generate behavior.
- Verification: read-only generator check, 19 generator и 14 core tests, whole-workspace tests, Clippy `-D warnings`, fmt/diff gates green; independent final reviewer — `Approved`, findings отсутствуют. Один temp corpus ≤1.73 MB очищается через RAII; background processes/leftovers отсутствуют, `target` 117 MiB.
- Decisions: [D-20260715-008](../decisions/decisions.md).
- Problems: новых durable problems нет; [P-20260715-003](../problems/problems.md) не затронута.
- Boundary: capability/risk/prerequisite/retry, constructors/updates/auth-state registry/codec/router и runtime остаются open; parent P0 phase не закрыта.
- Next: отдельным TDD/reviewed коммитом закрепить capability matrix и method policy classes, не расширяя owner-only artifact ложными defaults.

## [2026-07-15] work | W-20260715-012 | P0.5a bounded capability model и fail-closed generator foundation

- Цель: создать closed static capability domain и pure schema-bound generation boundary до ручного review полного 1010-method corpus, не заявляя runtime account truth или policy permission.
- Sources: [capability foundation digest](../raw/2026-07-15-tdlib-capability-generator-foundation.md), pinned schema/owner evidence, `plans.md`, `product.md`, `HARNESS.md` и два independent adversarial reviews.
- Actions: принят `D-20260715-009`; через red-green-refactor добавлены exact account/auth/entitlement/application/DC axes, additive synchronous classification, typed bounded DNF runtime requirements, parameter notices, semantic argument refs и canonical generator. Review findings закрыли extra narrowing, конкретные reviewed runtime/parameter signal gaps, account/runtime incompatibility, public constructor caps и ambiguous `connection_id`.
- Verification: 34 generator и 20 core tests, whole-workspace tests, Clippy `-D warnings`, fmt/diff и workspace boundary green; оба reviewer verdict — `Approved`. Tests запускались с `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2`; `target` 144 MiB, temp/background leftovers отсутствуют.
- Decisions: [D-20260715-009](../decisions/decisions.md).
- Problems: новых durable problems нет; [P-20260715-003](../problems/problems.md) не затронута.
- Boundary: canonical capability policy/artifact для 1010 methods, runtime evaluator, risk/prerequisite/retry, registry/codec/router и live acceptance остаются open; P0 capability checkbox не закрыт.
- Next: отдельным reviewed TDD-коммитом построить и semantically audit exact 1010-row capability policy/artifact поверх этого foundation.

## [2026-07-15] work | W-20260715-013 | P0.5b0 exact capability evidence baseline

- Цель: измерить full pinned capability-like documentation и исправить authorization scan до расширения runtime grammar.
- Sources: [capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md), pinned `td_api.tl`, `plans.md`, `product.md`, `HARNESS.md` и read-only schema/model audits.
- Actions: принят `D-20260715-010`; corpus test закрепил exact 193-method signal set и 188-method fail-closed open set. Authorization recognizer теперь читает все structured documentation tags и учитывает pre-authorization contract из `setCustomLanguagePack.@info`.
- Verification: red test воспроизвёл Ready-only ошибку; 37 generator и 20 core tests, Clippy/fmt/workspace/wiki/diff gates прошли с `jobs=2`; exact 73-method authorization и capability-signal hashes совпали; independent review — `Approved`.
- Decisions: [D-20260715-010](../decisions/decisions.md).
- Problems: открыт [P-20260715-005](../problems/problems.md); 188 methods не считаются capability coverage.
- Boundary: typed `ChatKind`/object-field/option facts, 1010-row policy/artifact, runtime evaluator и risk/prerequisite/retry fields остаются open.
- Next: отдельным TDD/reviewed task добавить per-signal disposition oracle, bounded `ChatKind` atom и exact conditional rights contract для pinned `unpinChatMessage`.

## [2026-07-15] work | W-20260715-014 | P0.5b1 exact ChatKind capability semantics

- Цель: закрыть первую conditional chat-right source family без generic predicate DSL и уменьшить exact fail-closed open set только после complete contract consumption.
- Sources: [ChatKind capability digest](../raw/2026-07-15-tdlib-chat-kind-capability.md), pinned `td_api.tl`, `plans.md`, `D-20260715-010` и два read-only semantic/schema audits.
- Actions: принят `D-20260715-011`; red-green-refactor добавил closed five-value `ResolvedChatKind`, typed `ChatKindCondition`, contradiction rejection, exact four-constructor `ChatType` pin, policy/canonical format `2` и reviewed DNF для шести pinned methods. `unpinChatMessage` получил exact five branches; остальные пять ранних contracts теперь также явно сужены по kind.
- Verification: 41 generator и 21 core tests, whole workspace, Clippy `-D warnings`, fmt/diff, workspace boundary и TDLib pin green с `jobs=2`; exact signal set 193 неизменен, supported 6, open 187. Два independent final reviewers дали `Approved` без findings; `target` 145 MiB, background leftovers `0`.
- Decisions: [D-20260715-011](../decisions/decisions.md).
- Problems: [P-20260715-005](../problems/problems.md) остаётся open, count обновлён 188 → 187.
- Boundary: per-signal disposition artifact, 187 typed dispositions, 1010-row capability policy/artifact, runtime evaluator, risk/prerequisite/retry и live acceptance не реализованы.
- Next: отдельным reviewed TDD task закрепить per-signal disposition representation и следующую closed object-field/`MessageProperties` source family.

## [2026-07-15] work | W-20260715-015 | P0.5b2 exact per-signal disposition oracle

- Цель: запретить method-level partial consumption и закрепить disposition каждого runtime-like documentation signal до следующего typed source-family task.
- Sources: [per-signal disposition digest](../raw/2026-07-15-tdlib-runtime-signal-dispositions.md), pinned `td_api.tl`, `plans.md`, `D-20260715-010` и два independent adversarial reviews.
- Actions: принят `D-20260715-012`; red-green-refactor добавил closed source/family/disposition model, exact 208-source/398-key oracle, explicit consumed-key equality, duplicate/unbound source rejection и exact normalized lexical non-gates/retry classification. Review findings закрыли broad auto-consumption и wording-insensitive `ChatBoost` exception.
- Verification: 48 generator и 21 core tests, whole workspace, Clippy `-D warnings`, fmt/diff, workspace boundary и TDLib pin green с `jobs=2`; signal set 193 неизменен, terminal complete 8, open 185. Два final reviewers дали `Approved`; `target` 145 MiB, background leftovers `0`.
- Decisions: [D-20260715-012](../decisions/decisions.md).
- Problems: [P-20260715-005](../problems/problems.md) остаётся open, count обновлён 187 → 185.
- Boundary: oracle остаётся in-code; 185 typed dispositions, generated 1010-method capability artifact, runtime evaluator, risk/prerequisite/retry и live acceptance не реализованы.
- Next: отдельным reviewed TDD task добавить exact `MessageProperties` vocabulary, typed quantified message predicate и только доказанно complete method contracts.

## [2026-07-15] work | W-20260715-016 | P0.5b3 exact MessageProperties capability semantics

- Цель: закрыть доказанно complete `MessageProperties` source family через typed scalar/universal facts, не поглощая mixed invocation semantics.
- Sources: [MessageProperties capability digest](../raw/2026-07-15-tdlib-message-properties-capabilities.md), pinned schema/source archive, `plans.md`, `D-20260715-012` и независимые inventory/C++/architecture audits.
- Actions: принято `D-20260715-013`; red-green-refactor добавил ordered 39-field schema gate, closed 36-value `MessageCapability`, semantic `One/Each` subjects, canonical format `3`, exact source-aware contracts для 29 methods и public generator E2E. `addOffer` моделируется OR двух mutually exclusive target-state predicates; `reportSupergroupSpam` — supergroup/admin/universal conjunction. Review gaps закрыли field-order validation, name-first missing-source fail-open, duplicate source tags, schema-derived 33/33 partition и scalar/public-generation proof.
- Verification: 52 generator и 22 core tests, 74 whole-workspace tests, Clippy `-D warnings`, fmt/diff, workspace boundary и TDLib pin green с `jobs=2`; 59 consumed и 11 deferred keys имеют exact hashes. Три independent final reviewers дали `Approved`; `target` 146 MiB, background leftovers `0`.
- Decisions: [D-20260715-013](../decisions/decisions.md).
- Problems: [P-20260715-005](../problems/problems.md) остаётся open, exact open set обновлён 185 → 156.
- Boundary: четыре mixed `MessageProperties` methods и ещё 152 иных runtime-signal methods остаются deferred; 1010-method capability artifact, runtime evaluator, risk/prerequisite/retry и live acceptance не реализованы.
- Next: отдельным reviewed TDD task закрывать следующую exact source family без generic predicate DSL и пересчитать open-set oracle.
