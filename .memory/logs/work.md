# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] work | W-20260715-027 | Exact supergroup subtype capability contracts

- Цель: закрыть только те methods, для которых pinned schema и deeper handlers дают полный account/kind/subtype/right contract.
- Sources: [supergroup subtype capability digest](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md), exact `supergroup`/`updateSupergroup`, Requests/DialogParticipantManager/ChatManager и [D-20260715-023](../decisions/decisions.md).
- Red/green: отсутствующий domain atom сначала дал compile failure; pinned invite/ordinary-setting tests сначала получили `SchemaDrift`; public serialization test затем обнаружил отсутствующий fixture type. Closed `SupergroupFlagCondition`, exact semantic rows и negative drift/policy controls сделали все tests green.
- Review/fix: reviewer P2 сначала доказал self/non-self partition в singular method, затем size-one basic-group delegation plural method в тот же flow. Весь invite semantic module удалён; оба methods сохранены deferred. Reviewer подтвердил toggle как exact для текущей static-capability boundary.
- Result: `toggleSupergroupJoinToSendMessages` имеет exact regular-user, kind, Boolean subtype и right requirements. Planning IDs отсутствуют в executable code; capability format `8`.
- Verification: 58 generator + 24 core = 82 workspace tests, Clippy `-D warnings` и fmt green с `jobs=2`; supported 66, terminal 69, open 124.
- Problems: [P-20260715-009](../problems/problems.md) остаётся resolved через deferred invite boundary, [P-20260715-005](../problems/problems.md) open at 124.
- Boundary: runtime evaluator/freshness, full 1010-method policy/artifact, P1–P10 и live acceptance не закрыты.

## [2026-07-15] review | W-20260715-027 | Final subtype code review accepted

- Findings fixed: singular self/non-self и plural size-one basic-group delegation сделали оба invite contracts не-exact; весь invite semantic module удалён, regression требует `SchemaDrift` для обоих methods.
- Verdict: final independent Rust review — `APPROVED`, actionable findings отсутствуют. Подтверждены exact toggle handler mapping, closed schema vocabulary, canonical serialization, contradiction checks и отсутствие executable plan/feature IDs.
- Final verification: 58 generator + 24 core tests, Clippy `-D warnings`, fmt, planning/workspace/schema/native/skeleton/process/rotation/diff gates green с `jobs=2`; `target` 151 MiB, project process leftovers 0.
- Archive link map после ротации: [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md) и [D-20260715-016](../decisions/archive/2026-07-15--2026-07-15-018.md).
- Boundary: approval относится к static schema/capability slice и не закрывает runtime freshness, 124-method open set, full corpus, P1–P10 или live acceptance.
