# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] work | W-20260715-020 | P0.5b7 exact runtime boolean option capability semantics

- Цель: закрыть доказанно complete method-level `OptionGate`, не смешивая runtime option с Premium entitlement, argument transformation и mixed owner semantics.
- Sources: [runtime boolean option digest](../raw/2026-07-15-tdlib-runtime-boolean-options.md), pinned schema/source archive, `plans.md`, `D-20260715-012` и независимые Rust/evidence/oracle audits.
- Actions: принято `D-20260715-016`; red-green-refactor добавил exact `OptionValue`/`getOption`/`updateOption` gate, closed three-name vocabulary, account-neutral boolean atom, canonical format `6` и exact regular-user contract для `setNewChatPrivacySettings`. `postStory` и withdrawal оставлены deferred. Reviewer P2 закрыл tautological-only enum test точной ordered equality всех трёх names.
- Verification: 65 generator и 25 core tests, 90 whole-workspace tests, Clippy `-D warnings`, fmt/diff green с `jobs=2`; exact family разделена 1 complete/2 deferred, semantic oracle изменил ровно один row. Rust repeat review, evidence review и independent oracle audit дали `Approved`; `target` 147 MiB, background leftovers `0`.
- Decisions: [D-20260715-016](../decisions/decisions.md).
- Problems: [P-20260715-005](../problems/problems.md) остаётся open, exact open set обновлён 138 → 137.
- Boundary: static option atom не доказывает current value или arbitrary payload validity; runtime store/evaluator, 137 runtime-signal methods, 1010-method artifact, prerequisite/risk/retry и live acceptance не реализованы.
- Next: отдельным reviewed TDD task закрывать следующую exact source family, сохраняя zero-open gate и generation/update completeness boundary.

## [2026-07-15] archive pointer | W-20260715-010 | P0.4a bounded feature-owner generator

- Canonical entry: [immutable work shard](archive/2026-07-15--2026-07-15-009.md). Pointer only; checkpoint не изменён.

## [2026-07-15] archive pointer | W-20260715-011 | P0.4b reviewed 1010-method owner corpus

- Canonical entry: [immutable work shard](archive/2026-07-15--2026-07-15-010.md). Pointer only; checkpoint не изменён.

## [2026-07-15] work | W-20260715-021 | Удалена numeric planning taxonomy из executable architecture

- Цель: исправить ошибочную материализацию `F001`…`F022` как Rust/domain/generated contract и вернуть schema/semantic module boundary.
- Sources: явная correction пользователя, [correction digest](../raw/2026-07-15-planning-taxonomy-removal.md), `D-20260715-017`, `P-20260715-006`, live diff и independent reviewer findings.
- Red: architecture checker обнаружил core/engine/policy/artifact/capability contamination; compile-red закрепил двухаргументный generator API; отдельный scripts fixture доказал discovery false negative, symlink review — обход первого исправления.
- Actions: удалены `FeatureId`, owner engine/app/main/tests и два 1010-row artifacts; capability generator schema+policy-only, format `7`; tooling crate library-only; planning boundary сканирует runtime/tooling/scripts/all root machine files и fail closed на file/root symlink; current docs/wiki corrected, historical raw immutable.
- Verification: bounded `jobs=2`, 69 Rust tests, Clippy `-D warnings`, fmt, workspace/planning/schema/native/skeleton/diff gates green; более 20 000 строк owner-taxonomy implementation удалено, `target` 150 MiB, project processes `0`.
- Review: opaque manifest, discovery scope, file/root symlink bypass и stale documentation findings закрыты; reviewer отдельно воспроизвёл root `build.rs`/symlink-root gates и дал final whole-diff `APPROVED`, findings отсутствуют.
- Decisions/problems: принят `D-20260715-017`; `P-20260715-006` resolved; `P-20260715-005` остаётся open с exact 137 methods.
- Boundary: full registry, runtime evaluator, P1–P10 и live acceptance не реализованы.

## [2026-07-15] work | W-20260715-022 | Exact supergroup username owner semantics

- Цель: закрыть доказанно homogeneous owner prerequisite для управления username супергруппы/канала без generic owner matcher и без planning taxonomy в code.
- Sources: [supergroup username owner digest](../raw/2026-07-15-tdlib-supergroup-username-owner-capabilities.md), pinned schema, `plans.md`, `D-20260715-012` и exhaustive owner-signal partition.
- Red: public `setSupergroupUsername` policy противоречил description; pinned `disableAllSupergroupUsernames` сохранял untyped runtime signal и давал `SchemaDrift`.
- Actions: принят `D-20260715-018`; semantic module pin-ит четыре signature/source contracts и строит explicit `supergroup/channel AND owner` DNF. Bot narrowing, source/signature drift и дополнительный argument signal fail closed; 13 mixed owner methods остаются deferred. Engine hash включает новый module.
- Verification: 48 generator + 23 core = 71 workspace tests, Clippy `-D warnings`, fmt, workspace/planning/schema/native/skeleton/wiki/diff gates green с `jobs=2`; exact open set обновлён 137 → 133.
- Decisions/problems: [D-20260715-018](../decisions/decisions.md); [P-20260715-005](../problems/problems.md) остаётся open.
- Boundary: static prerequisite не доказывает current ownership; runtime evaluator/freshness, full 1010-method policy/artifact, risk/retry, P1–P10 и live acceptance не реализованы.

## [2026-07-15] review | W-20260715-022 | Independent Rust review accepted

- Scope: exact username contract module, generator integration, exhaustive owner-family tests и negative controls; docs/wiki были вне reviewer scope.
- Result: `APPROVED`, findings отсутствуют. Ревьюер независимо подтвердил exact method/signature/source binding, `(supergroup AND owner) OR (channel AND owner)` DNF, 4 complete/13 deferred partition плюс prior complete method, bot rejection, schema/source/additional-signal fail-closed и inclusion module в engine hash.
- Verification: reviewer повторил 48 generator и 71 workspace tests, Clippy/fmt, planning/workspace/schema/native/diff gates с `jobs=2`; `target` 150 MiB, project processes `0`.
- Boundary: verdict относится только к W022 code scope и не закрывает [P-20260715-005](../problems/problems.md), runtime evaluator или live acceptance.
