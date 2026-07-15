# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] work | W-20260715-023 | Exact chat invite-link creation semantics

- Цель: закрыть homogeneous create/replace invite-link prerequisite, не поглощая creator/input-dependent own/other-link branches.
- Sources: [chat invite-link creation digest](../raw/2026-07-15-tdlib-chat-invite-link-creation-capabilities.md), pinned schema/source archive, `plans.md`, `D-20260715-012` и exhaustive triple-signal partition.
- Red: public и pinned tests дали `SchemaDrift`, потому что `RequiresAdministrator`, `RequiresRightPhrase` и named `can_invite_users` key оставались untyped.
- Actions: принят `D-20260715-019`; semantic module pin-ит две signature/source pairs и строит explicit basic-group/supergroup/channel + administrator-right DNF. Missing kind, member-right substitution, source/signature drift и extra argument signal fail closed; девять mixed methods остаются deferred. Engine hash включает module.
- Verification: 50 generator + 23 core = 73 workspace tests, Clippy `-D warnings`, planning/diff gates green с `jobs=2`; exact open set обновлён 133 → 131.
- Decisions/problems: [D-20260715-019](../decisions/decisions.md); [P-20260715-005](../problems/problems.md) остаётся open.
- Boundary: static prerequisite не доказывает current administrator status; runtime evaluator/freshness, own/other invocation predicates, full 1010-method policy/artifact, P1–P10 и live acceptance не реализованы.

## [2026-07-15] review | W-20260715-023 | Independent Rust review accepted

- Scope: invite-link semantic module, generator integration, C++ account/right evidence, exhaustive triple-signal partition и negative controls.
- Result: `APPROVED`, findings отсутствуют. Ревьюер подтвердил exact binding, three-kind administrator DNF, regular-user+bot compatibility, 2 safe/9 mixed split, drift/additional-signal fail-closed, engine hash и отсутствие planning taxonomy/лишних abstractions.
- Verification: reviewer повторил 50 generator и 73 workspace tests, Clippy/fmt, planning/workspace/schema/diff gates с `jobs=2`; `target` 150 MiB, project processes `0`.
- Boundary: verdict не закрывает [P-20260715-005](../problems/problems.md), invocation-dependent invite-link methods, runtime evaluator или live acceptance.

## [2026-07-15] work | W-20260715-024 | Exact supergroup setting-right semantics

- Цель: закрыть безусловные supergroup/channel setting rights, сохранив kind, member/admin и account distinctions и не поглощая boost/input branches.
- Sources: [supergroup setting-right digest](../raw/2026-07-15-tdlib-supergroup-setting-right-capabilities.md), pinned schema/source archive, `plans.md`, `D-20260715-012` и exhaustive family partition.
- Red: public и pinned tests дали `SchemaDrift`, пока role/right signals оставались untyped.
- Actions: принят `D-20260715-020`; semantic module pin-ит пять contracts и использует existing kind/administrator/member atoms. Четыре handlers regular-only, один допускает bot. Wrong kind/role, source/signature drift и extra argument signal fail closed; три mixed methods остаются deferred. Engine hash включает module.
- Verification: 52 generator + 23 core = 75 workspace tests, Clippy `-D warnings`, planning/diff gates green с `jobs=2`; exact open set обновлён 131 → 126.
- Decisions/problems: [D-20260715-020](../decisions/decisions.md); [P-20260715-005](../problems/problems.md) остаётся open.
- Boundary: static prerequisite не доказывает current right; runtime evaluator/freshness, boost/guard input predicates, full 1010-method policy/artifact, P1–P10 и live acceptance не реализованы.

## [2026-07-15] correction | W-20260715-024 | Ordinary-supergroup overclaim removed

- Finding: reviewer P2 показал, что `toggleSupergroupJoinToSendMessages` требует ordinary discussion supergroup; broad kind не исключает gigagroup/monoforum. Открыт и закрыт [P-20260715-007](../problems/problems.md), принята correction к [D-20260715-020](../decisions/decisions.md).
- Action: method удалён из complete table и добавлен в deferred partition; новый kind atom не вводился без runtime evidence.
- Verification: 52 generator tests green; current oracles supported 63, terminal 66, open 127.
- Boundary: future completion method требует closed ordinary-supergroup predicate; исходный raw digest сохранён immutable и superseded correction digest.

## [2026-07-15] review | W-20260715-024 | Post-fix independent Rust review accepted

- Initial finding: P2 broad kind для ordinary-only `toggleSupergroupJoinToSendMessages`; исправлен через deferred state и [P-20260715-007](../problems/problems.md).
- Result: post-fix `APPROVED`, новых findings нет. Подтверждены exact four contracts, kind/role/right/account mappings, deferred boost/guard/ordinary dependencies, drift controls, engine hash и отсутствие planning taxonomy.
- Verification: reviewer повторил 75 workspace tests, Clippy/fmt, planning/workspace/TDLib-pin/diff gates с `jobs=2`.
- Boundary: verdict не закрывает [P-20260715-005](../problems/problems.md), ordinary-supergroup predicate, runtime evaluator или live acceptance.

## [2026-07-15] work | W-20260715-025 | Exact chat setting-right semantics

- Цель: закрыть безусловные `setChat*` kind/right prerequisites без account-conditioned и input/boost/value overclaim.
- Sources: [chat setting-right digest](../raw/2026-07-15-tdlib-chat-setting-right-capabilities.md), pinned schema/C++ archive, `plans.md`, `D-20260715-012` и exhaustive family partition.
- Red/green/refactor: public `setChatTitle` сначала дал `SchemaDrift`; semantic module расширен до chat settings и старый supergroup module поглощён. Reviewer P2 обнаружил bot/basic-group appointed-admin guard для title/photo; открыт/закрыт [P-20260715-008](../problems/problems.md), оба methods возвращены в deferred, public fixture перенесён на description.
- Result: принят [D-20260715-021](../decisions/decisions.md); complete contracts — permissions, description, slow mode. Family разделена на 3 new complete, 1 prior complete и 12 deferred; source/signature/additional-signal и wrong-kind/right controls fail closed.
- Verification: 54 generator + 23 core = 77 workspace tests, Clippy `-D warnings`, fmt, planning/workspace/schema/native/skeleton/diff gates green с `jobs=2`; supported 66, terminal 69, open 124. Post-fix independent review — `APPROVED`, findings отсутствуют.
- Archive link map после ротации: [W-20260715-017 и W-20260715-018](archive/2026-07-15--2026-07-15-016.md).
- Boundary: static prerequisite не доказывает current rights; 124 runtime-signal methods, runtime evaluator/freshness, full 1010-method policy/artifact, P1–P10 и live acceptance остаются open.
- Next: отдельным reviewed TDD task закрывать следующую exact semantic family, сохраняя account/kind-conditioned fail-closed boundary.

## [2026-07-15] correction | W-20260715-026 | addChatMember fail-open contract удалён

- Цель: устранить обнаруженный static false positive до продолжения capability expansion.
- Sources: [addChatMember correction digest](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md), pinned schema/Requests/DialogParticipantManager/ChatManager, [D-20260715-022](../decisions/decisions.md) и [P-20260715-009](../problems/problems.md).
- Red: exact regression получил прежний `Some(RequirementAlternatives)` с broad basic-group/supergroup/channel branches вместо обязательного `SchemaDrift`.
- Green/refactor: удалены единственный `addChatMember` reviewed row и использовавшийся только им `MemberRightInKinds`; method dispositions снова deferred. Contract/oracle tests обновлены без нового generic predicate.
- Verification: 55 generator + 23 core = 78 workspace tests, Clippy `-D warnings`, fmt, planning/workspace/schema/diff gates green с `jobs=2`; supported 65, terminal 68, open 125.
- Review: independent Rust reviewer подтвердил exact archive/root, `CHECK_IS_USER`, monoforum/direct-messages mapping, полные oracle transitions и отсутствие planning taxonomy; verdict `APPROVED`, findings отсутствуют.
- Decisions/problems: принят [D-20260715-022](../decisions/decisions.md); [P-20260715-009](../problems/problems.md) resolved; [P-20260715-005](../problems/problems.md) остаётся open.
- Archive link map после ротации: [W-20260715-019](archive/2026-07-15--2026-07-15-017.md).
- Boundary: correction не реализует subtype atom/evaluator; full 1010-method policy/artifact, P1–P10 и live acceptance остаются open.
- Next: в следующем отдельном task ввести только schema-bound subtype fact, если он одновременно закрывает доказанный exact family без speculative generalization.
