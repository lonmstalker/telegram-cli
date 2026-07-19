# P10 CHAT-005 invite preview — sanitized live checkpoint

- Date: 2026-07-19 (Europe/Moscow).
- Fixture: disposable invite link supplied explicitly by the owner; URL/token, title and chat ID
  were not printed or stored.
- Runtime: current committed `telegramd`/`telegram-cli`, existing encrypted regular-user profile.
- Authorization: returning login reached terminal `Ready`; no owner secret input was requested.
- Discovery: `preview_invite_link` was present in the runtime workflow list and its strict input
  descriptor was loaded before execution.
- Execution: one `read` lease and one successful preview call. Root and result were
  `complete=true`; TDLib classified the fixture as `channel`, `visibility=non_public`, current
  `access=accessible`, non-null chat ID, zero temporary-access timer and no join request.
- Projection: result keys were limited to `access`, `accessible_for`, `chat_id`, `complete`,
  `creates_join_request`, `kind`, `title`, `visibility`. Description, member IDs and invite link
  were absent.
- Safety: only the preview workflow was invoked; membership, join, open and send workflows were
  not requested. Deterministic method-dispatch tests prove the preview path calls only
  `checkChatInviteLink`.
- Cleanup: read lease was explicitly released; daemon then reached `Draining -> Closed`.
