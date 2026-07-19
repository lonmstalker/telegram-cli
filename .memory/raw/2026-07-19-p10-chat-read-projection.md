# P10 chat read projection — sanitized live checkpoint

- Date: 2026-07-19 (Europe/Moscow).
- Runtime: current-worktree `telegramd`/`telegram-cli`, existing encrypted regular-user profile.
- Authorization: returning login reached terminal `Ready`; no phone/OTP/2FA input was requested.
- Discovery: runtime workflow list contained `resolve_chat`, `preview_invite_link` and
  `inspect_chat` from the freshly built daemon.
- CHAT-001: main list reached `complete=true`, `all_chats_loaded` after 2 load calls; 11 positions
  matched 11 compact entries. Entry keys were exactly `chat_id`, `is_pinned`, `kind`, `order`,
  `title`; forbidden payload keys were absent.
- CHAT-003: an existing public channel was inspected by ID with `open=false`; result and root were
  complete, `used_open_lease=false`, visibility was `public`, and no raw chat/full-info canary keys
  were serialized.
- CHAT-004: the inspected channel's projected canonical public URL resolved to the exact same chat
  ID; both root and result were complete. URL, username, title and ID were not printed or stored.
- Cleanup: read lease was explicitly released; daemon then reached `Draining -> Closed`.
- Boundary: no join, send, read-state mutation or open was requested. CHAT-005 remains pending:
  no disposable invite token was available, and a private invite was not extracted from full info
  merely to satisfy the test.
