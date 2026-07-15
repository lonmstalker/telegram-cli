# CLI routes для core workflows

Третий P6 CLI slice использует один discoverable protocol route вместо отдельного parser
для каждой TDLib-операции:

```text
telegram-cli workflow list
telegram-cli workflow describe <workflow>
telegram-cli workflow run <lease_id> <workflow> '<input-json>'
```

Daemon публикует и исполняет все реализованные P4 workflows:
`resolve_chat`, `ensure_membership`, `load_chat_list`, `inspect_chat`, `chat_history`,
`search_chat_messages`, `supergroup_members`, `chat_statistics`, `resync_after_gap`,
`download_file`, `upload_sticker_file`, `start_bot`, `open_web_app`.

Каждый run требует matching principal/lease и строит `RawPolicy` на daemon side. Input
strictly deserialized с `deny_unknown_fields` в owned adapter, затем преобразуется в
существующие `telegram_core::workflows` types. CLI не реализует pagination, cache waits,
terminal updates или retry самостоятельно.

Основные input shapes следуют core types: chat target — tagged `kind` с `chat_id`,
`username` или `url`; pagination — nested `page { count, min_date, page_limit }`; file
source — tagged `id/remote/local/generated`. `workflow list` является runtime discovery,
а `workflow describe` возвращает machine-readable `input_example` только для выбранного
route, поэтому agent skill не хранит копию каталога.

`open_web_app` выполняет scoped open/wait/close chain. Result содержит только terminal
receipt и `require_same_origin`; sensitive launch URL не сериализуется и не покидает daemon.
Browser handoff будет завершён domain slice F013 без ослабления этого redaction boundary.
