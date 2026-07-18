# Telegram CLI Wiki

Начинай долговечную работу с этой страницы и открывай только нужные ссылки.

## Canonical project sources

- [Product boundary](../../product.md)
- [Living plan](../../plans.md) — фазы, правила работы, зоны ответственности, acceptance
- [Feature inventory](../../HARNESS.md)
- [User guide: RU](../../docs/user-guide.ru.md) / [EN](../../docs/user-guide.en.md)
- [Authorization guide: RU](../../docs/authorization-guide.ru.md) / [EN](../../docs/authorization-guide.en.md)
- [TDLib coverage contract](../../docs/tdlib-api-coverage.md)
- [Reviewed capability contracts](../../docs/capability-notes.md)
- [`tg-analytics` reuse boundary](../../docs/tg-analytics-reuse.md)
- [TDJSON transport contract](../../docs/tdjson-transport.md)
- [Authorization state machine contract](../../docs/authorization-state-machine.md)
- [Database encryption key contract](../../docs/database-encryption-key.md)
- [Ordered state reducer contract](../../docs/ordered-state-reducer.md)
- [Core runtime startup contract](../../docs/core-runtime-startup.md)
- [Daemon profile ownership contract](../../docs/daemon-profile-ownership.md)
- [Daemon profile socket/election contract](../../docs/daemon-profile-socket.md)
- [Daemon lease contract](../../docs/daemon-leases.md)
- [Per-account scheduler contract](../../docs/daemon-scheduler.md)
- [Daemon shared-session lifecycle contract](../../docs/daemon-session-lifecycle.md)
- [Generated TDLib registry contract](../../docs/tdlib-generated-registry.md)
- [Core generated raw API contract](../../docs/core-raw-api.md)
- [Operational telemetry and audit contract](../../docs/telemetry-audit.md)
- [CLI session contract](../../docs/cli-session.md)
- [CLI schema/raw call contract](../../docs/cli-schema-call.md)
- [CLI workflow routes contract](../../docs/cli-workflows.md)
- [CLI secure login contract](../../docs/cli-secure-login.md)
- [P6 cold-agent eval](../../docs/agent-skill-eval.md)
- [MCP stdio/SSH transport contract](../../docs/mcp-transport.md)
- [F007 user/profile workflow](../../docs/user-profile-workflow.md)
- [F008 chat/folder/topic workflows](../../docs/forum-topic-workflow.md)
- [F009 message workflows](../../docs/message-workflow.md)
- [F010 file/media workflows](../../docs/file-transfer-workflow.md)
- [F011 groups/channels/moderation workflows](../../docs/chat-administration-workflow.md)
- [F012 bot testing workflows](../../docs/bot-testing-workflow.md)
- [F013 Mini App handoff](../../docs/mini-app-handoff.md)
- [F014 custom emoji set workflow](../../docs/sticker-set-workflow.md)
- [F015 story and group-call workflow](../../docs/story-call-workflow.md)
- [F016 account settings workflow](../../docs/account-settings-workflow.md)
- [F017 Business workflow](../../docs/business-workflow.md)
- [F018 Stars payment workflow](../../docs/stars-payment-workflow.md)
- [F020 platform utilities workflow](../../docs/platform-utilities-workflow.md)
- [Current project state](project-state.md)

## Memory streams

- [Active work journal](../logs/work.md)
- [Active decision journal](../decisions/decisions.md)
- [Active problem journal](../problems/problems.md)

## Raw evidence

- [Bootstrap source digest](../raw/2026-07-15-project-bootstrap.md)
- [TDLib 1.8.66 schema pin digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md)
- [TDLib macOS arm64 reviewed rebuild](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — current artifact truth
- [TDLib Linux x86_64 native build](../raw/2026-07-15-tdlib-1.8.66-native-linux-x86_64.md) — current artifact truth
- [Schema parser/inventory digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md)
- [`tg-analytics` reuse audit](../raw/2026-07-15-tg-analytics-reuse-audit.md) — exact source snapshot и test evidence
- [TDJSON transport native smoke](../raw/2026-07-15-tdjson-transport-native-smoke.md) — real correlated request
- [TDLib database-key codec](../raw/2026-07-15-tdlib-database-key-codec.md) — Base64, empty-key и wrong-key upstream semantics
- [P1 runtime acceptance](../raw/2026-07-15-p1-runtime-acceptance.md) — native handshake, wrong/missing-key, secret canary и returning live session
- [P2 daemon lifecycle acceptance](../raw/2026-07-15-p2-daemon-lifecycle-acceptance.md) — concurrency, TTL, crash recovery и graceful idle restart
- [P3 Rust bindings evaluation](../raw/2026-07-15-p3-rust-bindings-evaluation.md) — почему exact generated registry не заменён version-mismatched crate
- [P10 authorization returning check](../raw/2026-07-17-p10-authorization-returning-check.md) — свежий daemon/CLI `Ready + getMe`, lease release и graceful `Closed`; first login остаётся pending
- [`tg-analytics` TDLib authorization preflight](../raw/2026-07-17-tg-analytics-tdlib-auth-preflight.md) — source prod session fail closed на wrong database key; номер не раскрывался
- [P10 isolated first-login challenge](../raw/2026-07-18-p10-first-login-challenge.md) — новый profile дошёл до sanitized `phone_number/challenge_id=1`; owner TTY input и terminal proof pending
- [P10 owner TTY failure](../raw/2026-07-18-p10-owner-tty-failure.md) — input не был submitted; local `/dev/tty` reader исправлен и ждёт live owner retry
- [P10 owner TTY retry](../raw/2026-07-18-p10-owner-tty-retry.md) — corrected prompt ждал скрытый ввод до owner cancellation; immediate failure resolved, phone submission pending
- [P10 first-login and returning acceptance](../raw/2026-07-18-p10-first-login-returning-acceptance.md) — owner TTY first login, `Ready + getMe`, graceful `Closed` и returning `Ready` без повторного secret input
- [P9 reproducible native builds](../raw/2026-07-18-p9-reproducible-native-builds.md) — два independent exact-recipe build для macOS arm64 и Linux x86_64 совпали bit-for-bit; свежий provenance gate и guard suite green

## Current records

- Implementation: P0–P8 accepted; первый Tasks-пункт P9 про reproducible pinned TDLib builds закрыт, P9 продолжается с launchd/systemd packaging; см. [project-state.md](project-state.md) и [W-20260718-011](../logs/work.md).
- P10 authorization slice принят: 2026-07-18 отдельный profile прошёл owner TTY first login, daemon `Ready + getMe`, graceful `Closed` и returning `Ready` без повторного phone/OTP; общая P10 остаётся pending по другим scenarios; см. [W-20260718-008](../logs/work.md).
- Authorization review hardening: timeout после dispatch остаётся uncertain без blind replay;
  challenge token boot-scoped; re-auth Ready повторяет `getMe`/identity proof и отзывает leases;
  QR/ToS/privacy/email resend идут через owner-only prompt. Protocol machine envelope — v4;
  см. [D-20260718-005](../decisions/decisions.md), [P-20260718-003](../problems/problems.md),
  [W-20260718-009](../logs/work.md).
- Authorization architecture consolidated: один coordinator владеет startup, broker state,
  uncertain outcome и verified account; lifecycle видит только closed observation. CLI login loop
  отделён от socket/TTY/runtime adapters и покрыт deterministic multi-step tests. Mixed
  orchestration разделена по transition/outcome; большие auth unit tests лежат в sibling
  `tests.rs`, а exhaustive state mappings остаются цельными; см.
  [D-20260718-006](../decisions/decisions.md), [D-20260718-008](../decisions/decisions.md),
  [W-20260718-010](../logs/work.md), [W-20260718-012](../logs/work.md).
- Owner TTY echo boundary: phone/OTP/email/registration видны; cloud password спрашивает `[y/N]` и hidden только по opt-in, никакого args/stdin/env/machine-output path не добавлено; [D-20260718-003](../decisions/decisions.md) supersedes [D-20260718-001](../decisions/decisions.md).
- Primary human auth UX: один `telegram-cli login` сам проходит всю fresh challenge chain; JSON/JSONL остаётся status-only, exact-token handoff — только MCP/operator compatibility; см. [D-20260718-002](../decisions/decisions.md), [D-20260718-005](../decisions/decisions.md).
- Code resend: TDLib `timeout` считается eligibility delay, не guessed OTP TTL; при `next_type` human flow сам запрашивает fresh code и ждёт новый challenge. First login принят, но actual expired-code resend остаётся узким live follow-up [P-20260718-002](../problems/problems.md); см. [D-20260718-004](../decisions/decisions.md), [W-20260718-007](../logs/work.md).
- Двуязычные пользовательские инструкции добавлены для source checkout и brokered authorization; acceptance-границы P9/P10 в них названы явно; см. [W-20260718-001](../logs/work.md).
- Открытые проблемы: external source-session blocker [P-20260717-001](../problems/problems.md); live code resend [P-20260718-002](../problems/problems.md); owner TTY failure закрыт в [P-20260718-001](../problems/problems.md); local gateway key wiring закрыт в [P-20260715-001](../problems/problems.md), Linux artifact — в [P-20260715-003](../problems/problems.md).
- Консолидация журналов и удаление capability-движка: [D-20260715-035](../decisions/decisions.md), [W-20260715-039](../logs/work.md).
- Linux x86_64 native artifact: [W-20260715-040](../logs/work.md), [P-20260715-003](../problems/problems.md).
- Reuse/account model: [D-20260715-036](../decisions/decisions.md), [W-20260715-041](../logs/work.md), [`docs/tg-analytics-reuse.md`](../../docs/tg-analytics-reuse.md).
- TDJSON transport: [D-20260715-037](../decisions/decisions.md), [W-20260715-042](../logs/work.md), [`docs/tdjson-transport.md`](../../docs/tdjson-transport.md).
- Authorization machine: [D-20260715-038](../decisions/decisions.md), [W-20260715-043](../logs/work.md), [`docs/authorization-state-machine.md`](../../docs/authorization-state-machine.md).
- Database key provider: [D-20260715-039](../decisions/decisions.md), [W-20260715-044](../logs/work.md), [`docs/database-encryption-key.md`](../../docs/database-encryption-key.md).
- Ordered reducer: [D-20260715-040](../decisions/decisions.md), [W-20260715-045](../logs/work.md), [`docs/ordered-state-reducer.md`](../../docs/ordered-state-reducer.md).
- Lossless unknown updates: [D-20260715-041](../decisions/decisions.md), [W-20260715-046](../logs/work.md), [`docs/ordered-state-reducer.md`](../../docs/ordered-state-reducer.md).
- Bounded startup runtime и P1 acceptance: [D-20260715-042](../decisions/decisions.md), [W-20260715-047](../logs/work.md), [`docs/core-runtime-startup.md`](../../docs/core-runtime-startup.md).
- Canonical DB owner lock: [D-20260715-043](../decisions/decisions.md), [W-20260715-048](../logs/work.md), [`docs/daemon-profile-ownership.md`](../../docs/daemon-profile-ownership.md).
- Private profile socket/election: [D-20260715-044](../decisions/decisions.md), [W-20260715-049](../logs/work.md), [`docs/daemon-profile-socket.md`](../../docs/daemon-profile-socket.md).
- Lease protocol: [D-20260715-045](../decisions/decisions.md), [W-20260715-050](../logs/work.md), [`docs/daemon-leases.md`](../../docs/daemon-leases.md).
- Per-account scheduler: [D-20260715-046](../decisions/decisions.md), [W-20260715-051](../logs/work.md), [`docs/daemon-scheduler.md`](../../docs/daemon-scheduler.md).
- Shared-session lifecycle и P2 acceptance: [D-20260715-047](../decisions/decisions.md), [W-20260715-052](../logs/work.md), [`docs/daemon-session-lifecycle.md`](../../docs/daemon-session-lifecycle.md).
- Exact generated registry: [D-20260715-048](../decisions/decisions.md), [W-20260715-053](../logs/work.md), [`docs/tdlib-generated-registry.md`](../../docs/tdlib-generated-registry.md).
- Capability table/default-deny: [D-20260715-049](../decisions/decisions.md), [W-20260715-054](../logs/work.md), [`docs/capability-notes.md`](../../docs/capability-notes.md).
- Core discovery/call: [D-20260715-050](../decisions/decisions.md), [W-20260715-055](../logs/work.md), [`docs/core-raw-api.md`](../../docs/core-raw-api.md).
- Raw policy-before-send: [D-20260715-051](../decisions/decisions.md), [W-20260715-056](../logs/work.md), [`docs/core-raw-api.md`](../../docs/core-raw-api.md).
- Generated coverage report и P3 Acceptance: [D-20260715-052](../decisions/decisions.md), [W-20260715-057](../logs/work.md), [`docs/tdlib-api-coverage.md`](../../docs/tdlib-api-coverage.md).
- Chat resolve/membership boundary: [D-20260715-053](../decisions/decisions.md), [W-20260715-058](../logs/work.md), [`docs/chat-resolution-membership.md`](../../docs/chat-resolution-membership.md).
- Chat-list loading: [D-20260715-054](../decisions/decisions.md), [W-20260715-059](../logs/work.md), [`docs/chat-list-loading.md`](../../docs/chat-list-loading.md).
- Chat inspection/open lease: [D-20260715-055](../decisions/decisions.md), [W-20260715-060](../logs/work.md), [`docs/chat-inspection-workflow.md`](../../docs/chat-inspection-workflow.md).
- Message pagination: [D-20260715-056](../decisions/decisions.md), [W-20260715-061](../logs/work.md), [`docs/message-pagination.md`](../../docs/message-pagination.md).
- Members/statistics workflows: [D-20260715-057](../decisions/decisions.md), [W-20260715-062](../logs/work.md), [`docs/members-statistics-workflow.md`](../../docs/members-statistics-workflow.md).
- File/sticker/bot/Web App terminal workflows: [D-20260715-058](../decisions/decisions.md), [W-20260715-063](../logs/work.md), [`docs/terminal-domain-workflows.md`](../../docs/terminal-domain-workflows.md).
- Update gap/resync и P4 Acceptance: [D-20260715-059](../decisions/decisions.md), [W-20260715-064](../logs/work.md), [`docs/update-gap-resync.md`](../../docs/update-gap-resync.md).
- Scheduler budgets/flood backoff: [D-20260715-060](../decisions/decisions.md), [W-20260715-065](../logs/work.md), [`docs/daemon-scheduler.md`](../../docs/daemon-scheduler.md).
- Safe-read/convergent retry: [D-20260715-061](../decisions/decisions.md), [W-20260715-066](../logs/work.md), [`retry.rs`](../../crates/telegram-core/src/retry.rs).
- Durable idempotency journal: [D-20260715-062](../decisions/decisions.md), [W-20260715-067](../logs/work.md), [`docs/idempotency-journal.md`](../../docs/idempotency-journal.md).
- Typed risk scopes: [D-20260715-063](../decisions/decisions.md), [W-20260715-068](../logs/work.md), [`docs/daemon-leases.md`](../../docs/daemon-leases.md).
- External exact-plan approval: [D-20260715-064](../decisions/decisions.md), [W-20260715-069](../logs/work.md), [`docs/external-plan-approval.md`](../../docs/external-plan-approval.md).
- Metrics/redacted audit и P5 Acceptance: [D-20260715-065](../decisions/decisions.md), [W-20260715-070](../logs/work.md), [`docs/telemetry-audit.md`](../../docs/telemetry-audit.md).
- CLI session client: [D-20260715-066](../decisions/decisions.md), [W-20260715-071](../logs/work.md), [`docs/cli-session.md`](../../docs/cli-session.md).
- CLI schema/raw call: [D-20260715-067](../decisions/decisions.md), [W-20260715-072](../logs/work.md), [`docs/cli-schema-call.md`](../../docs/cli-schema-call.md).
- CLI workflow routes: [D-20260715-068](../decisions/decisions.md), [W-20260715-073](../logs/work.md), [`docs/cli-workflows.md`](../../docs/cli-workflows.md).
- CLI login/events routes: [D-20260715-069](../decisions/decisions.md), [W-20260715-074](../logs/work.md), [`docs/cli-login-events.md`](../../docs/cli-login-events.md).
- CLI output/exit contract: [D-20260715-070](../decisions/decisions.md), [W-20260715-075](../logs/work.md), [`docs/cli-output-contract.md`](../../docs/cli-output-contract.md).
- CLI streaming/cancellation: [D-20260715-071](../decisions/decisions.md), [W-20260715-076](../logs/work.md), [`docs/cli-streaming.md`](../../docs/cli-streaming.md).
- CLI secure login: [D-20260715-072](../decisions/decisions.md), [W-20260715-077](../logs/work.md), [`docs/cli-secure-login.md`](../../docs/cli-secure-login.md).
- Compact agent skill и P6 Acceptance: [D-20260715-073](../decisions/decisions.md), [W-20260715-078](../logs/work.md), [`docs/agent-skill-eval.md`](../../docs/agent-skill-eval.md).
- F007 user/profile resolver/redaction/mutation: [D-20260715-074](../decisions/decisions.md), [W-20260715-079](../logs/work.md), [`docs/user-profile-workflow.md`](../../docs/user-profile-workflow.md).
- F008 chat/folder/topic pagination/reconciliation: [D-20260715-075](../decisions/decisions.md), [W-20260715-080](../logs/work.md), [`docs/forum-topic-workflow.md`](../../docs/forum-topic-workflow.md).
- F009 message read/send safety: [D-20260715-076](../decisions/decisions.md), [W-20260715-081](../logs/work.md), [`docs/message-workflow.md`](../../docs/message-workflow.md).
- F010 file terminal/path safety: [D-20260715-077](../decisions/decisions.md), [W-20260715-082](../logs/work.md), [`docs/file-transfer-workflow.md`](../../docs/file-transfer-workflow.md).
- F011 exact-plan chat administration: [D-20260715-078](../decisions/decisions.md), [W-20260715-083](../logs/work.md), [`docs/chat-administration-workflow.md`](../../docs/chat-administration-workflow.md).
- F012 ordered bot reply/callback test: [D-20260715-079](../decisions/decisions.md), [W-20260715-084](../logs/work.md), [`docs/bot-testing-workflow.md`](../../docs/bot-testing-workflow.md).
- F013 one-shot Mini App browser handoff: [D-20260715-080](../decisions/decisions.md), [W-20260715-085](../logs/work.md), [`docs/mini-app-handoff.md`](../../docs/mini-app-handoff.md).
- F014 typed custom emoji lifecycle: [D-20260715-081](../decisions/decisions.md), [W-20260715-086](../logs/work.md), [`docs/sticker-set-workflow.md`](../../docs/sticker-set-workflow.md).
- F015 story/group-call proof and cleanup: [D-20260715-082](../decisions/decisions.md), [W-20260715-087](../logs/work.md), [`docs/story-call-workflow.md`](../../docs/story-call-workflow.md).
- F016 partial settings and exact session termination: [D-20260715-083](../decisions/decisions.md), [W-20260715-088](../logs/work.md), [`docs/account-settings-workflow.md`](../../docs/account-settings-workflow.md).
- F017 explicit Business connection and runtime bot policy: [D-20260715-084](../decisions/decisions.md), [W-20260715-089](../logs/work.md), [`docs/business-workflow.md`](../../docs/business-workflow.md).
- F018 Stars-only exact payment and ledger proof: [D-20260715-085](../decisions/decisions.md), [W-20260715-090](../logs/work.md), [`docs/stars-payment-workflow.md`](../../docs/stars-payment-workflow.md).
- F019 terminal graph and redacted resource snapshot: [D-20260715-086](../decisions/decisions.md), [W-20260715-091](../logs/work.md), [`docs/members-statistics-workflow.md`](../../docs/members-statistics-workflow.md).
- F020 generated platform coverage and proxy transition: [D-20260715-087](../decisions/decisions.md), [W-20260715-092](../logs/work.md), [`docs/platform-utilities-workflow.md`](../../docs/platform-utilities-workflow.md).
- F021 cross-cutting reliability dispatch: [D-20260715-088](../decisions/decisions.md), [W-20260715-093](../logs/work.md), [`docs/feature-logic-harness/reliability-policy-observability.md`](../../docs/feature-logic-harness/reliability-policy-observability.md).
- F022 compact agent skill и P7 Acceptance: [D-20260715-073](../decisions/decisions.md), [D-20260715-088](../decisions/decisions.md), [W-20260715-094](../logs/work.md), [`docs/agent-skill-eval.md`](../../docs/agent-skill-eval.md).
- MCP small tool adapter: [D-20260715-089](../decisions/decisions.md), [W-20260715-095](../logs/work.md), [`docs/feature-logic-harness/mcp.md`](../../docs/feature-logic-harness/mcp.md).
- MCP local/SSH transport: [D-20260715-090](../decisions/decisions.md), [W-20260715-096](../logs/work.md), [`docs/mcp-transport.md`](../../docs/mcp-transport.md).
- MCP brokered login и P8 Acceptance: [D-20260715-091](../decisions/decisions.md), [W-20260715-097](../logs/work.md), [`docs/cli-secure-login.md`](../../docs/cli-secure-login.md).

## Operating rules

- Wiki pages — компактный synthesis; обновляются при изменении verified state.
- Work, decisions и problems не смешиваются в одном журнале; гранулярность — пункт Tasks фазы, не отдельный метод (см. `plans.md`, «Правила работы»).
- Raw digests — только для внешних доказательств (сборка, сеть, live-сессия).
- `.env.local` используется только через protected loader; значения не читаются и не записываются в память.
