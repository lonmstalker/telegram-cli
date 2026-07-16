# Feature Logic Harness: локальный и серверный MCP

## Summary

- Feature ID: F006
- Context sufficiency: blocked_by_decision
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: blocked_by_decision
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: опционально открыть тот же broker/protocol MCP-клиентам без второго TDLib owner.
- Product workflow/job served: brokered login, status/discovery/call/workflow/events через local или authenticated remote transport.
- Primary ambiguity to keep explicit: feature начинается только после acceptance CLI/core.

## Product Context

- Product context source: product.md
- Product purpose: remote/standard tool access без дублирования логики.
- Primary users: remote AI-агент и operator.
- Core workflows touched: все protocol routes, кроме brokered secret entry.
- Domain terms used: adapter, transport principal, scoped tool.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: optional MCP rule; limits: none.
- SRC002: HARNESS.md; type: file; supports: parity/one-owner invariants; limits: none.
- SRC003: plans.md P8/P9 и `apps/telegram-mcp`; type: file/code; supports: decision gate, small tool inventory, strict protocol translation and transport-owned principal; limits: transport choice deferred.

## TDLib API Coverage

- MCP does not create tool-per-method. Eight stable tools expose session, brokered auth metadata, generic schema/call, workflows and events over the same daemon protocol.
- Each method records `direct`, `brokered` or `blocked` MCP exposure with reason; parity report is generated.
- Curated `workflow` inputs do not require TDJSON constructors. Only the universal raw `call` accepts a schema-described request with `@type`; generated Rust validation still runs in core before dispatch.

## Request Graph

Working path: `authenticate transport principal -> acquire scoped daemon lease -> schema/workflow/call -> stream/poll events -> release`.

Login path: `auth.begin -> challenge_id + next_action -> owner submits secret through protected local TTY/SSH operator channel -> auth.status/auth.wait -> Ready -> getMe proof`.

## Completion Proof

MCP parity is complete only when generated protocol matrix matches CLI/core and start/stop tests prove no extra TDLib client/session.

## Cache and Update Semantics

MCP subscribes to broker events with sequence/gap handling; it owns no cache.

## Retry and Reconciliation

Transport reconnect does not replay non-idempotent operations. Request/idempotency IDs survive reconnect.

## CLI/MCP Exposure

Minimal tools: `session`, `auth.begin`, `auth.status`, `auth.wait`, `schema`, `workflow`, `call` and `events`. MCP coordinates login but carries only challenge metadata; secrets remain in a protected CLI/TTY or SSH operator channel. Principal is injected by transport context and cannot be supplied as a tool argument.

## Permissions and Account Capabilities

Remote principal identity maps to scopes/policy; public unauthenticated access is forbidden.

## Live Verification Boundary

The strict adapter and its MCP 2025-11-25-compatible tool schemas are unit-tested against shared `DaemonRequest`; binary startup still fails closed because local/remote transports belong to the next P8 task. No live endpoint has been started.

## Scope

### In scope

- Local stdio and authenticated server adapter, brokered login coordination, shared schemas and scoped streaming/calls.

### Out of scope

- Direct DB access, separate auth state machine, secret submission as ordinary tool args, 1000 tools.

### Ambiguous

- The exact remote transport/auth stack remains an implementation decision; see Q001.

## Context Map

- User surfaces: MCP tools/resources.
- Backend surfaces: MCP adapter and protocol client.
- Data entities: Principal, Scope, ToolEnvelope, EventCursor.
- External dependencies: MCP runtime and secure transport.
- Async flows: event streaming/reconnect.
- Config flags: local/remote mode, identity provider, TLS/SSH settings.
- Tests/examples/docs: generated parity and no-second-owner tests.
- Observability: principal/request/scope without Telegram payload labels.

## Actors and Permissions

- Remote agent: scoped methods/workflows.
- Operator: endpoint identity and policy configuration.
- MCP adapter: no secret/DB authority.

## Domain Entities

- Principal: authenticated remote/local identity.
- ExposureDecision: direct/brokered/blocked with reason.
- ToolEnvelope: protocol-compatible result.

## State Model

- Disabled -> Configured -> Listening -> Draining/Stopped.
- Connection loss -> Detached; in-flight uncertain writes are reconciled, not replayed.

## Operations and Data Model

- Operations: session status, auth begin/status/wait, schema/call/workflow/events.
- Reads/writes: protocol only.
- Side effects: delegated to daemon under scopes.
- Shapes: small stable tool schemas with on-demand descriptions.

## Contracts

- C001: MCP startup never creates a TDLib client.
- C002: enabled MCP routes have CLI/protocol parity.
- C003: remote transport is authenticated and encrypted.
- C004: MCP login uses the daemon auth state machine and challenge IDs; secret submission stays outside MCP arguments.

## Invariants

- I001: disabling MCP cannot reduce core/CLI capability.
- I002: secrets never enter model-visible tool arguments.
- I003: reconnect never blindly repeats uncertain mutation.
- I004: one MCP-coordinated login cannot create a second DB owner.

## Dimensions

- D001 - Deployment
  - Description: disabled/local/server; Status: filled; Values: three; Boundary values: public bind; Why it matters: trust boundary; Related entities: Principal; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Exposure
  - Description: direct/brokered/blocked; Status: filled; Values: three; Boundary values: auth-secret/financial method; Why it matters: tool behavior; Related entities: ExposureDecision; Related contracts: C002; Related invariants: I002; Unknowns: none.

## Domain Overlays Used

- Remote security/protocol parity: adapter attack surface and reconnect semantics.

## Scenario Cells

- SC001 - Remote read workflow
  - Dimensions: D001, D002; Workflow/entity anchor: workflow; Scenario: authenticated remote agent requests history; Expected behavior: same envelope as CLI; Related contracts: C002/C003; Related invariants: I001; Why this matters: parity; Status: modeled.
- SC002 - OTP requested through MCP
  - Dimensions: D001, D002; Workflow/entity anchor: auth; Scenario: fresh profile requires OTP; Expected behavior: MCP returns challenge ID/next action, owner submits through protected channel, MCP waits for Ready/getMe; Related contracts: C003/C004; Related invariants: I002/I004; Why this matters: login support with secret isolation; Status: modeled.

## Assumptions

- A001: server operator can provide authenticated transport identity; support_basis: inference.

## Open Questions

- Q001: выбрать SSH-tunneled local transport или TLS/OIDC remote service; owner: operator; blocking for P8 only.

## Coverage Notes

- Kernel coverage: small adapter surface, protocol translation and principal-injection boundary verified; transport parity/security/reconnect modeled.
- Modeled: optional transport and brokered wait behavior.
- Partial: remote transport decision.
- Unknown: exact MCP version/hosting stack.
- Not applicable: direct secret entry in MCP arguments.
