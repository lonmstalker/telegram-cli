# Разрешение чата и membership

Первый P4 slice разделяет две операции, которые нельзя скрыто смешивать:

- `workflows::resolve` — read-only разрешение `ChatTarget::Id`,
  `PublicUsername` или `PublicLink`; оно вызывает только `getChat` или
  `searchPublicChat`;
- `workflows::preview_invite_link` — отдельный terminal read через
  `checkChatInviteLink`; preview классифицирует `is_public` и текущий access, но не
  вступает в чат;
- `workflows::ensure_membership` — явная reversible mutation для
  `MembershipTarget::ChatId` или `InviteLink`; только здесь разрешены `joinChat` и
  `joinChatByInviteLink`;
- `workflows::membership_status` — отдельный read-only server snapshot по chat ID или invite
  link; он разрешает chat, читает `getBasicGroup/getSupergroup` и применяет preceding TDLib
  updates через response boundary, но никогда не повторяет join;
- `workflows::leave_chat` — отдельная reversible mutation: `leaveChat` считается terminal
  только после более нового reducer status `chatMemberStatusLeft/Banned`; timeout остаётся
  `uncertain` без blind retry, а уже отсутствующий membership не dispatch-ит повторно.

Публичный Rust API не принимает TDJSON и не требует от caller поля `@type`: discriminator
создаётся внутри workflow перед общим schema-validated `td_call`. `resolve` возвращает только
allowlisted `ChatIdentity`, public usernames/link и freshness; raw `chat`, message payload и
draft не сериализуются. Invite preview не возвращает description, member IDs или сам token.
Join receipt различает `Member`, `RequestPending`, `ApprovalRequired`, `Declined` и неизвестный
будущий result. Известный `request_pending` завершает submission (`status=ok`), но сохраняет
`membership_complete=false`; raw `ChatJoinResult` не сериализуется. Поздний
`membership_status=member` является отдельным доказательством membership. Если invite ещё не
разрешается в chat ID, status остаётся `unresolved/complete=false`. TDLib `error` не превращается
в доказанный `not_found`.

Все операции проходят общую generated policy: resolver/preview/status methods имеют `read`,
join/leave methods — `reversible_mutation` и `reconcile`. Invite link не входит в `Debug`
публичных target types и не добавляется в error.

Базовое разделение дополнено cache-independent hydration, link normalization, `openChat` lease и full info в
[`chat-inspection-workflow.md`](chat-inspection-workflow.md). Chat-list engine описан в
[`chat-list-loading.md`](chat-list-loading.md).
