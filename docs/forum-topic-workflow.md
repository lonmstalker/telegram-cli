# F008 chat/folder/topic workflows

P4 уже реализует ключевые chat contracts: `resolve_chat` не вступает в чат,
`load_chat_list` повторяет `loadChats` до documented terminal и одинаково принимает
main/archive/folder list, а optional `inspect_chat(open=true)` всегда парно вызывает
`openChat`/`closeChat`.

P7 добавляет два forum-topic route поверх generated raw API:

- `forum_topics` принимает `chat_id`, query, requested count и page limit. Short page не
  считается концом: workflow следует тройному cursor из `getForumTopics`, удаляет дубли и
  завершает результат только по count, zero cursor или explicit `no_progress`. Последний
  случай возвращает `complete=false`.
- `set_forum_topic_closed` читает `getForumTopic`, не отправляет уже достигнутое desired
  state, вызывает `toggleForumTopicIsClosed` и снова читает topic. Даже после response
  timeout success возможен только при совпавшем server state; иначе receipt остаётся
  `uncertain/complete=false`, без blind retry.

Capability data ревьюит только фактические новые read consumers `getForumTopic` и
`getForumTopics`; существующий admin contract `toggleForumTopicIsClosed` переиспользуется.
Folder CRUD, chat/private/secret creation, Saved/direct topics и остальные topic methods
доступны через universal raw gate, но остаются default-deny до отдельного доказанного
runtime contract. Это не создаёт per-method Rust/CLI family: typed route скрывает `@type`,
а raw `td call` сохраняет его только как обязательный TDJSON discriminator.

Behavior tests подтверждают continuation после short page, repeated-cursor partial result,
desired-state shortcut и reconciliation после timeout. Live folder/forum mutation не
выполнялась без disposable fixture; это P10 boundary.
