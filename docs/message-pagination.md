# History и search pagination

`chat_history` и `search_chat_messages` используют общий `PageOptions`: requested `count`,
optional inclusive `min_date` и TDLib page limit `1..=100`. Вся цепочка bounded одним
deadline и проходит generated `read/safe_read` policy.

History вызывает `getChatHistory` с `from_message_id=0`, затем использует минимальный ID
полученной страницы как следующий cursor. Повторяющийся boundary message дедуплицируется;
короткая page не считается концом и запускает следующий call.

Chat search передаёт только `next_from_message_id`, который вернул
`searchChatMessages`. Пустая/короткая page с новым cursor продолжает chain; cursor `0`
означает documented `Exhausted`. Seen-cursor set обнаруживает immediate и cyclic repeats.

Terminal result различает:

- `Count` — собрано запрошенное число сообщений, `complete=true`;
- `Date` — reverse-chronological stream пересёк `min_date`, `complete=true`;
- `Exhausted` — search вернул cursor `0`, `complete=true`;
- `NoProgress` — history не дал нового ID/cursor или search повторил cursor,
  `complete=false`.

`total_count` не используется как terminal proof: pinned schema называет его approximate.
Messages сохраняются в исходном порядке; дедупликация использует только ID внутри одного
chat workflow. Если cached chat имеет `has_protected_content=true`, поле `content`
заменяется closed marker с исходным constructor и `content_redacted=true`.

Mark-read/presence по умолчанию не выполняются. Только explicit `mark_read=true` после
complete page вызывает один `viewMessages(..., force_read=true)` для реально возвращённых
IDs; partial/no-progress result не создаёт скрытого side effect.
