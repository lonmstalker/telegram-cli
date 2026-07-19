# P10 CHAT-010 async membership status — sanitized live digest

Date: 2026-07-19.

## Scope

- Verify a previously submitted join request without repeating the membership mutation.
- The owner-supplied invite URL/token, chat title and chat ID are absent from this digest.
- Execution used current-worktree binaries, singleton daemon and a read-only lease.

## Runtime discovery and authorization

- Returning authorization reached `ready`.
- `workflow list` exposed `membership_status`; its strict input contract was described before use.
- The status input reused the owner-supplied invite, but the workflow output did not return it.

## Terminal evidence

- Root status: `ok`; root/result complete: true.
- Membership state: `member`.
- Freshness: `server_snapshot`; chat ID present.
- No `ensure_membership`, `joinChat` or `joinChatByInviteLink` call was made in this live check.

## Cleanup and privacy

- Read lease released explicitly.
- Daemon reached `Draining -> Closed`.
- No invite, title, identifier, raw group/chat response or member list was persisted.

## Deterministic verification

- Pending join is a completed submission receipt with `membership_complete=false`.
- A later `updateSupergroup` is applied through the read response boundary; the next status is
  `member` and the original join method count remains exactly one.
- Current TDLib member/admin/creator/restricted/left/banned statuses have closed mappings; unknown
  future status and unresolved invite remain incomplete instead of being guessed.
- Workspace verification before live: 163 passed, 0 failed, 3 ignored; clippy, formatting,
  planning/workspace, skeleton, generated registry, secret-output and diff gates green.
