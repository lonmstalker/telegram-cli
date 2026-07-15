# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] work | W-20260715-028 | Exact chat event log capability contract

- Цель: закрыть один полностью доказанный method contract без нового generic domain atom или broad lexical inference.
- Sources: [chat event log capability digest](../raw/2026-07-15-tdlib-chat-event-log-capability.md), exact pinned schema, `Requests.cpp`, `DialogEventLog.cpp` и [D-20260715-024](../decisions/decisions.md).
- Red/green/refactor: pinned test сначала получил `SchemaDrift`; exact semantic module добавил regular-user scope, two-branch kind/admin DNF, consumed-signal equality и source/signature/additional-signal/bot negative controls.
- Result: supported 67, terminal 70, open 123; capability format — `8`; semantic module включён в engine hash. Planning/task IDs в executable code отсутствуют.
- Verification: 59 generator + 24 core = 83 workspace tests, Clippy `-D warnings`, fmt, planning/workspace/schema/native/skeleton/diff gates green с `jobs=2`.
- Review: independent reviewer дал `APPROVED`, actionable findings отсутствуют; повторно проверены 59 generator tests и clean diff.
- Problems: [P-20260715-005](../problems/problems.md) open at 123.
- Boundary: static prerequisite не доказывает current administrator status; runtime evaluator/freshness, full 1010-method policy/artifact, P1–P10 и live acceptance не закрыты.

## [2026-07-15] review | W-20260715-028 | Code and evidence reviews accepted

- Code verdict: independent reviewer дал `APPROVED`, findings отсутствуют. Exact signature/source, `CHECK_IS_USER()`, channel-dialog kind, administrator DNF, signal dispositions, account controls, oracle transitions и engine hash подтверждены.
- Evidence verdict: отдельный read-only review дал `APPROVED`; raw digest, [D-20260715-024](../decisions/decisions.md), latest [P-20260715-005](../problems/problems.md), `plans.md`, wiki, archive checksums/counts и canonical semantic links согласованы.
- Final verification: 59 generator + 24 core = 83 tests, Clippy `-D warnings`, fmt, planning/workspace/schema/native/skeleton/process/rotation/diff gates green с `jobs=2`.
- Archive link map после ротации: [W-20260715-021](archive/2026-07-15--2026-07-15-019.md) и historical [P-20260715-005 SupergroupFullInfo update](../problems/archive/2026-07-15--2026-07-15-007.md).
- Boundary: verdicts относятся только к static W028 slice и не закрывают 123-method open set, runtime freshness, full corpus, P1–P10 или live acceptance.

## [2026-07-15] correction | W-20260715-029 | unpinChatMessage false-positive coverage removed

- Цель: устранить incomplete description-only contract до следующего capability expansion.
- Sources: [unpinChatMessage correction digest](../raw/2026-07-15-tdlib-unpin-chat-message-overclaim-correction.md), pinned schema/Requests/MessagesManager/DialogManager, [D-20260715-025](../decisions/decisions.md) и [P-20260715-010](../problems/problems.md).
- Red: pinned regression ожидал deferred `SchemaDrift`, но получил old five-branch DNF с secret-chat alternative.
- Green/refactor: real row и positive pinned assertion удалены; generic conditional-right fixture переименован в `requireSyntheticConditionalPinRight`; real recognizer regression требует `SchemaDrift`.
- Result: supported 66, terminal 69, open 124; format — `8`; semantic disposition SHA-256 `15f4aba2...53fa2`. Planning/task IDs в executable code отсутствуют.
- Verification: 60 generator + 24 core = 84 workspace tests, Clippy `-D warnings`, fmt, planning/workspace/schema/native/skeleton/process/diff gates green с `jobs=2`.
- Review: independent reviewer дал `APPROVED`, findings отсутствуют; source branches, synthetic/pinned boundary, exact oracles и no-planning-ID gate подтверждены.
- Problems: [P-20260715-010](../problems/problems.md) resolved как removed overclaim; [P-20260715-005](../problems/problems.md) open at 124.
- Boundary: correction не реализует missing account/subtype/message evaluator; full corpus, P1–P10 и live acceptance не закрыты.

## [2026-07-15] review | W-20260715-029 | Evidence accepted

- Verdict: separate docs/wiki review — `APPROVED`; D025 canonical link и immutable-shard P005 link map исправлены, findings закрыты.
- Verification: 84/84 tests, exact 66/69/124 oracles, shard SHA/counts, rotation и diff green; active journals ниже limits.
- Boundary: verdict не закрывает runtime/full-corpus/live acceptance.

## [2026-07-15] work | W-20260715-030 | Exact chat invite-link counts capability

- Цель: закрыть count method в existing invite-link domain без duplicate module или generic role DSL.
- Sources: [invite-link counts digest](../raw/2026-07-15-tdlib-chat-invite-link-counts-capability.md), pinned schema/Requests/DialogInviteLinkManager и [D-20260715-026](../decisions/decisions.md).
- Red/green/refactor: pinned test сначала получил `SchemaDrift`; local `RequiredAccess::Owner` добавил account scope, three-kind owner DNF и exact consumed keys. Duplicate account bool и hardcoded right удалены.
- Result: owner family — 4 local safe, 2 complete elsewhere, 12 deferred; supported 67, terminal 70, open 123; format `8`.
- Verification: 61 generator + 24 core = 85 workspace tests, Clippy `-D warnings`, fmt, planning/diff green с `jobs=2`.
- Review: independent source/code reviewer — `APPROVED`, findings отсутствуют; exact source, partition/oracles и no-overengineering boundary подтверждены.
- Problems: [P-20260715-005](../problems/problems.md) open at 123.
- Boundary: runtime owner/write/active-chat freshness, full corpus, P1–P10 и live acceptance не закрыты.

## [2026-07-15] review | W-20260715-030 | Evidence accepted

- Verdict: docs/wiki review — `APPROVED`; current owner partition исправлен на 4+2+12, findings закрыты.
- Verification: 85/85 tests, 67/70/123 oracles, four hashes, shard SHA/counts, rotation и diff green.
- Boundary: runtime/full-corpus/live acceptance остаются open.

## [2026-07-15] archive link map | W-20260715-030 | Rotated owner decision

- Immutable [W022 work shard](archive/2026-07-15--2026-07-15-020.md) ссылается на [canonical D018 decision shard](../decisions/archive/2026-07-15--2026-07-15-020.md); historical link в shard не переписывается.
- Status W030 и owner-family boundary не изменены.

## [2026-07-15] review delta | W-20260715-030 | Rotation maps accepted

- Final reviewer verdict — `APPROVED`: decision/work/problem maps для shards 009–011 и 019–023, checksums/counts и canonical targets exact.
- Rotation/diff green; W030 semantics и 123-method boundary не изменены.

## [2026-07-15] work | W-20260715-031 | Exact video chat RTMP access capability

- Цель: закрыть `getVideoChatRtmpUrl` как chat-bound streaming access, не смешивая его с `group_call_id` property contracts и не вводя planning IDs в executable code.
- Sources: [RTMP access digest](../raw/2026-07-15-tdlib-video-chat-rtmp-access-capability.md), pinned schema/Requests/GroupCallManager и [D-20260715-027](../decisions/decisions.md).
- Red/green/refactor: pinned test сначала получил `SchemaDrift`; exact semantic contract добавил regular-user scope, three-kind `can_manage_video_chats` DNF и exact two-key signal consumption. Ложный `NamedRight` scanner expectation удалён: конкретное право закреплено exact source text и typed contract.
- Result: supported 68, terminal 71, open 122; format `8`; no owner/active-call/RTMP-state gate invented.
- Verification: 62 generator + 24 core = 86 workspace tests, Clippy `-D warnings`, fmt/project gates green с `jobs=2`; target 151 MiB, project processes 0.
- Review: independent source/Rust reviewer — `APPROVED`, findings отсутствуют; dispatcher, deeper handler, oracles и fail-closed drift подтверждены.
- Problems: [P-20260715-005](../problems/problems.md) open at 122.
- Boundary: runtime dialog/right freshness, full corpus, P1–P10 и live acceptance не закрыты.

## [2026-07-15] archive link map | W-20260715-031 | Rotated invite-link decision

- Immutable W023 shard 022: canonical [D-20260715-019](../decisions/archive/2026-07-15--2026-07-15-021.md); historical link не переписывается.

## [2026-07-15] link | W-20260715-031 | D020

- Canonical [D020](../decisions/archive/2026-07-15--2026-07-15-022.md) for legacy active/shard-024 links.

## [2026-07-15] link correction | W-20260715-031 | D020 split

- Legacy links resolve [base](../decisions/archive/2026-07-15--2026-07-15-022.md) + [accepted correction](../decisions/decisions.md); correction current.

## [2026-07-15] work | W-20260715-032 | Exact RTMP replacement owner contract

- Sources: [replacement digest](../raw/2026-07-15-tdlib-video-chat-rtmp-replacement-capability.md), pinned schema/Requests/GroupCallManager, [D-20260715-028](../decisions/decisions.md).
- TDD/result: initial `SchemaDrift`; streaming `RequiredAccess::Owner` дал regular-user three-kind owner DNF. Shared admin precheck не ослабляет public revoke contract; call-state atoms не добавлены.
- Oracles: supported 69, terminal 72, open 121; format `8`.
- Verification/review: 63 generator + 24 core = 87 tests, Clippy/fmt/planning/diff green, independent `APPROVED`.
- Boundary: runtime freshness/server revoke, full corpus, P1–P10/live acceptance open; [P-20260715-005](../problems/problems.md) at 121.

## [2026-07-15] archive link map | W-20260715-032 | W024/P007 split

- W024: [base](archive/2026-07-15--2026-07-15-024.md), [correction](archive/2026-07-15--2026-07-15-025.md), [accepted review](archive/2026-07-15--2026-07-15-026.md).
- P007: [open](../problems/archive/2026-07-15--2026-07-15-013.md), [resolved](../problems/archive/2026-07-15--2026-07-15-014.md); resolved current.
