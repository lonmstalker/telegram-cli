# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] work | W-20260715-001 | Bootstrap Karpathy Wiki и secret-safe env

- Цель: добавить repo-local Karpathy Wiki, правила ротации, отдельные decision/problem journals и безопасный `.env.local` workflow.
- Sources: инструкция пользователя, `product.md`, `plans.md`, `HARNESS.md`, source digest `../raw/2026-07-15-project-bootstrap.md`, patterns из `tg-analytics` и `my-harness`.
- Actions: инициализирован `.agents/skills/karpathy-wiki`; выбран generic three-journal rotation contract; выполнена secret-safe инвентаризация нужных env names без вывода значений.
- Verification: skill/script/env checks ещё не завершены на этом checkpoint.
- Decisions: [D-20260715-001](../decisions/decisions.md).
- Problems: [P-20260715-001](../problems/problems.md).
- Next: создать `.env.local` атомарным quiet transfer, проверить loader, skill, journals, ссылки, permissions и Git ignore.

## [2026-07-15] work | W-20260715-002 | Wiki и local env bootstrap проверены

- Цель: закрыть bootstrap без раскрытия env values и подтвердить рабочую ротацию трёх журналов.
- Sources: `.agents/skills/karpathy-wiki`, `AGENTS.md`, `.env.example`, `scripts/with-env-local.sh`, `scripts/rotate-wiki-journal.py`.
- Actions: создан `.env.local` с минимальными TDLib development entries; irrelevant bot/admin/database vars и phone не переносились; добавлены protected loader и Git ignore.
- Verification: skill validator passed; loader подтвердил обязательные значения и file/dir references без вывода; mode `0600` и Git ignore подтверждены; work/decision/problem contracts passed; synthetic rotation создала checksum-indexed shard из целой entry.
- Decisions: [D-20260715-001](../decisions/decisions.md).
- Problems: [P-20260715-001](../problems/problems.md) остаётся open до реализации gateway key provider.
- Next: использовать wiki при P0/P1, а `.env.local` — только через protected loader.

## [2026-07-15] work | W-20260715-003 | GitHub remote подключён

- Цель: подключить локальный репозиторий к созданному GitHub remote и опубликовать `main`.
- Sources: GitHub repository `lonmstalker/telegram-cli`, live Git state и явное подтверждение пользователя о допустимости public visibility.
- Actions: проверены clean worktree, ignore/mode/untracked status `.env.local` и отсутствие типовых secret patterns в `HEAD`; `origin` настроен на `https://github.com/lonmstalker/telegram-cli.git`.
- Verification: visibility подтверждена как `PUBLIC`; push и final remote/default-branch checks выполняются после этого checkpoint.
- Decisions: [D-20260715-002](../decisions/decisions.md).
- Problems: none.
- Next: выполнить rotation checks, commit memory checkpoint, push `main` и проверить upstream/remote HEAD.

## [2026-07-15] work | W-20260715-004 | Публикация GitHub remote проверена

- Цель: подтвердить завершение настройки remote и публикации локального состояния.
- Sources: live Git state и GitHub repository metadata.
- Actions: checkpoint commit опубликован в `origin/main`; локальная `main` настроена на tracking `origin/main`.
- Verification: local HEAD и `refs/heads/main` на remote совпали; GitHub default branch — `main`, visibility — `PUBLIC`; journal rotation contracts passed.
- Decisions: [D-20260715-002](../decisions/decisions.md).
- Problems: none.
- Next: использовать `origin/main` как canonical integration branch.

## [2026-07-15] work | W-20260715-005 | P0.1 Cargo workspace boundary закрыт

- Цель: создать минимальный Rust workspace с физическими границами целевой архитектуры, не выдавая skeleton за рабочий runtime.
- Sources: `plans.md`, `product.md`, `HARNESS.md`, `.memory/wiki/project-state.md` и независимый gap-аудит live tree.
- Actions: созданы `telegram-protocol`, `telegram-core`, `telegramd`, `telegram-cli`, optional `telegram-mcp` и `telegram-webapp-runner`; default build исключает deferred MCP; jobs ограничены двумя, debug/incremental payload отключён; незаполненные binaries fail closed.
- Verification: workspace contract и два его negative controls passed; timeout/descendant process-group guards passed; `cargo test --workspace --all-targets --jobs 2`, `cargo clippy --workspace --all-targets --jobs 2 -- -D warnings`, format/diff checks passed; `target` 9.1 MiB; независимый reviewer после трёх циклов исправлений дал `Approved`.
- Decisions: использованы существующие product decisions, новых durable решений нет.
- Problems: [P-20260715-001](../problems/problems.md) не затронута.
- Next: отдельным TDD-коммитом закрепить exact TDLib schema/native provenance; не копировать внешний generated snapshot без повторной provenance-проверки.

## [2026-07-15] work | W-20260715-006 | P0.2a exact TDLib schema pin

- Цель: сохранить exact official TDLib schema snapshot и сделать schema drift проверяемым offline до parser/runtime implementation.
- Sources: [TDLib 1.8.66 schema pin digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md), `plans.md`, `docs/tdlib-api-coverage.md`, F003 harness.
- Actions: принят `D-20260715-003`; добавлены manifest, vendored upstream CMake/schema/license, hard-capped two-phase vendor script и offline checker с provenance/hash/count negative controls.
- Verification: три sequential upstream fetches прошли exact byte/hash/version checks до per-file atomic publish; offline gate подтвердил 2168 definitions, 1010 functions, 184 updates, 13 authorization states; guard test отклонил 10 provenance mutations, oversized sparse file и incomplete preparation; workspace tests/clippy green; independent reviewer после исправлений дал `Approved`.
- Decisions: [D-20260715-003](../decisions/decisions.md).
- Problems: [P-20260715-002](../problems/problems.md); объединённый P0 task checkbox не закрыт.
- Next: закрепить target-specific native build/artifact без тяжёлого или неограниченного resource footprint.

## [2026-07-15] work | W-20260715-007 | P0.2b exact macOS arm64 TDJSON artifact

- Цель: закрепить exact target-specific `tdjson` artifact без глобальной установки, orphan processes и неограниченного build footprint.
- Sources: [native macOS arm64 digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64.md), `vendor/tdlib/native-build-policy.json`, exact schema pin и independent pre-build review.
- Actions: принят `D-20260715-004`; реализованы single-owner bounded builder, safe archive extraction, synthetic detached-HEAD commit proof, content-addressed local cache, closed provenance schema и раздельные provenance/local-artifact check modes.
- Verification: exact build дал artifact SHA-256 `99e7cdb1...fbb6b49`, 27 637 784 bytes; peak RSS 1.74 GiB, peak tree 288.6 MiB, peak processes 12 при `jobs=2`; Mach-O/dependency/export/version/commit/no-DB smoke passed; 8 tar, 7 process-group, lock, commit и 16 provenance negative controls passed; scratch/process cleanup доказан.
- Decisions: [D-20260715-004](../decisions/decisions.md).
- Problems: [P-20260715-002](../problems/problems.md) resolved; [P-20260715-003](../problems/problems.md) open для Linux x86_64.
- Next: независимый final review и commit; затем schema parser/inventory/feature-owner manifest отдельным TDD task.

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
