# Feature Logic Harness: CLI

## Summary

- Feature ID: F005
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: предоставить человеку и агенту полный, discoverable и стабильный интерфейс к daemon/core.
- Product workflow/job served: session, schema, call, workflow, events и secure login.
- Primary ambiguity to keep explicit: shell completion и rich TUI не входят в MVP.

## Product Context

- Product context source: product.md
- Product purpose: CLI является обязательной поверхностью и может заменить MCP локально.
- Primary users: агент, владелец аккаунта, operator/developer.
- Core workflows touched: все F001–F004 и F007–F022.
- Domain terms used: compact JSON, JSONL, exit code, locator.
- Open product questions: none.

## Source Ledger

- SRC001: product.md; type: file; supports: CLI-first rule; limits: none.
- SRC002: HARNESS.md; type: file; supports: full API/parity; limits: none.
- SRC003: plans.md P6; type: file; supports: commands/acceptance; limits: workflow/login/events pending.

## TDLib API Coverage

- Every generated raw method is reachable through one `td call` contract.
- Every curated core workflow has a CLI route; exact command matrix is generated from protocol descriptors.

## Request Graph

`parse command -> attach/acquire daemon lease -> validate protocol/schema -> execute/stream -> format -> release/hold`.

## Completion Proof

Exit 0 means protocol-declared success, not merely printed output. Partial/uncertain results use stable non-terminal status without being hidden.

## Cache and Update Semantics

CLI does not own cache. Events carry sequence/gap markers; reconnect resumes from supported cursor or declares resync.

## Retry and Reconciliation

CLI never independently retries mutations. It follows broker `retry_after`/`next_action` and preserves idempotency keys.

## CLI/MCP Exposure

This is canonical local surface. Stable protocol schema is shared with MCP; human output is non-contractual, JSON/JSONL is versioned.

## Permissions and Account Capabilities

Commands display capability/policy requirements. Auth secrets use protected TTY/file descriptor, never normal flags.

## Live Verification Boundary

Session и schema/version/capabilities/search/describe используют private daemon JSONL
protocol; universal `td call` идёт через lease-derived policy и единственный core raw API.
CLI не зависит от core и не открывает DB. Hold пока выдаёт bounded lease без heartbeat loop;
workflow/login/events, formatting и signal cleanup принадлежат следующим P6 slices.

## Scope

### In scope

- Session/login/status, schema discovery, raw call, workflows, events, cancellation, human and compact machine output.

### Out of scope

- Direct TDLib DB ownership, 1000 hand-written subcommands, parsing prose as API.

### Ambiguous

- None.

## Context Map

- User surfaces: terminal/stdin/stdout/stderr.
- Backend surfaces: protocol client/Unix socket/remote connector.
- Data entities: Command, Envelope, EventCursor, IdempotencyKey.
- External dependencies: daemon.
- Async flows: watch/stream/cancel/signals.
- Config flags: profile/output/timeout/remote endpoint.
- Tests/examples/docs: golden CLI protocol and exit codes.
- Observability: request ID and safe diagnostics on stderr.

## Actors and Permissions

- Agent: machine output and scoped commands.
- Human/operator: secure login and approvals.
- CLI: thin client only.

## Domain Entities

- Locator: username/link/id/profile-safe identifier.
- Envelope: versioned result/error/partial metadata.
- EventCursor: resume/gap state.

## State Model

- Detached -> Attached -> Running/Streaming -> Released.
- Interrupted stream releases lease unless explicit hold remains.

## Operations and Data Model

- Operations: status/login/schema/call/workflow/events/lease.
- Reads/writes: protocol only; local config contains references, not secrets.
- Side effects: delegated and audited by daemon.
- Shapes: versioned JSON/JSONL plus human presentation.

## Contracts

- C001: all core raw/workflow routes are discoverable from CLI.
- C002: machine output is stable and prose-free.
- C003: signal/pipe close has deterministic cancellation/lease behavior.

## Invariants

- I001: CLI never bypasses daemon policy.
- I002: auth secret is not a normal command-line argument.
- I003: partial/uncertain status remains machine-visible.

## Dimensions

- D001 - Consumer
  - Description: human/agent/pipeline; Status: filled; Values: three; Boundary values: non-TTY auth; Why it matters: format/secret handling; Related entities: Envelope; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Connection
  - Description: local/remote/reconnect; Status: filled; Values: three; Boundary values: interrupted stream; Why it matters: lease/cursor; Related entities: EventCursor; Related contracts: C003; Related invariants: I003; Unknowns: none.

## Domain Overlays Used

- Developer tooling/protocol: stable automation surface.

## Scenario Cells

- SC001 - Agent reads history
  - Dimensions: D001, D002; Workflow/entity anchor: workflow; Scenario: compact JSON local call; Expected behavior: structured complete/next_action result; Related contracts: C001/C002; Related invariants: I001/I003; Why this matters: primary agent path; Status: modeled.
- SC002 - Ctrl-C during watch
  - Dimensions: D001, D002; Workflow/entity anchor: events; Scenario: interrupted stream; Expected behavior: cancel/release without daemon stop if other leases exist; Related contracts: C003; Related invariants: I003; Why this matters: cleanup; Status: modeled.

## Assumptions

- A001: local agents can spawn or connect to a CLI process; support_basis: explicit_user_decision.

## Open Questions

- None.

## Coverage Notes

- Kernel coverage: session и full generated schema/raw-call routes implemented; remaining workflow/login/events/output/cancellation modeled.
- Modeled: full raw/workflow reachability contract.
- Partial: session/schema/raw-call grammar implemented; workflow/login/events and goldens absent.
- Unknown: none blocking product intent.
- Not applicable: TDLib domain semantics owned elsewhere.
