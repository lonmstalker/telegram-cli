# Feature Logic Harness: единая сессия и жизненный цикл

## Summary

- Feature ID: F001
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: несколько агентов переиспользуют одного владельца TDLib DB без повторного login и конфликтов.
- Product workflow/job served: acquire -> use -> release -> idle close -> restart.
- Primary ambiguity to keep explicit: default idle timeout остаётся deployment setting.

## Product Context

- Product context source: product.md
- Product purpose: полный и безопасный агентный доступ к Telegram.
- Primary users: AI-агент, владелец аккаунта, оператор.
- Core workflows touched: session acquire/release, returning login, shutdown/restart.
- Domain terms used: profile, lease, daemon, terminal close.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: purpose/users/rules; limits: none.
- SRC002: HARNESS.md; type: file; supports: feature boundary/invariants; limits: none.
- SRC003: plans.md; type: file; supports: lifecycle architecture and gates; limits: implementation absent.
- SRC004: official TDLib `close`/`authorizationStateClosed`; type: supplied; supports: C002/I002; limits: daemon protocol is local design.

## TDLib API Coverage

- Primary owner: lifecycle termination/status methods; exact assignments come from generated ownership manifest.
- Coverage levels: codec, surface, semantics, workflow, live proof are tracked separately.
- Cross-feature dependencies: F002 authorization, F003 schema registry, F021 reliability.

## Request Graph

`acquire -> socket probe -> startup lock -> daemon Ready -> lease -> requests -> release/TTL -> draining -> close -> authorizationStateClosed -> unlock`.

## Completion Proof

Session stop is complete only after `authorizationStateClosed`, no in-flight workflow/watch/job and released OS lock. Process exit or response `ok` alone is insufficient.

## Cache and Update Semantics

Lifecycle consumes ordered authorization/connection updates. Crash recovery marks old leases expired and rehydrates state before accepting work.

## Retry and Reconciliation

Acquire may retry deterministic startup races; `close` is not followed by a second owner until lock release. `logOut`/`destroy` never participate in automatic recovery.

## CLI/MCP Exposure

CLI: `session status/acquire/hold/release/stop`. MCP: brokered status/acquire/release; no direct DB path or secret arguments.

## Permissions and Account Capabilities

Any authenticated principal may hold a scoped lease; force-stop, logout and destroy require operator/destructive scope.

## Live Verification Boundary

Current evidence proves one encrypted returning session can reach Ready, `getMe` and Closed. Multi-agent/lease behavior remains unimplemented.

## Scope

### In scope

- Singleton ownership, autostart, leases, heartbeat, idle/resident modes, crash recovery and graceful close.

### Out of scope

- Telegram account logout as ordinary shutdown; sharing one DB between hosts/processes.

### Ambiguous

- The production default idle timeout remains undecided; see Q001.

## Context Map

- User surfaces: CLI/MCP session commands.
- Backend surfaces: daemon, lock, socket, scheduler.
- Data entities: profile, lease, owner lock, daemon state.
- External dependencies: TDLib DB/files and OS process primitives.
- Async flows: heartbeat expiry, draining, restart.
- Config flags: idle mode/timeout, lease TTL.
- Tests/examples/docs: plans.md P2/P10.
- Observability: active leases, owner PID, state, close duration; no Telegram identifiers as labels.

## Actors and Permissions

- Агент: scoped lease, no lifecycle-destructive actions.
- Оператор: profile configuration, force stop, repair.
- Daemon: единственный DB owner.

## Domain Entities

- Profile: account/DC/DB/files/key references.
- Lease: principal, scopes, TTL, heartbeat, status.
- DaemonState: Stopped, Starting, Ready, Draining, Closed, Failed.

## State Model

- `Stopped -> Starting -> Ready`: first acquire wins startup election.
- `Ready -> Draining -> Closed`: zero work and idle policy trigger close.
- `Draining -> Ready`: new acquire may cancel draining only before TDLib close dispatch; otherwise bounded retry.

## Operations and Data Model

- Operations: acquire, renew, release, status, graceful stop.
- Reads: daemon/profile/lease state.
- Writes: lease journal and lifecycle markers.
- Side effects: open/close TDLib DB and process.
- Input and output shapes: stable protocol envelope with state/retry_after/lease_id.

## Contracts

- C001: canonical DB path has exactly one live owner.
- C002: normal stop uses `close` and waits `authorizationStateClosed`.
- C003: agent crash cannot keep a lease forever.

## Invariants

- I001: CLI/MCP never open TDLib DB.
- I002: normal idle stop preserves authorization.
- I003: one agent release does not stop a session used by another.

## Dimensions

- D001 - Concurrency
  - Description: число клиентов и race startup/close; Status: filled; Values: one, many, crash; Boundary values: acquire during draining; Why it matters: owner uniqueness; Related entities: Lease/DaemonState; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Runtime mode
  - Description: on-demand/resident/scheduled; Status: partial; Values: three modes; Boundary values: background watcher exists; Why it matters: close eligibility; Related entities: Profile; Related contracts: C002; Related invariants: I002; Unknowns: Q001.

## Domain Overlays Used

- Concurrency/lifecycle: shared session and crash recovery are the feature itself.

## Scenario Cells

- SC001 - Два агента одновременно
  - Dimensions: D001, D002; Workflow/entity anchor: acquire; Scenario: два startup requests на stopped profile; Expected behavior: один daemon, два leases; Related contracts: C001; Related invariants: I001/I003; Why this matters: предотвращает DB conflict; Status: modeled.
- SC002 - Последний lease завершён
  - Dimensions: D001, D002; Workflow/entity anchor: idle close; Scenario: work/watch/job отсутствуют; Expected behavior: close -> Closed -> unlock; Related contracts: C002; Related invariants: I002; Why this matters: безопасная остановка; Status: modeled.

## Assumptions

- A001: OS предоставляет надёжный advisory/exclusive lock; support_basis: inference.

## Open Questions

- Q001: какой idle timeout использовать по умолчанию для local/server; owner: operator; non-blocking.

## Coverage Notes

- Kernel coverage: lifecycle/concurrency/recovery modeled.
- Modeled: intended session semantics.
- Partial: platform-specific lock/socket details.
- Unknown: default timeout.
- Not applicable: Telegram domain data operations.
