# Feature Logic Harness: группы, каналы и модерация

## Summary

- Feature ID: F011
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: создавать и настраивать groups/channels/forums, управлять membership/rights/invites/moderation и проверять результат.
- Product workflow/job served: resolve -> capability/current state -> preview diff -> membership/admin operation -> wait update -> verify.
- Primary ambiguity to keep explicit: операции доступны только при фактических правах аккаунта.

## Product Context

- Product context source: product.md
- Product purpose: рутинная настройка каналов и полный TDLib domain coverage.
- Primary users: агент под контролем владельца/администратора.
- Core workflows touched: create/configure/join/leave, members/admins/bans, invite links/requests, forums, boosts/event log.
- Domain terms used: membership, administrator rights, desired-state diff, invite request.
- Open product questions: none.

## Source Ledger

- SRC001: product.md; type: file; supports: routine administration; limits: none.
- SRC002: HARNESS.md; type: file; supports: policy/completion invariants; limits: none.
- SRC003: pinned official schema; type: supplied; supports: group/channel method/update families; limits: source alone does not prove generated registry.
- SRC004: plans.md P4/P7/P10; type: file; supports: workflow/live gates; limits: implementation absent.

## TDLib API Coverage

- Primary owner: basic-group/supergroup/channel creation and configuration, member/admin/restrict/ban/ownership/invite/join-request/event-log/forum-enable/boost/giveaway-admin families.
- Chat container/list and forum/Saved/direct topic CRUD belong to F008; messages to F009; statistics to F019.

## Request Graph

`resolve without join -> get chat/group/full info -> inspect rights/capabilities -> preview desired diff -> approval -> sequential operation -> wait group/chat/member updates -> verify`.

## Completion Proof

Membership/configuration is complete after authoritative state/update matches desired outcome. Invite request sent is pending, not joined; cached full info includes observed_at/cache age.

## Cache and Update Semantics

Reduce basic/supergroup/full/member/chat updates; openChat lease where required. Gap invalidates rights/member snapshots and blocks unsafe changes until refresh.

## Retry and Reconciliation

Reads safe-retry. Desired-state settings may probe/reapply. Join/create/transfer/ban/delete timeout becomes uncertain and is reconciled first.

## CLI/MCP Exposure

Named `channel/group inspect/configure/member/invite/moderate` workflows plus raw calls; mutations require scopes/plan capability.

## Permissions and Account Capabilities

Owner/admin/member/bot capability matrix is explicit; cannot-get-members/statistics and ownership transfer are structured results.

## Live Verification Boundary

Core отделяет read-only resolve/inspection от explicit `ensure_membership`, сохраняет pending
join outcome и не join-ит private invite при cache miss. Members workflow проверяет
`can_get_members`, продолжает короткие страницы и отмечает no-progress как partial.
Live join/member query не выполнялись; будущие тесты используют disposable chats/channels
и cleanup evidence.

## Scope

### In scope

- Full group/channel/forum membership, rights, invite, moderation, configuration and boost-related schema domain.

### Out of scope

- Implicit join during read resolve; claiming rights not held; stealth membership actions.

### Ambiguous

- None.

## Context Map

- User surfaces: admin workflows/preview/audit.
- Backend surfaces: capability resolver, desired-state planner, update verifier.
- Data entities: ChatRights, MemberState, InviteLink, JoinRequest, ConfigPlan.
- External dependencies: Telegram permissions/rate limits.
- Async flows: join requests, member updates, ownership transfer.
- Config flags: approval/risk scopes and operation budgets.
- Tests/examples/docs: permission matrix, pending join, timeout reconciliation.
- Observability: operation/result/retry, no usernames/chat IDs as labels.

## Actors and Permissions

- Agent: read and pre-authorized reversible admin tasks.
- Owner/admin: grants exact plan capability.
- Core: verifies current rights and resulting state.

## Domain Entities

- ConfigPlan: current/desired diff and hash.
- MemberState: role/rights/status/source/freshness.
- InviteOperation: link/request/join outcome.

## State Model

- Plan: Draft -> Approved -> Applying -> Verified/Partial/Uncertain.
- Join: NotMember -> RequestPending/Member/Failed; Member -> Left/Banned.

## Operations and Data Model

- Operations: create/configure/join/leave/invite/approve/member/admin/moderate/boost.
- Reads: chat/group/full/member/event state.
- Writes: Telegram group/channel state and audit receipt.
- Side effects: membership/admin/destructive operations.
- Shapes: capability + diff + step results + final proof.

## Contracts

- C001: read resolve and membership are separate operations.
- C002: mutation requires current rights and exact plan.
- C003: pending invite request is not reported as membership.

## Invariants

- I001: agent cannot self-grant approval or rights.
- I002: gapped/stale rights block unsafe mutation.
- I003: unknown mutation outcome is reconciled before repeat.

## Dimensions

- D001 - Role/outcome
  - Description: owner/admin/member/nonmember and desired/pending/uncertain; Status: filled; Values: combinations; Boundary values: request pending, ownership transfer timeout; Why it matters: capability/retry; Related entities: ConfigPlan/MemberState; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Rights evidence
  - Description: fresh, stale or gapped rights snapshot with absent/present approval; Status: filled; Values: fresh, stale, gapped, unapproved, approved; Boundary values: rights change after preview; Why it matters: safe administration; Related entities: ConfigPlan/MemberState; Related contracts: C001-C002; Related invariants: I001-I002; Unknowns: none.

## Domain Overlays Used

- Authorization/stateful admin: rights, approval, async membership and verification.

## Scenario Cells

- SC001 - Configure channel title
  - Dimensions: D001, D002; Workflow/entity anchor: ConfigPlan; Scenario: admin right present; Expected behavior: preview/apply/wait/verify; Related contracts: C002; Related invariants: I001/I002; Why this matters: primary routine action; Status: modeled.
- SC002 - Invite requires approval
  - Dimensions: D001, D002; Workflow/entity anchor: InviteOperation; Scenario: Telegram returns request sent; Expected behavior: pending with next_action, not member; Related contracts: C003; Related invariants: I003; Why this matters: honest state; Status: modeled.

## Assumptions

- A001: disposable live targets can be provisioned for P10; support_basis: inference.

## Open Questions

- None.

## Coverage Notes

- Kernel coverage: resolve/membership dispatch boundary и read-only members pagination implemented;
  rights/config/uncertainty modeled.
- Modeled: routine administration and moderation boundaries.
- Partial: exact method classification and live matrix.
- Unknown: account-specific available rights.
- Not applicable: payment settlement.
