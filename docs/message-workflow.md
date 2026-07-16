# F009 message workflows

`chat_history` и `search_chat_messages` сохраняют P4 cursor rules: short page не terminal,
date/count/zero cursor доказывают completion, repeated/no-progress cursor возвращает
`complete=false`. Оба route требуют cached chat и блокируются на update gap.

Read-state не меняется по умолчанию. Owned inputs имеют explicit `mark_read`; только
`true` после complete page вызывает `viewMessages` для returned message IDs. Partial page
не создаёт presence/read side effect. Для `has_protected_content` chat message `content`
заменяется marker с исходным constructor и `content_redacted=true`; payload не достигает
daemon protocol.

`send_text_message` принимает только `chat_id` и plain text. Core сам строит generated-
validated `formattedText/inputMessageText/sendMessage` request, поэтому caller не передаёт
TDJSON `@type`. Response message даёт temporary ID; success/failure принимается только по
matching ordered `updateMessageSendSucceeded|Failed`. Timeout до response или terminal
update возвращает `uncertain/complete=false`, и workflow не делает blind resend.

Capability data добавляет только actual consumers `sendMessage` и `viewMessages`;
existing history/search contracts переиспользуются. Edit/delete/forward/reaction/poll и
остальные message methods доступны через universal raw registry, но default-deny до
доказанного runtime contract.

Behavior tests подтверждают short-page/cursor boundaries, explicit view dispatch,
protected-content canary redaction, terminal send и single-dispatch timeout. Live send или
mark-read на реальном аккаунте не выполнялись без disposable target; это P10 boundary.
