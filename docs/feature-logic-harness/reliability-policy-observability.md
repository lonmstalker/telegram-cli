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
- SRC003: plans.md P5; type: file; supports: classes/gates; limits: implementation absent.
- SRC004: user request; type: supplied; supports: retries/limits/metrics requirement; limits: exact budgets unspecified.

## TDLib API Coverage

- Every generated method receives explicit risk, retry, sensitivity and surface classification.
- Regex/prefix classification is bootstrap only; unreviewed method remains blocked.
- State-critical updates receive lag/resync and metric decisions.

## Request Graph

`descriptor -> scope/policy -> queue/rate budget -> dispatch -> terminal/timeout -> safe retry or reconciliation -> audit/result metrics`.

## Completion Proof

Reliability is accepted only with fault injection for flood, timeout, update gap, cancellation, crash and unknown mutation outcome; green happy-path tests are insufficient.

## Cache and Update Semantics

Update lag, cache age/source and reducer sequence appear in workflow metrics/result, never silently hidden.

## Retry and Reconciliation

Safe reads may bounded-retry after server delay. Convergent set-to-value probes current state. Send/create/delete/join/payment timeouts become uncertain and reconcile before repeat.

## CLI/MCP Exposure

Both surfaces receive the same policy/error/retry envelope. Agent cannot override risk class with a flag/tool argument.

## Permissions and Account Capabilities

Scopes: read, presence, send, reversible mutation, admin, destructive, financial, auth/security. High-risk operations use preview + plan hash + external capability.

## Live Verification Boundary

P1 реализует transport deadlines/cancellation и native secret-output canary. P5 scheduler
теперь имеет explicit account/chat/generated-risk queue/rate budgets и bounded flood delay
with jitter. Core retry executor допускает только generated `safe_read` и `convergent`:
read ждёт весь supplied server delay, convergent повторяет тот же request только после
desired-state probe. Production values и live FLOOD_WAIT ещё не измерены. Idempotency,
approval, metrics exporter и fault injection остаются следующими P5 slices.

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

- Агент: cannot expand its own scopes or approval.
- Human approver: authorizes exact plan hash.
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
  - Dimensions: D001, D002; Workflow/entity anchor: retry; Scenario: read returns delay; Expected behavior: bounded wait then retry; Related contracts: C001; Related invariants: I002; Why this matters: safe resilience; Status: implemented in core executor with synthetic delay.
- SC002 - Send timeout
  - Dimensions: D001, D002; Workflow/entity anchor: reconciliation; Scenario: no terminal send update before deadline; Expected behavior: uncertain, probe, no blind resend; Related contracts: C002; Related invariants: I002; Why this matters: prevents duplicates; Status: modeled.

## Assumptions

- A001: exact budgets are tuned from metrics rather than encoded as Telegram truth; support_basis: inference.

## Open Questions

- Q001: initial read/write concurrency values; owner: maintainer/operator; non-blocking.

## Coverage Notes

- Kernel coverage: generated risk/retry admission, queue/rate scopes, flood backoff и bounded
  safe-read/convergent retry implemented; durable outcome journal/telemetry remain modeled.
- Modeled: policy and reconciliation contract.
- Partial: production budget values, durable reconciliation and exporter.
- Unknown: measured production thresholds.
- Not applicable: domain-specific data shapes.
