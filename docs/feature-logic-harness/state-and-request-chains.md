# Feature Logic Harness: состояние и цепочки запросов

## Summary

- Feature ID: F004
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: выполнять prerequisite/pagination/update chains до честного complete result.
- Product workflow/job served: resolve -> preload/open -> request/page -> wait updates -> verify -> close/release.
- Primary ambiguity to keep explicit: freshness является domain-specific, не единым boolean.

## Product Context

- Product context source: product.md
- Product purpose: устранить ложные `not found` и устаревшие результаты.
- Primary users: агент и оператор.
- Core workflows touched: chat lists, history/search, stats, files, sends, bots, Mini Apps.
- Domain terms used: prerequisite graph, terminal proof, gap, freshness, completeness.
- Open product questions: none.

## Source Ledger

- SRC001: product.md; type: file; supports: complete/partial rule; limits: none.
- SRC002: HARNESS.md; type: file; supports: update/cache invariants; limits: none.
- SRC003: `plans.md` P1/P4 и `telegram-core::reducer`; type: file/code; supports: ordered transport-to-cache reducer, versioned core caches and lossless raw unknown queue; limits: durable journal, gap/resync and workflow engine absent.
- SRC004: official TDLib getting-started/schema; type: supplied; supports: ordered updates/loadChats/history semantics; limits: product envelope is local design.

## TDLib API Coverage

- Owns cross-domain orchestration metadata; exact domain methods remain with F007–F020.
- Manifest records prerequisites, update dependencies, cursors and completion rules per method/workflow.

## Request Graph

Generic: `ensure Ready -> resolve references -> hydrate/update cache -> satisfy capabilities -> dispatch -> page/wait -> verify terminal state -> emit result`.

## Completion Proof

Each workflow declares method-specific terminal conditions. Short page, cached full-info response or temporary empty result cannot imply global completeness.

## Cache and Update Semantics

One receiver applies updates in order to versioned User/Chat/Group/File/Message/Auth/Connection caches. Реализованный reducer выдаёт monotonic sequence в transport order; unknown constructors сохраняются целиком в ordered raw queue, а lag/gap handling остаётся дальнейшей boundary.

## Retry and Reconciliation

Missing prerequisites start the graph, not blind retry. Retry classification comes from F021; uncertain mutations probe resulting state before any repeat.

## CLI/MCP Exposure

CLI/MCP expose named workflows and generic `next_action`; raw single calls remain available but do not claim workflow completeness.

## Permissions and Account Capabilities

Graph checks rights/capability fields before protected steps and returns forbidden/requires_* separately from absent data.

## Live Verification Boundary

Pure core tests подтверждают transport-order sequence, representative caches, exact unknown payload/order, terminal message-send state, startup snapshot boundary, history/search/member pagination, async statistics graphs и synthetic file/sticker/bot/Web App terminal chains. Pinned native runtime применяет `getCurrentState` к reducer до дальнейших events. Durable raw journal, gap/resync и остальные domain completion proofs ещё не реализованы.

## Scope

### In scope

- Ordered cache, resolver, pagination, waits, open/close leases, gap recovery, freshness/completeness envelope.

### Out of scope

- Fabricating unavailable data or infinite retries.

### Ambiguous

- None; domain-specific rules belong to corresponding feature harness.

## Context Map

- User surfaces: workflow commands/tools and result envelope.
- Backend surfaces: reducer, cache, workflow executor, event waiters.
- Data entities: CachedEntity, Cursor, CompletionProof, Gap, WorkflowRun.
- External dependencies: TDLib updates/functions.
- Async flows: update wait, paging, reconnect/resync.
- Config flags: deadlines/page budgets/freshness policy.
- Tests/examples/docs: negative prerequisite/short-page/gap scenarios.
- Observability: step latency, cache source/age, gap and resync counts.

## Actors and Permissions

- Агент: requests workflow and follows next_action.
- Core: owns state/chain decisions.
- Operator: sets budgets but not false completion.

## Domain Entities

- CompletionProof: terminal rule and evidence.
- WorkflowRun: steps, deadline, state, result.
- Gap: affected sequence/cache domains and resync status.

## State Model

- Pending -> Running -> WaitingUpdate/Paging -> Complete/Partial/Forbidden/Uncertain/Failed.
- Gapped cache cannot transition to Complete until resync proof.

## Operations and Data Model

- Operations: resolve, ensure/open, page, wait, reconcile, resync.
- Reads: cache and TDLib responses.
- Writes: cache/reducer/workflow journal.
- Side effects: optional membership/presence only through explicit workflow scope.
- Input and output shapes: request budget + structured status/freshness/next_action.

## Contracts

- C001: prerequisites are explicit and deterministic.
- C002: every paginated workflow has cursor/no-progress/deadline rules.
- C003: gap prevents false complete.

## Invariants

- I001: `partial/pending` never becomes `not_found` by prose formatting.
- I002: resolver does not implicitly join unless workflow requests membership.
- I003: update receiver is never blocked by a workflow step.

## Dimensions

- D001 - Cache state
  - Description: fresh/stale/not-loaded/gapped; Status: filled; Values: four; Boundary values: gap; Why it matters: completion; Related entities: Gap/CachedEntity; Related contracts: C001/C003; Related invariants: I001/I003; Unknowns: none.
- D002 - Page progress
  - Description: full/short/empty/repeated cursor/boundary; Status: filled; Values: progress cases; Boundary values: repeated cursor; Why it matters: termination; Related entities: Cursor/Proof; Related contracts: C002; Related invariants: I001; Unknowns: none.

## Domain Overlays Used

- Async/data lifecycle: ordered updates, cursors, resync and uncertainty.

## Scenario Cells

- SC001 - Существующий, но не загруженный чат
  - Dimensions: D001, D002; Workflow/entity anchor: resolve/history; Scenario: direct getChat would fail; Expected behavior: resolver/cache hydration then history; Related contracts: C001/C002; Related invariants: I001/I002; Why this matters: основной false-not-found case; Status: modeled.
- SC002 - Lagged update stream
  - Dimensions: D001, D002; Workflow/entity anchor: resync; Scenario: receiver reports gap; Expected behavior: partial + resync, never complete; Related contracts: C003; Related invariants: I001/I003; Why this matters: state honesty; Status: modeled.

## Assumptions

- A001: each domain feature supplies its terminal proof rules; support_basis: inference.

## Open Questions

- None.

## Coverage Notes

- Kernel coverage: state/paging/gap/concurrency modeled.
- Modeled: generic engine and result semantics.
- Partial: ordered/lossless caches, startup snapshot, history/search/member paging, statistics graph traversal и representative terminal-update chains реализованы; durable journal, gap/resync и остальные domain completion rules остаются дальнейшими slices.
- Not applicable: account secret entry.
