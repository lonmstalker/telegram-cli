# Custom emoji set workflow

F014 не добавляет Rust API на каждый sticker/emoji-метод. Generated registry сохраняет
полную pinned-schema достижимость, а два curated route закрывают проверяемый lifecycle:

```text
upload_sticker_file
plan_custom_emoji_set
apply_custom_emoji_set
```

`plan_custom_emoji_set` и `apply_custom_emoji_set` принимают один tagged input с
`action=create|add|delete`. Caller не передаёт `@type`: `newSticker`,
`stickerTypeCustomEmoji`, format и `inputFileId` формируются внутри core.

Для `create` и `add` нужен `sticker_file_id` из уже завершённого
`upload_sticker_file`. Apply перечитывает `getFile` и не dispatch-ит mutation, пока
`remote.is_uploading_completed` не подтверждён. Выбор/conversion исходного WEBP/TGS/WEBM
остаётся явной границей F010; встроенный converter не добавлен, Q001 harness остаётся
открытым.

## Typed actions

```json
{"action":"create","user_id":1,"title":"Disposable","name":"codex_disposable","format":"webp","sticker_file_id":1,"emojis":"🧪","needs_repainting":false}
{"action":"add","user_id":1,"set_id":1,"name":"codex_disposable","format":"webp","sticker_file_id":2,"emojis":"✅"}
{"action":"delete","set_id":1,"name":"codex_disposable"}
```

Каждая action сначала проходит plan route. Create/add классифицированы как `admin`, delete
как `destructive`; apply принимает только external one-shot approval точного plan hash.
Daemon/CLI не имеют signing key или self-approval path.

Create проверяет доступность имени, выполняет mutation один раз и перечитывает set по ID.
Если response потерян, reconciliation проверяет имя и exact inventory, не повторяя create.
Add сначала проверяет ownership/type/current inventory, затем после success или timeout
перечитывает set; только наличие exact uploaded file даёт `verified`.

Delete сначала доказывает ownership exact `set_id/name`, выполняет один request и считает
cleanup завершённым лишь когда `checkStickerSetName` подтверждает, что имя снова доступно.
Timeout/mismatch возвращают `uncertain`, а не ложный success и не blind retry.

Synthetic lifecycle test покрывает create, lost-response add reconciliation и verified
delete cleanup. Реальная mutation disposable набора и её cleanup относятся к P10 и требуют
явного разрешения владельца непосредственно перед destructive операцией.
