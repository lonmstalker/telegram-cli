# Feature Logic Harness: сообщения, поиск и взаимодействия

## Summary

- Feature ID: F009
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: полностью читать, искать, отправлять и изменять messages/interactions с корректными cursors и terminal updates.
- Product workflow/job served: resolve/open chat -> read/search or plan mutation -> wait terminal message update -> verify.
- Primary ambiguity to keep explicit: read-state/presence по умолчанию не меняются.

## Product Context

- Product context source: product.md
- Product purpose: статистика, рутина и bot testing используют актуальные сообщения без дублей.
- Primary users: агент, owner, bot tester.
- Core workflows touched: history/search/links, send/edit/delete/forward/drafts/scheduled, reactions/polls/checklists/effects.
- Domain terms used: send state, cursor, presence, uncertain mutation.
- Open product questions: none.

## Source Ledger

- SRC001: product.md; type: file; supports: use cases/safety; limits: none.
- SRC002: HARNESS.md; type: file; supports: retry/completion invariants; limits: none.
- SRC003: pinned official schema/getting-started; type: supplied; supports: message/history/update families; limits: source alone does not prove generated registry.
- SRC004: plans.md P4/P7; type: file; supports: workflow gates; limits: implementation absent.

## TDLib API Coverage

- Primary owner: message retrieval/history/search/calendar/links/viewers, send/edit/delete/forward/copy/drafts/scheduling, message-level reaction application/removal/readers, polls/checklists/effects/translation/rich/ephemeral families.
- Files F010; bot callback F012; paid/financial accounting F018; read-state policy cross-references F016/F021.

## Request Graph

Read: `resolve/open -> page by returned cursor -> boundary/no-progress proof`. Write: `current state -> preview/scope -> dispatch with receipt -> wait send/edit/delete updates -> verify/reconcile`.

## Completion Proof

History/search complete only at requested date/count or terminal cursor/no-progress rule. Send complete only after succeeded/failed update or reconciliation evidence.

## Cache and Update Semantics

Reduce new/edit/content/delete/send-state updates; gap invalidates incremental transcript and requires bounded snapshot/resync.

## Retry and Reconciliation

Reads bounded-retry. Send/create/delete/forward timeouts become uncertain; never blind-repeat. Convergent edit may probe current content.

## CLI/MCP Exposure

Curated history/search/send/edit/delete/reaction/poll workflows plus raw methods; compact message output and explicit mark-read flag.

## Permissions and Account Capabilities

Chat permissions, protected content, paid reactions/messages and bot/account type are checked before dispatch.

## Live Verification Boundary

P4 history/search paginator проверен deterministic pages: short history продолжает chain,
returned search cursor управляет следующим call, date/count/exhausted/no-progress различаются.
Live message read/send не выполнялся; mutations await disposable-target approval.

## Scope

### In scope

- All message reads/search/links/content interactions and mutation lifecycle represented by pinned TDLib.

### Out of scope

- Browser UI assertions; automatic mark-read; blind resend after timeout.

### Ambiguous

- None.

## Context Map

- User surfaces: message/history/search/action workflows.
- Backend surfaces: message reducer, pager, receipt/reconciliation journal.
- Data entities: Message, MessageContent, SearchCursor, SendReceipt, Interaction.
- External dependencies: chat permissions/TDLib updates.
- Async flows: send state, media upload, scheduled delivery, poll/reaction updates.
- Config flags: page/deadline/output/mark-read budgets.
- Tests/examples/docs: short page, gap, timeout, protected content.
- Observability: latency/result class without text/user labels.

## Actors and Permissions

- Agent: read by default; scoped message actions.
- Owner/admin: approvals for send/delete/paid/protected operations.
- Core: terminal update correlation.

## Domain Entities

- MessageCursor: returned ID and boundary.
- SendReceipt: operation/message IDs and terminal/uncertain state.
- Interaction: reaction/poll/checklist/effect state.

## State Model

- Read: Pending -> Paging -> Complete/Partial.
- Mutation: Planned -> Sending -> Succeeded/Failed/Uncertain -> Reconciled.

## Operations and Data Model

- Operations: get/history/search/send/edit/delete/forward/react/poll/checklist/draft/schedule.
- Reads/writes: TDLib message state and local receipt metadata.
- Side effects: presence/send/destructive/financial classified separately.
- Shapes: compact/full message plus source/completeness/send state.

## Contracts

- C001: short history/search page does not prove end.
- C002: mark-read/presence is explicit.
- C003: mutation waits terminal update or returns uncertain.

## Invariants

- I001: no blind resend/delete retry.
- I002: incremental view with gap is partial.
- I003: protected/sensitive content is policy-redacted.

## Dimensions

- D001 - Operation/result
  - Description: read/presence/mutation and complete/partial/uncertain; Status: filled; Values: classes; Boundary values: timeout after dispatch; Why it matters: retry/policy; Related entities: Cursor/Receipt; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Stream/network state
  - Description: ordered, delayed or gapped updates under online/offline/timeout transport; Status: filled; Values: ordered, delayed, gapped, offline, timeout; Boundary values: dispatch succeeds before timeout; Why it matters: completeness and duplicate prevention; Related entities: MessageCursor/SendReceipt; Related contracts: C001-C003; Related invariants: I001-I002; Unknowns: none.

## Domain Overlays Used

- Stateful messaging: pagination, terminal updates, permission and idempotency.

## Scenario Cells

- SC001 - Короткая history page
  - Dimensions: D001, D002; Workflow/entity anchor: history; Scenario: fewer messages than limit before date boundary; Expected behavior: continue cursor; Related contracts: C001; Related invariants: I002; Why this matters: avoids missing data; Status: modeled.
- SC002 - Timeout после send
  - Dimensions: D001, D002; Workflow/entity anchor: SendReceipt; Scenario: dispatch occurred, no terminal update; Expected behavior: uncertain + reconcile, no resend; Related contracts: C003; Related invariants: I001; Why this matters: duplicate prevention; Status: modeled.

## Assumptions

- A001: domain manifest distinguishes presence and financial interactions; support_basis: inference.

## Open Questions

- None.

## Coverage Notes

- Kernel coverage: history/chat-search pagination implemented; write/uncertainty modeled.
- Modeled: major message families and safety.
- Partial: exact 1.8.66 rich/AI/ephemeral mapping.
- Unknown: live permission matrix.
- Not applicable: channel membership administration.
