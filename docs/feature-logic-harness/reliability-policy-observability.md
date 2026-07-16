# Feature Logic Harness: надёжность, policy и наблюдаемость

## Summary

- Feature ID: F021
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: ограниченно повторять безопасные операции, не дублировать мутации и давать оператору измеримую картину работы.
- Product workflow/job served: classify -> authorize -> schedule -> execute -> retry/reconcile -> audit/metrics.
- Primary ambiguity to keep explicit: Telegram limits динамические и не фиксируются выдуманными константами.

## Product Context

- Product context source: product.md
- Product purpose: безопасная автоматизация реального аккаунта.
- Primary users: агент, владелец, operator.
- Core workflows touched: все methods/workflows.
- Domain terms used: risk class, retry class, uncertain outcome, plan capability.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: safety rules; limits: none.
- SRC002: HARNESS.md; type: file; supports: retry/policy/metrics invariants; limits: none.
- SRC003: plans.md P5; type: file; supports: classes/gates и accepted kernel; limits: live domain verification belongs to P10.
- SRC004: user request; type: supplied; supports: retries/limits/metrics requirement; limits: exact budgets unspecified.

## TDLib API Coverage

- Every generated method receives explicit risk, retry, sensitivity and surface classification.
- Regex/prefix classification is bootstrap only; unreviewed method remains blocked.
- State-critical updates receive lag/resync and metric decisions.

## Request Graph

`descriptor -> scope/policy -> queue/rate budget -> dispatch -> terminal/timeout -> safe retry or reconciliation -> audit/result metrics`.

## Completion Proof

Fault proof is executable and split by boundary: `workflows::safe_read_retries_tdlib_flood_once`,
`retry::safe_read_respects_server_delay_before_retry` and
`raw_api::flood_delay_comes_only_from_tdlib_rate_limit_errors` cover flood;
`workflows::send_timeout_is_uncertain_and_is_not_repeated` covers send timeout;
`workflows::gapped_state_blocks_workflow_until_snapshot_resync` covers update gap;
`transport::deadline_and_explicit_cancellation_remove_pending_response` covers cancellation;
`idempotency::interrupted_dispatch_requires_reconciliation_after_reopen` covers crash; and
`retry::uncertain_outcome_stops_without_retry` covers unknown mutation outcome.

## Cache and Update Semantics

Update lag, cache age/source and reducer sequence appear in workflow metrics/result, never silently hidden.

## Retry and Reconciliation

Safe reads may bounded-retry after server delay. Convergent set-to-value probes current state. Send/create/delete/join/payment timeouts become uncertain and reconcile before repeat.

## CLI/MCP Exposure

Both surfaces receive the same policy/error/retry envelope. Agent cannot override risk class with a flag/tool argument.

## Permissions and Account Capabilities

Scopes: read, presence, send, reversible mutation, admin, destructive, financial, auth/security. High-risk operations use preview + plan hash + external capability.

## Live Verification Boundary

P1 реализует transport deadlines/cancellation и native secret-output canary. Raw daemon
dispatch после policy/approval входит в generated-risk scheduler, journal и owner-only
audit. TDLib 429 для generated `safe_read` блокирует method class, выдерживает supplied
delay и допускает один bounded repeat; другие retry classes generic layer не повторяет.
Generic raw mutation response остаётся partial `reconciliation_required`, потому что один
TDLib response не доказывает domain terminal state. Reconcile/never workflows используют
canonical workflow fingerprint, а typed workflow сохраняет incomplete outcome как
`uncertain`. Shared fixed-shape metrics доступны через
CLI status и не содержат payload/identifier labels. Live Telegram fault injection и
измеренные multi-read budgets остаются P10/Q001.

## Scope

### In scope

- Scheduling, backpressure, dynamic limits, retry/reconciliation, idempotency journal, policy, approval, audit, metrics/redaction.

### Out of scope

- Circumventing Telegram limits, hidden behavior, self-authored human approval.

### Ambiguous

- Default per-profile concurrency budgets require measurement; see Q001.

## Context Map

- User surfaces: status/errors/approvals/metrics dashboard.
- Backend surfaces: scheduler, policy engine, idempotency journal, telemetry.
- Data entities: Descriptor, PlanCapability, OperationRecord, Budget.
- External dependencies: TDLib errors/updates and metrics backend.
- Async flows: delayed retry/reconciliation/approval expiry.
- Config flags: budgets, deadlines, metric exporter, policy scopes.
- Tests/examples/docs: fault and secret-leak suites.
- Observability: latency/queue/retry/flood/update lag/cache/workflow/lease/close metrics.

## Actors and Permissions

- Агент: cannot expand its own scopes or sign approval.
- Human approver/external broker: authorizes exact plan hash вне model-visible interfaces.
- Scheduler/policy: authoritative dispatch gate.

## Domain Entities

- OperationRecord: fingerprint, state, outcome, reconciliation evidence.
- PlanCapability: signed/scoped/expiring approval.
- Budget: account/chat/method-class capacity and blocked_until.

## State Model

- Planned -> Authorized -> Queued -> Running -> Succeeded/Failed/Uncertain -> Reconciling -> Resolved.
- Uncertain never auto-transitions back to Queued.

## Operations and Data Model

- Operations: classify, authorize, schedule, retry, reconcile, audit.
- Reads: descriptor/current state/budgets.
- Writes: idempotency/audit/metrics.
- Side effects: dispatch only after gates.
- Shapes: stable error codes, retry_after, risk and reconciliation metadata.

## Contracts

- C001: every method has explicit risk/retry class.
- C002: unknown mutation outcome is never blindly retried.
- C003: metrics/audit redact secrets and payloads.

## Invariants

- I001: agent cannot self-approve.
- I002: limit handling is conservative and auditable.
- I003: telemetry labels exclude Telegram identifiers/content.

## Dimensions

- D001 - Operation risk
  - Description: read through auth/financial; Status: filled; Values: scope classes; Boundary values: destructive/financial; Why it matters: approval/exposure; Related entities: Descriptor/Capability; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Outcome certainty
  - Description: known/timeout/uncertain/reconciled; Status: filled; Values: four; Boundary values: remote side effect before timeout; Why it matters: retry; Related entities: OperationRecord; Related contracts: C002; Related invariants: I002; Unknowns: none.

## Domain Overlays Used

- Security/async/observability: cross-cutting execution semantics.

## Scenario Cells

- SC001 - Flooded read
  - Dimensions: D001, D002; Workflow/entity anchor: retry; Scenario: read returns delay; Expected behavior: bounded wait then retry; Related contracts: C001; Related invariants: I002; Why this matters: safe resilience; Status: implemented in common raw dispatch with generated class, parsed TDLib 429 and synthetic delay proof.
- SC002 - Send timeout
  - Dimensions: D001, D002; Workflow/entity anchor: reconciliation; Scenario: no terminal send update before deadline; Expected behavior: uncertain, probe, no blind resend; Related contracts: C002; Related invariants: I002; Why this matters: prevents duplicates; Status: send returns incomplete uncertain exactly once; journal/restart blocks repeat until reconciliation.

## Assumptions

- A001: exact budgets are tuned from metrics rather than encoded as Telegram truth; support_basis: inference.

## Open Questions

- Q001: initial read/write concurrency values; owner: maintainer/operator; non-blocking.

## Coverage Notes

- Implemented: generated risk/retry admission, eight typed lease scopes with owner ceiling,
  exact-plan Ed25519 approval, raw queue/rate admission, bounded flood retry, durable raw/
  reconcile-workflow journal, fixed status metrics и redacted owner-only raw audit.
- Modeled: protected operator UI/wire delivery; unavailable authoritative probes remain
  explicit `reconciliation_required`, never an automatic repeat.
- Partial: measured multi-read production budgets and live Telegram fault injection belong
  to Q001/P10 and do not weaken synthetic Acceptance.
- Unknown: measured production thresholds.
- Not applicable: domain-specific data shapes.
