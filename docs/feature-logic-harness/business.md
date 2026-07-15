# Feature Logic Harness: Telegram Business

## Summary

- Feature ID: F017
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: покрыть весь доступный TDLib Business surface, включая connections, managed bots, quick replies, business profile/chat links/messages/stories.
- Product workflow/job served: discover account/business capability -> resolve connection -> load state -> execute scoped workflow -> wait updates -> reconcile and audit.
- Primary ambiguity to keep explicit: Business/Premium entitlement and connection scopes определяются сервером во время выполнения.

## Product Context

- Product context source: product.md
- Product purpose: агент автоматизирует routine Business-операции в пределах делегированных прав.
- Primary users: business owner, operator, support automation и QA agent.
- Core workflows touched: business connections, connected/managed bots, quick replies, greeting/away/location/hours/profile, chat links, messages and stories.
- Domain terms used: business connection, managed bot, quick reply, delegated scope.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: routine agent actions and full API; limits: none.
- SRC002: HARNESS.md; type: file; supports: capabilities, shared session and audit; limits: none.
- SRC003: pinned official schema; type: supplied; supports: Business method/object/update families; limits: source alone does not prove generated registry.
- SRC004: plans.md P3/P7; type: file; supports: generated parity and vertical slices; limits: implementation absent.

## TDLib API Coverage

- Primary owner: all methods/objects/updates whose semantics are Telegram Business, business connections/messages/stories/profile features, chat links, quick replies and managed/connected bots.
- Generic bot management/testing stays F012; generic messages/stories stay F009/F015 unless a Business connection is part of the contract.
- Runtime capability and exact schema registration/classification are both required.

## Request Graph

`get capabilities -> list/resolve connection -> fetch business feature state -> validate delegated scope -> execute read/mutation -> consume business updates -> reread/reconcile -> return scoped proof`.

## Completion Proof

Success includes connection ID, applied scope and current server state. A generic message/story success without matching Business connection context is not sufficient.

## Cache and Update Semantics

Business connection and quick-reply state is connection-scoped. Updates carry ordering and invalidate only the matching connection/account records; disconnect forces capability refresh.

## Retry and Reconciliation

Reads retry boundedly. Message/story/profile/quick-reply writes reconcile via connection-scoped identifiers and current state before retry.

## CLI/MCP Exposure

Commands require explicit connection selection unless exactly one safe default is configured. MCP capabilities hide unavailable Business operations rather than exposing failing tool variants.

## Permissions and Account Capabilities

Check Business/Premium availability, connection validity, managed-bot rights, writable feature flags and chat-specific restrictions.

## Live Verification Boundary

Current account type was verified as regular; no Business entitlement or Business mutation was tested.

## Scope

### In scope

- Complete pinned-schema Telegram Business API and its connection-scoped updates/workflows.

### Out of scope

- Inventing Business entitlement, cross-connection data mixing and unapproved customer messaging.

### Ambiguous

- Live disposable Business fixture/account availability remains unknown; see Q001.

## Context Map

- User surfaces: capability discovery, connection selector and workflows.
- Backend surfaces: connection-scoped caches, policy, update reducer and audit.
- Data entities: BusinessConnectionRef, BusinessFeatureState, QuickReplyRef, DelegatedScope.
- External dependencies: Telegram Business entitlement and connected bots.
- Async flows: connection updates, messages/stories and bot delegation.
- Config flags: allowed connection IDs and mutation scopes.
- Tests/examples/docs: fake multi-connection isolation and optional live fixture.
- Observability: connection class/outcome only; customer/chat identifiers excluded.

## Actors and Permissions

- Business owner: grants connection and approves policy.
- Operator agent: acts within delegated scope.
- Managed bot: external actor whose rights are server-defined.

## Domain Entities

- BusinessConnectionRef: stable connection identity, owner and capability snapshot.
- DelegatedScope: allowed operations/chats and freshness.
- BusinessMutationReceipt: connection-scoped request and reconciliation proof.

## State Model

- Unknown -> Available/Unavailable -> Connected/Disconnected -> Ready -> Mutating -> Confirmed/Uncertain/Failed.

## Operations and Data Model

- Operations: inspect/configure Business features, links and quick replies; connection-scoped bot/message/story actions.
- Reads: connections, rights, feature configuration and templates.
- Writes: Business account configuration and delegated content.
- Side effects: customer communication and public/business profile changes.
- Shapes: connection-scoped envelopes and audit receipts.

## Contracts

- C001: every Business call carries a resolved connection context.
- C002: caches and idempotency keys are isolated per connection.
- C003: unavailable entitlement is a capability result, not missing API.

## Invariants

- I001: no data or mutation crosses connection boundaries.
- I002: disconnect invalidates delegated write capability immediately.
- I003: uncertain customer-facing mutation is reconciled before retry.

## Dimensions

- D001 - Connection/capability/outcome
  - Description: unavailable/disconnected/connected crossed with read/write and confirmed/uncertain; Status: partial; Values: combinations; Boundary values: disconnect during send; Why it matters: tenant isolation; Related entities: BusinessConnectionRef/DelegatedScope; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Connection selection
  - Description: zero/one/multiple matching Business connections; Status: filled; Values: none, one, many; Boundary values: ambiguous multiple match; Why it matters: scope isolation; Related entities: BusinessConnectionRef; Related contracts: C001-C002; Related invariants: I001-I002; Unknowns: none.

## Domain Overlays Used

- Multi-tenant scope, entitlement, messaging and audit overlays.

## Scenario Cells

- SC001 - Two Business connections
  - Dimensions: D001, D002; Workflow/entity anchor: BusinessConnectionRef; Scenario: same chat-like identifier under separate scopes; Expected behavior: explicit connection selection and isolated state; Related contracts: C001-C002; Related invariants: I001; Why this matters: tenant safety; Status: modeled.
- SC002 - Send timeout then disconnect
  - Dimensions: D001, D002; Workflow/entity anchor: BusinessMutationReceipt; Scenario: outcome unknown and capability disappears; Expected behavior: reconcile read-only, block retry; Related contracts: C003; Related invariants: I002-I003; Why this matters: duplicate prevention; Status: modeled.

## Assumptions

- A001: most development uses fake Business capability until an approved fixture exists; support_basis: inference.

## Open Questions

- Q001: which test account/connection can provide P10 live evidence; owner: maintainer; blocking for live Business acceptance only.

## Coverage Notes

- Kernel coverage: connection isolation and capability behavior modeled.
- Modeled: Business families at domain level.
- Partial: exact schema mapping and live entitlement.
- Unknown: live fixture.
- Not applicable: financial settlement.
