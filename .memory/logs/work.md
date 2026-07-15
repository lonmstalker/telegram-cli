# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] work | W-20260715-033 | Exact video chat creation contract

- Sources: [creation digest](../raw/2026-07-15-tdlib-video-chat-creation-capability.md), pinned schema/Requests/GroupCallManager и [D-20260715-029](../decisions/decisions.md).
- TDD/result: initial `SchemaDrift`; exact regular-user three-kind `can_manage_video_chats` DNF. Domain module переименован в `video_chats.rs`; request values не стали capability atoms, get/replace semantics сохранены.
- Oracles: supported 70, terminal 73, open 120; format `8`; semantic SHA-256 `d050be73...370e`.
- Verification/review: 64 generator + 24 core = 88 tests, Clippy/fmt/project gates green с `jobs=2`; independent source/Rust review — `APPROVED`, findings absent.
- Boundary: runtime right/read freshness, server value validation, full corpus, P1–P10/live acceptance open; [P-20260715-005](../problems/problems.md) at 120.

## [2026-07-15] archive link map | W-20260715-033 | Rotated W025 decisions

- [W025 shard 027](archive/2026-07-15--2026-07-15-027.md): canonical [D020 correction](../decisions/archive/2026-07-15--2026-07-15-023.md) и [D021](../decisions/archive/2026-07-15--2026-07-15-023.md); D020 historical base остаётся в decision shard 022.

## [2026-07-15] correction | W-20260715-034 | deleteChatMessagesBySender fail-open removed

- Sources: [correction digest](../raw/2026-07-15-tdlib-delete-chat-messages-by-sender-correction.md), pinned schema/Requests/MessageQueryManager/DialogManager/ChatManager, [D-20260715-030](../decisions/decisions.md), [P-20260715-011](../problems/problems.md).
- Red/green: bot-enabled descriptor initially returned `Ok`; flag-removal negative control reproduced broad DNF. Exact regular-user + supergroup + `is_direct_messages_group=false` + `can_delete_messages` contract is green.
- Refactor: row moved from legacy generic table to semantic `message_moderation.rs`; signature/source/additional-signal drift fail closed, engine hash includes module, planning IDs absent.
- Result: supported/terminal/open remain 70/73/120; format `8`, semantic SHA-256 `d050be73...370e`.
- Verification/review: 65 generator + 24 core = 89 workspace tests, Clippy/fmt/project gates green with `jobs=2`; independent source/Rust review — `APPROVED` after subtype correction.
- Boundary: write/right/sender freshness, full corpus, P1–P10/live acceptance remain open.

## [2026-07-15] archive link map | W-20260715-034 | Rotated W026

- [W026 shard 028](archive/2026-07-15--2026-07-15-028.md); canonical D022 и P009 остаются active, historical W019 target — work shard 017.

## [2026-07-15] work | W-20260715-035 | Exact recent-reaction sender moderation

- Sources: [capability digest](../raw/2026-07-15-tdlib-delete-recent-reactions-by-sender-capability.md), pinned schema/Requests/MessageQueryManager и [D-20260715-031](../decisions/decisions.md).
- TDD/result: initial deferred `SchemaDrift`; second `message_moderation.rs` row gives both-account two-kind `can_delete_messages` DNF with empty subtype flags. Regular-only policy and drift fail closed.
- Oracles: supported 71, terminal 74, open 119; format `8`, semantic SHA-256 `44c17d11...d4b1`.
- Verification/review: 66 generator + 24 core = 90 workspace tests with `jobs=2`; independent source/Rust review — `APPROVED`, findings absent.
- Boundary: write/right/sender freshness, full corpus, P1–P10/live acceptance remain open; [P-20260715-005](../problems/problems.md) at 119.

## [2026-07-15] link correction | W-20260715-035 | D022 rotated

- W026/W034 historical D022 links now resolve to [decision shard 024](../decisions/archive/2026-07-15--2026-07-15-024.md).

## [2026-07-15] archive link map | W-20260715-035 | Rotated W027

- [W027 shard 029](archive/2026-07-15--2026-07-15-029.md); canonical D023 and P009 remain active.

## [2026-07-15] work | W-20260715-036 | Exact channel gift-notification access

- Sources: [capability digest](../raw/2026-07-15-tdlib-chat-gift-notification-capability.md), pinned schema/Requests/StarGiftManager/DialogManager/ChatManager and [D-20260715-032](../decisions/decisions.md).
- TDD/result: initial deferred `SchemaDrift`; one existing `chat_settings.rs` row gives exact regular-user + channel + `can_post_messages` DNF. Bot-enabled policy and source/signature/additional-signal drift fail closed.
- Oracles: supported 72, terminal 75, open 118; format `8`, semantic SHA-256 `f6d02581...1742b7f`.
- Verification/review: 67 generator + 24 core = 91 workspace tests with `jobs=2`; independent source/Rust review — `APPROVED`, findings absent.
- Boundary: read/right/chat-status freshness, full corpus, P1–P10/live acceptance remain open; [P-20260715-005](../problems/problems.md) at 118.

## [2026-07-15] archive link map | W-20260715-036 | Rotated D023 and P009 discovery

- [Decision shard 025](../decisions/archive/2026-07-15--2026-07-15-025.md) contains accepted D023; historical active-path links are immutable.
- [Problem shard 018](../problems/archive/2026-07-15--2026-07-15-018.md) contains the P009 discovery entry; its resolved lifecycle remains active in `problems.md`.

## [2026-07-15] link correction | W-20260715-036 | Exact P009 lifecycle split

- P009 discovery is in [problem shard 018](../problems/archive/2026-07-15--2026-07-15-018.md), first resolution in [shard 019](../problems/archive/2026-07-15--2026-07-15-019.md), and the later reviewer-confirmed resolved update remains active in `problems.md`.
- W027 implementation remains in shard 029; its final accepted review is now in [work shard 030](archive/2026-07-15--2026-07-15-030.md).
