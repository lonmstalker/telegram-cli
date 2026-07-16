# CLI routes для core workflows

Третий P6 CLI slice использует один discoverable protocol route вместо отдельного parser
для каждой TDLib-операции:

```text
telegram-cli workflow list
telegram-cli workflow describe <workflow>
telegram-cli workflow run <lease_id> <workflow> '<input-json>' ['<approval-json>']
```

Daemon публикует и исполняет все реализованные core workflows:
`user_profile`, `update_profile_name`, `plan_chat_title`, `apply_chat_title`, `resolve_chat`, `ensure_membership`, `load_chat_list`, `inspect_chat`,
`forum_topics`, `set_forum_topic_closed`, `chat_history`,
`search_chat_messages`, `send_text_message`, `supergroup_members`, `chat_statistics`, `resync_after_gap`,
`download_file`, `cancel_download`, `upload_sticker_file`, `start_bot`,
`start_bot_and_wait_reply`, `click_bot_callback`, `open_web_app`.

Каждый run требует matching principal/lease и строит `RawPolicy` на daemon side. Input
strictly deserialized с `deny_unknown_fields` в owned adapter, затем преобразуется в
существующие `telegram_core::workflows` types. CLI не реализует pagination, cache waits,
terminal updates или retry самостоятельно.

Optional approval JSON разрешён только route, который публикует exact plan hash. Сейчас
это `apply_chat_title`; receipt проверяется общим daemon verifier и расходуется внутри
matching `setChatTitle`, а не workflow name.

Основные input shapes следуют core types: chat target — tagged `kind` с `chat_id`,
`username` или `url`; pagination — nested `page { count, min_date, page_limit }`; file
source — tagged `id/remote/local/generated`. `workflow list` является runtime discovery,
а `workflow describe` возвращает machine-readable `input_example` только для выбранного
route, поэтому agent skill не хранит копию каталога.

`open_web_app` выполняет scoped open/wait/close chain. Result содержит только terminal
receipt и `require_same_origin`; sensitive launch URL не сериализуется и не покидает daemon.
Browser handoff будет завершён domain slice F013 без ослабления этого redaction boundary.

Message routes описаны в [`message-workflow.md`](message-workflow.md): history/search
никогда не меняют read-state без `mark_read=true`, а `send_text_message` ждёт matching
send terminal update и не повторяет uncertain dispatch.

File routes описаны в [`file-transfer-workflow.md`](file-transfer-workflow.md); local и
generated source paths принимаются только внутри daemon-owned artifact root.

`user_profile` и `update_profile_name` описаны в
[`user-profile-workflow.md`](user-profile-workflow.md); sensitive profile fields не входят
в output template или result.

`forum_topics` и `set_forum_topic_closed` описаны в
[`forum-topic-workflow.md`](forum-topic-workflow.md). Folder list не имеет отдельного
engine: `load_chat_list` принимает `{"kind":"folder","folder_id":...}` и сохраняет те же
terminal/gap guarantees, что main/archive.

`plan_chat_title`/`apply_chat_title` и raw moderation boundary описаны в
[`chat-administration-workflow.md`](chat-administration-workflow.md).

Bot reply/callback routes описаны в [`bot-testing-workflow.md`](bot-testing-workflow.md);
callback data не является CLI input и выбирается по recorded button position.
