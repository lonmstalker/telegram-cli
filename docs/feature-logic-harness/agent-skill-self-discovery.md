# Feature Logic Harness: компактный agent skill и self-discovery

## Summary

- Feature ID: F022
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: научить cold agent корректно пользоваться платформой в малом контексте, загружая детали API по требованию.
- Product workflow/job served: acquire -> capabilities -> workflow/describe/call -> follow next_action -> release.
- Primary ambiguity to keep explicit: финальный token budget проверяется реальным eval tokenizer.

## Product Context

- Product context source: product.md
- Product purpose: агент использует весь TDLib без огромной инструкции.
- Primary users: AI-агент и maintainer harness.
- Core workflows touched: session, discovery, chains, safety, Mini App handoff.
- Domain terms used: cold agent, self-discovery, context budget.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: agent workflow; limits: none.
- SRC002: HARNESS.md; type: file; supports: invariants; limits: none.
- SRC003: plans.md P6 и `.agents/skills/telegram-cli`; type: file; supports: compact skill budget/on-demand flow; limits: live model variance не измерялась.
- SRC004: user request; type: supplied; supports: concise but precise skill requirement; limits: exact tokenizer unspecified.

## TDLib API Coverage

- Skill contains no hand-maintained method list. It calls `capabilities/schema describe` on demand.
- Every semantic workflow supplies discoverable examples through protocol metadata, not skill prose.

## Request Graph

`session acquire -> inspect capabilities -> prefer workflow -> describe before raw fallback -> execute -> follow incomplete/next_action -> release`.

## Completion Proof

Skill accepted only when cold-agent evals complete representative history/statistics/sticker/bot/Mini App tasks without duplicate login, false not-found or unsafe mutation.

## Cache and Update Semantics

Skill teaches that `partial/pending/gap` require continuation/resync and never means absent.

## Retry and Reconciliation

Skill forbids self-invented retries; follows broker retry/reconciliation fields and stops on uncertain mutation.

## CLI/MCP Exposure

Same behavioral rules for CLI and MCP; CLI is default. MCP details appear only when connector is configured.

## Permissions and Account Capabilities

Skill never self-approves or asks model to reveal OTP/key; it requests operator action through the provided secure path.

## Live Verification Boundary

Repo-local skill использует только stable CLI machine envelope и on-demand
`workflow list/describe`/schema discovery. Offline cold-context traces закрывают history,
statistics, sticker, bot и Mini App handoff; live Telegram side effects остаются P10.

## Scope

### In scope

- Minimal routing rules, safety invariants, compact output, on-demand discovery and evals.

### Out of scope

- API encyclopedia, deployment runbook, secret values, duplicated workflow documentation.

### Ambiguous

- Exact tokenizer and budget gate remain undecided; see Q001.

## Context Map

- User surfaces: repo-local skill and generated help.
- Backend surfaces: CLI/MCP capabilities/schema.
- Data entities: SkillInstruction, CapabilityDescriptor, EvalScenario.
- External dependencies: agent runtime tokenizer/harness.
- Async flows: continuation and operator handoff.
- Config flags: surface/profile/compact mode.
- Tests/examples/docs: cold-agent eval corpus.
- Observability: tool count, token usage, recovery path and unsafe-attempt failures.

## Actors and Permissions

- Cold agent: follows skill, cannot grant scopes.
- Operator: configures profile/approvals.
- Maintainer: updates skill only when protocol invariants change.

## Domain Entities

- SkillInstruction: compact stable rule.
- EvalScenario: input/task/expected calls/safety assertions.
- CapabilityDescriptor: on-demand method/workflow metadata.

## State Model

- Unattached -> Acquired -> Discovered -> Executing/Waiting -> Complete/Partial/Blocked -> Released.

## Operations and Data Model

- Operations: acquire, discover, execute, continue, release.
- Reads: capability/result metadata.
- Writes: none outside delegated workflow.
- Side effects: governed by daemon policy.
- Shapes: concise rules and machine envelopes.

## Contracts

- C001: skill stays under agreed context budget.
- C002: skill never enumerates full TDLib API.
- C003: skill explicitly handles incomplete/uncertain/approval states.

## Invariants

- I001: skill cannot create a second TDLib owner/login.
- I002: detailed docs load only on demand.
- I003: no secret or approval forgery instructions.

## Dimensions

- D001 - Agent state
  - Description: cold/familiar/recovery; Status: filled; Values: three; Boundary values: first raw fallback; Why it matters: discovery amount; Related entities: EvalScenario; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Surface
  - Description: CLI/MCP; Status: filled; Values: two; Boundary values: MCP disabled; Why it matters: routing only; Related entities: CapabilityDescriptor; Related contracts: C002; Related invariants: I002; Unknowns: Q001.

## Domain Overlays Used

- Agent UX/safety: context footprint and deterministic recovery.

## Scenario Cells

- SC001 - Cold-agent history
  - Dimensions: D001, D002; Workflow/entity anchor: history workflow; Scenario: chat not cached; Expected behavior: acquire, workflow, follow next_action, release; Related contracts: C001-C003; Related invariants: I001-I003; Why this matters: primary correctness test; Status: modeled.
- SC002 - Destructive raw method
  - Dimensions: D001, D002; Workflow/entity anchor: raw fallback; Scenario: agent discovers delete/logout; Expected behavior: policy/approval handoff, no self-approval; Related contracts: C003; Related invariants: I003; Why this matters: full API safety; Status: modeled.

## Assumptions

- A001: CLI/MCP expose sufficient self-description to replace prose catalog; support_basis: inference.

## Open Questions

- Q001: final token counting tool and threshold; owner: harness maintainer; non-blocking, default <=1500 tokens.

## Coverage Notes

- Kernel coverage: agent flow/safety/context modeled.
- Modeled: intended minimal skill behavior.
- Partial: live model variance и live Telegram scenario fixtures остаются P10.
- Unknown: exact tokenizer.
- Not applicable: TDLib domain implementation details.
