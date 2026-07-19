# Live-регресс Telegram Agent CLI

Этот файл — canonical журнал реально выполненных пользовательских сценариев. Он отвечает на
два вопроса: что именно проверялось на настоящей авторизованной TDLib-сессии и как повторить
проверку без догадок. Synthetic/unit tests сами по себе не переводят сценарий в `passed`.

## Правила ведения

- ID сценария и его смысл не меняются; новый вариант получает новый ID.
- `passed` требует свежий terminal proof. `partial`, `pending`, `uncertain`, `next_action` и
  незавершённая pagination/request chain не считаются успехом.
- В Git не записываются phone, OTP, 2FA, database key, chat IDs, usernames, invite links,
  тексты сообщений, remote file IDs или raw private responses.
- Live evidence хранит только дату, account/profile class, агрегаты, terminal boundary и
  точные команды с placeholders.
- Mutating, presence, admin, destructive, financial и account-security сценарии выполняются
  только с подходящим scope и отдельным разрешением. `close` допустим; `logOut`/`destroy` — нет.
- После прогона lease освобождается явно, а daemon должен штатно пройти `Draining -> Closed`.

## Общая подготовка

Проверить protected local config, не читая его значения:

```sh
test -f .env.local
stat -f '%Lp' .env.local
git check-ignore -v .env.local
cargo build --locked -p telegramd -p telegram-cli
```

Ожидается mode `600`, Git ignore и успешная сборка. Затем в отдельном терминале:

```sh
scripts/with-env-local.sh -- target/debug/telegramd
```

Все machine-команды ниже выполняются через protected loader. В каждом JSON-ответе сначала
проверяется корневой `status`; затем scenario-specific `complete`, `terminal` и `next_action`.

## AUTH-001 — Returning authorization

Статус: `passed` (2026-07-19).

Цель: доказать, что существующая encrypted session достигает verified `Ready` без повторного
ввода phone/OTP/2FA.

Воспроизведение:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json login
```

Ожидается `status=ok`, `data.type=login_status`, `data.state=ready`,
`data.challenge_id=null`, `data.next_action=ready`. Любой другой state оставляет сценарий
`partial` и требует typed owner action; secret нельзя передавать через args/stdin/output.

Evidence 2026-07-19: returning daemon дважды достиг `Ready`; machine login вернул exact
terminal shape выше. Первый запуск затем штатно завершился по idle `Draining -> Closed`.

## CHAT-001 — Полный main chat list

Статус: `passed` (2026-07-19).

Цель: загрузить main list до documented TDLib terminal и получить компактный inventory без
message/file payload.

Воспроизведение:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session hold read 60000
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe load_chat_list
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow run LEASE_ID load_chat_list '{"list":{"kind":"main"},"limit":100}'
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session release LEASE_ID
```

Ожидается `status=ok`, `workflow=load_chat_list`, `complete=true`,
`result.terminal=all_chats_loaded`, `load_calls>=1`, одинаковое число `positions` и `entries`.
Каждый `entry` содержит только `chat_id`, `title`, `kind`, `is_pinned`, `order`; ключи
`last_message`, `content`, `remote`, `invite_link` и `username` отсутствуют во всём result.

Evidence 2026-07-19: terminal достигнут за 2 `loadChats` calls; в main list 11 entries:
4 `channel`, 1 `supergroup`, остальные 6 — private/service chats. Названия и IDs в Git не
сохраняются.

## CHAT-002 — Полный archive chat list

Статус: `passed` (2026-07-19).

Воспроизведение: повторить CHAT-001 с input
`{"list":{"kind":"archive"},"limit":100}`.

Ожидается тот же terminal contract. Пустой archive является успешным только при
`terminal=all_chats_loaded`, а не по одной короткой/пустой странице.

Evidence 2026-07-19: terminal достигнут за 2 `loadChats` calls; `positions=[]`, `entries=[]`.

## CHAT-003 — Read-only inspection без open/join

Статус: `passed` (2026-07-19), compact projection повторно проверен live.

Цель: проверить доступный chat по ID и получить full info без `openChat`, membership mutation
или read-state mutation.

Воспроизведение:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe inspect_chat
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow run LEASE_ID inspect_chat '{"target":{"kind":"id","chat_id":CHAT_ID_FROM_CHAT_001},"open":false}'
```

Ожидается `status=ok`, `complete=true`, `result.complete=true`,
`result.used_open_lease=false`, `result.full_info_kind` соответствует типу чата. Во всём result
отсутствуют `last_message`, `draft_message`, `description`, `invite_link`, `member_user_ids` и
raw TDLib constructors full info; для перечня каналов используется CHAT-001.

Evidence 2026-07-19: один реальный supergroup проверен; `complete=true`, full info получен,
`used_open_lease=false`, join/open/send не вызывались.

Refactor evidence 2026-07-19: public channel повторно проверен по ID; root/result complete,
`visibility=public`, `used_open_lease=false`; raw chat/full-info и запрещённые поля в JSON
отсутствуют. Inspection успешно использует прямой `chat` response без обязательного
`updateNewChat` (эта ветка дополнительно закрыта deterministic backend test).

## CHAT-004 — Public link resolve без join

Статус: `passed` (2026-07-19).

Цель: разрешить выбранную публичную ссылку в уже известный channel без вступления, открытия
чата или изменения read-state.

Воспроизведение:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session hold read 60000
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe resolve_chat
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow run LEASE_ID resolve_chat '{"kind":"public_link","url":"PUBLIC_LINK"}'
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session release LEASE_ID
```

Ожидается `status=ok`, `complete=true` и `result.chat.chat_id`, совпадающий с ID того же
чата из CHAT-001. Совпадение только названия не является proof. Для публичной ссылки не должен
вызываться `ensure_membership`; ошибка или другой chat ID не засчитываются. Дополнительно
`result.chat.visibility=public`, а `canonical_public_url` строится только из public username.

Evidence 2026-07-19: три независимо найденные публичные ссылки разрешились в три точных channel
из CHAT-001; ещё три похожих кандидата были отклонены по другому chat ID или terminal error.
Один оставшийся channel не имеет подтверждённой публичной ссылки. Выполнялся только
`resolve_chat` под `read` lease; join/open/send не вызывались. URL, usernames и IDs в Git не
сохранены.

Refactor evidence 2026-07-19: canonical public URL из compact identity повторно разрешён в exact
тот же chat ID; `result.chat.visibility=public`, root/result complete, raw/description/invite
поля отсутствуют, lease освобождён явно.

## Следующие chat-сценарии

| ID | Сценарий | Статус | Условие live-прогона |
|---|---|---|---|
| CHAT-005 | Invite preview без membership | pending | Нужен disposable invite fixture; terminal `complete=true`, visibility берётся из `is_public`, token/description/member IDs не возвращаются. |
| CHAT-006 | `openChat`/`closeChat` pairing | pending | Нужен `presence` scope и безопасный fixture; cleanup обязателен при success/error/cancel. |
| CHAT-007 | Folder list | pending | Нужен существующий folder ID; terminal contract тот же, что main/archive. |
| CHAT-008 | Forum topics pagination | pending | Нужен доступный forum fixture; short page не terminal, repeated cursor остаётся partial. |
| CHAT-009 | Update gap и resync | pending | Нужен контролируемый gap; complete запрещён до успешного `getCurrentState` resync. |

History/search относятся к F009 и будут добавлены следующим slice отдельно: они не должны
неявно смешиваться с chat-list inventory или менять read-state без `mark_read=true`.
