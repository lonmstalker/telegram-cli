# P10 CHAT-010 membership round-trip — sanitized live digest

Date: 2026-07-19.

## Scope

- Owner explicitly requested a leave followed by a rejoin through an owner-supplied invite.
- The invite URL/token, chat title and chat ID are intentionally absent from this digest.
- Execution used the singleton daemon, current-worktree CLI and typed workflows only.
- The process-only owner ceiling was limited to `read,reversible_mutation`; `.env.local` was not
  edited and the generated capability policy remained authoritative.

## Discovery and authorization

- Returning authorization reached `ready` before any workflow.
- Runtime discovery confirmed `preview_invite_link`, `leave_chat` and `ensure_membership` input
  contracts.
- The target preview was complete and classified as an accessible non-public channel with a chat
  ID present.

## Terminal evidence

- `leave_chat`: root complete, receipt complete, outcome `verified_left`.
- A follow-up invite preview remained complete and accessible; this was not treated as membership
  proof.
- `ensure_membership`: root partial, state `request_pending`.
- Therefore the leave is proven, the join request is proven, but final membership is not proven
  and CHAT-010 remains partial pending administrator approval.

## Safety and cleanup

- No blind join retry and no chat-ID/raw TDLib bypass followed the pending result.
- The bounded lease was released explicitly.
- Daemon shutdown reached `Draining -> Closed`.
- No invite, title, identifier, raw response, description or member list was persisted.

## Deterministic verification

- `leave_chat` has tests for ordered `left` update, idempotent already-left handling and uncertain
  timeout without retry.
- Workspace tests: 158 passed, 0 failed, 3 ignored.
- Workspace clippy with `-D warnings`, formatting, planning/workspace boundaries, secret-output
  canary and diff check were green before the live run.
