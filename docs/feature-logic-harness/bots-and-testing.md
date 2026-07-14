# Feature Logic Harness: боты и bot testing

## Summary

- Feature ID: F012
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: использовать user/bot account для полного bot/inline/callback/game protocol и воспроизводимого тестирования ботов.
- Product workflow/job served: resolve bot -> subscribe updates -> start/send -> correlate reply -> callback/assert -> cleanup.
- Primary ambiguity to keep explicit: test oracle задаётся сценарием, не TDLib.

## Product Context

- Product context source: product.md
- Product purpose: агент тестирует Telegram-ботов и выполняет bot-related TDLib operations.
- Primary users: developer/QA agent и owner.
- Core workflows touched: bot start, inline queries/results, callbacks, games, commands/menu, shipping/precheckout/managed-bot boundary.
- Domain terms used: bot user, reply markup, callback answer, correlation window.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: bot testing use case; limits: none.
- SRC002: HARNESS.md; type: file; supports: full API/policy; limits: none.
- SRC003: pinned official schema; type: supplied; supports: bot method/update families; limits: generated mapping absent.
- SRC004: plans.md P4/P7/P10; type: file; supports: terminal/callback tests; limits: implementation absent.

## TDLib API Coverage

- Primary owner: bot start/commands/menu/inline/guest/callback/game/shipping/precheckout/prepared messages/managed bot and bot testing workflows.
- Generic message send F009; Web Apps/OAuth F013; payments state F018.

## Request Graph

`resolve/preload bot -> establish update boundary -> start/send -> wait correlated incoming message -> inspect markup -> callback/inline/game action -> wait answer/update -> assert/cleanup`.

## Completion Proof

Send uses terminal send state; bot response uses scenario correlation/deadline. Callback timeout/error is reported explicitly, never converted to “bot did not respond” without evidence.

## Cache and Update Semantics

Bot/user/chat/message updates flow through shared reducers; test run records starting sequence and gap invalidates assertions.

## Retry and Reconciliation

Read/inline queries bounded-retry; start/send/callback unknown outcomes are reconciled and not blindly repeated.

## CLI/MCP Exposure

`bot test/start/send/wait/click/inline` workflows plus raw API. Bot interactions need explicit scope; test specs are artifacts, not giant CLI flags.

## Permissions and Account Capabilities

User-side testing and bot-account methods differ. Owner-only BotFather/managed-bot actions require matching account identity and elevated scopes.

## Live Verification Boundary

No messages/callbacks sent in this task. P10 requires disposable/test bot scenarios and exact cleanup.

## Scope

### In scope

- Full bot-related schema reachability and user-side/bot-side test workflows.

### Out of scope

- Bypassing bot ownership, pretending timeout is success/failure, browser Mini App assertions.

### Ambiguous

- The canonical declarative bot-test spec format remains undecided; see Q001.

## Context Map

- User surfaces: bot test workflows/spec/results.
- Backend surfaces: update correlation/assertion runner.
- Data entities: BotTestRun, CorrelationBoundary, Assertion, CallbackReceipt.
- External dependencies: target bot behavior.
- Async flows: incoming reply/callback/inline result/timeouts.
- Config flags: deadlines, cleanup, interaction scopes.
- Tests/examples/docs: fake bot fixtures and bounded live tests.
- Observability: step/result/duration; content redacted by default.

## Actors and Permissions

- QA agent: executes scoped test plan.
- Owner: approves live interactions/ownership actions.
- Test runner: correlates and cleans exact artifacts.

## Domain Entities

- BotTestRun: steps, boundary, assertions, cleanup.
- CallbackReceipt: request/answer/update state.
- BotCapability: user-side/bot-side/owner-only.

## State Model

- Planned -> Subscribed -> Acting -> Waiting -> Passed/Failed/Partial/Uncertain -> Cleaned.

## Operations and Data Model

- Operations: resolve/start/send/wait/inline/callback/game/assert/cleanup.
- Reads: messages/reply markup/capabilities.
- Writes: Telegram test interactions and local run artifact.
- Side effects: test messages/callbacks.
- Shapes: redacted step timeline and exact assertion result.

## Contracts

- C001: test subscribes/records boundary before triggering action.
- C002: reply/callback correlation is explicit.
- C003: cleanup targets only artifacts owned by the run.

## Invariants

- I001: no blind repeated interaction after timeout.
- I002: update gap invalidates pass claim.
- I003: agent cannot impersonate bot owner approval.

## Dimensions

- D001 - Account/test outcome
  - Description: user/bot/owner capability and reply/timeout/gap; Status: partial; Values: combinations; Boundary values: callback timeout, update gap; Why it matters: assertion/policy; Related entities: BotTestRun; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Test target state
  - Description: synthetic/live target and unresolved/ready/gapped chat state; Status: partial; Values: synthetic, approved live, unapproved, ready, gapped; Boundary values: live target lacks approval; Why it matters: side effects and proof; Related entities: BotTestRun/CallbackReceipt; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.

## Domain Overlays Used

- Test automation/async messaging: triggers, correlation, assertions and cleanup.

## Scenario Cells

- SC001 - `/start` и ответ
  - Dimensions: D001, D002; Workflow/entity anchor: BotTestRun; Scenario: user starts test bot; Expected behavior: terminal send + correlated reply; Related contracts: C001/C002; Related invariants: I001/I002; Why this matters: primary flow; Status: modeled.
- SC002 - Callback timeout
  - Dimensions: D001, D002; Workflow/entity anchor: CallbackReceipt; Scenario: no answer within bound; Expected behavior: timeout/uncertain with evidence, no repeat; Related contracts: C002; Related invariants: I001; Why this matters: honest failure; Status: modeled.

## Assumptions

- A001: live acceptance uses an operator-owned disposable bot; support_basis: inference.

## Open Questions

- Q001: JSON/YAML test-spec contract; owner: maintainer; non-blocking for raw coverage.

## Coverage Notes

- Kernel coverage: trigger/correlation/callback/cleanup modeled.
- Modeled: core bot testing flow.
- Partial: bot-account/managed-bot ownership matrix.
- Unknown: canonical test spec.
- Not applicable: Mini App DOM.
