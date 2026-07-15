# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] work | W-20260715-017 | P0.5b4 exact chat-boost link lexical disposition

- Цель: убрать один доказанный lexical false positive без ослабления fail-closed recognizer.
- Evidence: [exact digest](../raw/2026-07-15-tdlib-chat-boost-link-non-gate.md), pinned schema/Requests/BoostManager/LinkManager и `D-20260715-012`.
- Result: exact unique description переводит только `getChatBoostLinkInfo` в non-gate; same-family drift остаётся deferred. Terminal complete 38, open set 155.
- Verification: 74 workspace tests, Clippy/fmt/diff green с `jobs=2`; два independent reviews — `Approved`.
- Boundary: runtime/capability surface не расширена; [P-20260715-005](../problems/problems.md) остаётся open.

## [2026-07-15] work | W-20260715-018 | P0.5b5 exact GroupCall capability semantics

- Цель: закрыть доказанно complete `GroupCall`/`GroupCallMessage` source family через typed kind/property/cardinality, не смешивая static capability с lifecycle readiness и argument-dependent branches.
- Sources: [GroupCall capability digest](../raw/2026-07-15-tdlib-group-call-capabilities.md), pinned schema/source archive, `plans.md`, `D-20260715-012` и независимые schema/C++/architecture audits.
- Actions: принято `D-20260715-014`; red-green-refactor добавил exact 32/7-field schema gates, closed kind/property vocabularies, semantic `group_call_id:int32`, universal `Each(message_ids:vector<int32>)`, contradiction/account guards, canonical format `4` и 12 exact reviewed DNF. Два argument-dependent methods оставлены deferred. Adversarial review исправил ложное consumption `OnlyByAdministrator`: exact wording теперь отдельный setting-semantics non-gate.
- Verification: 57 generator и 23 core tests, 80 whole-workspace tests, Clippy `-D warnings`, fmt/diff, workspace boundary, TDLib pin и wiki integrity green с `jobs=2`; exact family разделена на 12 complete/2 deferred, 38 consumed/6 deferred keys. Три independent final reviewers дали `Approved`; `target` не превышает 147 MiB, background leftovers `0`.
- Decisions: [D-20260715-014](../decisions/decisions.md).
- Problems: [P-20260715-005](../problems/problems.md) остаётся open, exact open set обновлён 155 → 143.
- Boundary: static atoms не доказывают `is_active`/`is_joined`, freshness или invocation inputs; future message evidence обязано fail closed на stale/unknown. 1010-method artifact, runtime evaluator, risk/prerequisite/retry и live acceptance не реализованы.
- Next: отдельным reviewed TDD task закрывать следующую exact source family, сохраняя zero-open gate и freshness boundary.

## [2026-07-15] work | W-20260715-019 | P0.5b6 exact SupergroupFullInfo capability semantics

- Цель: закрыть доказанно complete subset `SupergroupFullInfo` source family через typed property/target facts, не смешивая static capability с stale runtime snapshot и mixed invocation semantics.
- Sources: [SupergroupFullInfo capability digest](../raw/2026-07-15-tdlib-supergroup-full-info-capabilities.md), pinned schema/source archive, `plans.md`, `D-20260715-012` и независимые Rust/evidence/oracle audits.
- Actions: принято `D-20260715-015`; red-green-refactor добавил exact 42-field schema gate, closed eight-property vocabulary, semantic `chat_id`/`supergroup_id:int53`, canonical format `5` и пять exact reviewed DNF. Семь owner/filter/value/password-dependent methods оставлены deferred. Два cross-token `OnlyIfAdministrator` false positives получили exact lexical non-gate; evidence review исправил misleading freshness completion wording.
- Verification: 61 generator и 24 core tests, 85 whole-workspace tests, Clippy `-D warnings`, fmt/diff green с `jobs=2`; exact family разделена на 5 complete/7 deferred, 12 consumed/18 deferred keys. Rust и evidence reviews дали `Approved`; independent scanner воспроизвёл global hashes. `target` 147 MiB, background leftovers `0`.
- Decisions: [D-20260715-015](../decisions/decisions.md).
- Problems: [P-20260715-005](../problems/problems.md) остаётся open, exact open set обновлён 143 → 138.
- Boundary: static atom не доказывает full-info freshness, `chat_id` resolution или current permission; seven family methods, 1010-method artifact, runtime evaluator, risk/prerequisite/retry и live acceptance не реализованы.
- Next: отдельным reviewed TDD task закрывать следующую exact source family, сохраняя zero-open gate и stale/unknown fail-closed boundary.
