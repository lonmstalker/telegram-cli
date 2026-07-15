# Feature Logic Harness: Аккаунт, настройки и безопасность

## Summary

- Feature ID: F016
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: поддержать полный TDLib surface настроек аккаунта, privacy, notifications, sessions, websites, password/security и device registration.
- Product workflow/job served: read current configuration -> build desired-state diff -> preview risk -> apply ordered mutations -> reread and prove convergence.
- Primary ambiguity to keep explicit: destructive/security mutations требуют отдельного approval boundary.

## Product Context

- Product context source: product.md
- Product purpose: агент выполняет routine account configuration, не теряя сессию и не ослабляя безопасность незаметно.
- Primary users: account owner, security operator и automation agent.
- Core workflows touched: profile/settings, privacy, notifications, push devices, active sessions/websites, password/recovery, account TTL/deletion.
- Domain terms used: desired state, active session, recovery state, approval plan hash.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: owner jobs and default-deny destructive actions; limits: none.
- SRC002: HARNESS.md; type: file; supports: capability/risk dimensions; limits: none.
- SRC003: pinned official schema; type: supplied; supports: account/settings/security method families; limits: source alone does not prove generated registry.
- SRC004: plans.md P5/P7; type: file; supports: preview/approval/reconciliation; limits: implementation absent.

## TDLib API Coverage

- Primary owner: Ready-state account/profile settings not owned by F007, privacy rules, notifications, push devices, active sessions/websites, account password/recovery email, account TTL and deletion/security settings.
- Authentication password/email challenges before Ready belong to F002; Passport data belongs to F018.
- Generated manifest must classify every matching method by read/reversible/admin-security/destructive risk.

## Request Graph

`get current state -> normalize desired state -> compute diff -> capability/risk check -> preview + plan hash where required -> apply ordered calls -> consume updates/reread -> report converged/partial/uncertain`.

## Completion Proof

Writes complete only after current settings converge to desired state. Session termination/account deletion require explicit target identity and postcondition proof.

## Cache and Update Semantics

Settings reads use versioned snapshots; relevant updates invalidate them. Active sessions/websites are always refreshed before destructive operations.

## Retry and Reconciliation

Reads retry boundedly. Convergent setters may reapply only after reread. Password, session termination and deletion never blind-retry after uncertain outcome.

## CLI/MCP Exposure

Read and preview are machine-friendly. Passwords, recovery codes and database key use protected input channels and never ordinary flags or MCP arguments.

## Permissions and Account Capabilities

Regular/bot account differences, current-session restrictions, 2FA/recovery availability and method-specific server constraints are explicit capability results.

## Live Verification Boundary

Only existing account access was verified via `getMe`; no account setting or security mutation was performed.

## Scope

### In scope

- Complete pinned-schema account configuration, notifications, privacy, device/session, website and security management.

### Out of scope

- Secret recovery storage, silent session termination and irreversible action without external approval.

### Ambiguous

- Human-approval transport for local and remote operation remains undecided; see Q001.

## Context Map

- User surfaces: inspect/diff/preview/apply/verify commands.
- Backend surfaces: settings adapter, policy engine, secure input and audit.
- Data entities: SettingsSnapshot, DesiredSettings, SessionRef, SecurityPlan.
- External dependencies: Telegram and OS keychain/file-secret provider.
- Async flows: updates, password/recovery transitions and logout effects.
- Config flags: default policy, approval TTL and protected input providers.
- Tests/examples/docs: fake desired-state convergence and disposable session tests.
- Observability: category/outcome only; secrets and device/session IDs excluded.

## Actors and Permissions

- Read agent: inspects settings and sessions.
- Automation agent: applies pre-approved reversible desired state.
- Owner/security operator: approves auth/security/destructive changes.

## Domain Entities

- SettingsSnapshot: normalized current state and freshness.
- DesiredSettings: declarative target with omitted/managed fields.
- SecurityPlan: exact target, risk, plan hash, approval and expiry.

## State Model

- Observed -> Planned -> Approved/Denied -> Applying -> Converged/Partial/Uncertain/Failed.

## Operations and Data Model

- Operations: get/set/reset settings, register devices, list/terminate sessions/websites, manage password/recovery and account lifecycle.
- Reads: current configuration, devices, sessions and security state.
- Writes: account-wide settings and authentication/security state.
- Side effects: notifications, session invalidation and possible account loss.
- Shapes: redacted diff, approval receipt and convergence evidence.

## Contracts

- C001: security/destructive writes require immutable approved plan hash.
- C002: sensitive input is never model-visible or persisted in audit.
- C003: target state is reread after mutation.

## Invariants

- I001: current session cannot be accidentally selected by broad termination.
- I002: omitted desired-state fields are not reset.
- I003: uncertain destructive outcome blocks automatic retry.

## Dimensions

- D001 - Risk/convergence
  - Description: read/reversible/security/destructive crossed with planned/applied/converged/uncertain; Status: partial; Values: method-class matrix; Boundary values: session terminate timeout; Why it matters: account safety; Related entities: SettingsSnapshot/SecurityPlan; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Approval channel
  - Description: local/remote approval availability and validity; Status: partial; Values: absent, valid, expired; Boundary values: approval expires during apply; Why it matters: mutation authorization; Related entities: SecurityPlan; Related contracts: C001-C002; Related invariants: I001-I003; Unknowns: Q001.

## Domain Overlays Used

- Desired-state, secrets, approval and destructive-action overlays.

## Scenario Cells

- SC001 - Change notification settings
  - Dimensions: D001, D002; Workflow/entity anchor: DesiredSettings; Scenario: reversible desired-state update; Expected behavior: diff, apply and reread convergence; Related contracts: C003; Related invariants: I002; Why this matters: routine automation; Status: modeled.
- SC002 - Terminate another session
  - Dimensions: D001, D002; Workflow/entity anchor: SecurityPlan; Scenario: exact target approved, response times out; Expected behavior: refresh session list, never blind retry; Related contracts: C001-C003; Related invariants: I001/I003; Why this matters: account safety; Status: modeled.

## Assumptions

- A001: owner approval can be represented by a signed/scoped capability external to the model; support_basis: repo_source.

## Open Questions

- Q001: local TTY confirmation, policy file signature or external approval broker; owner: maintainer; blocking for destructive actions.

## Coverage Notes

- Kernel coverage: desired state, secrets and destructive boundaries modeled.
- Modeled: settings/security method families.
- Partial: exact generated method classification and approval transport.
- Unknown: production secret provider.
- Not applicable: Telegram Passport document processing.
