# Разрешение чата и membership

Первый P4 slice разделяет две операции, которые нельзя скрыто смешивать:

- `workflows::resolve` — read-only разрешение `ChatTarget::Id`,
  `PublicUsername` или `InviteLink`; оно вызывает только `getChat`,
  `searchPublicChat` или `checkChatInviteLink`;
- `workflows::ensure_membership` — явная reversible mutation для
  `MembershipTarget::ChatId` или `InviteLink`; только здесь разрешены `joinChat` и
  `joinChatByInviteLink`.

Публичный Rust API не принимает TDJSON и не требует от caller поля `@type`: discriminator
создаётся внутри workflow перед общим schema-validated `td_call`. Wire result остаётся
lossless в `raw`, а поверх него возвращается компактное состояние. Join различает
`Member`, `RequestPending`, `ApprovalRequired`, `Declined` и неизвестный будущий result;
pending не считается complete. TDLib `error` не превращается в доказанный `not_found`.

Обе операции проходят общую generated policy: resolver methods имеют `read`, join methods —
`reversible_mutation` и `reconcile`. Invite link не входит в `Debug` публичных target types и
не добавляется в error.

Этот slice не заявляет cache wait, link normalization, `openChat` lease или full info: они
принадлежат следующему chat-workflow пункту P4 после chat-list engine.
