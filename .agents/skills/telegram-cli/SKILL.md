---
name: telegram-cli
description: "Безопасная работа с Telegram через singleton telegramd: session lease, on-demand discovery, curated workflows, universal TDLib call, partial/next_action и Mini App handoff. Используй для любых Telegram/TDLib операций этого проекта."
---

# Telegram CLI

Работай только через `telegram-cli`; не открывай TDLib DB, не запускай второй TDLib owner и
не вызывай native TDLib напрямую. Для machine decisions всегда добавляй `--output json` и
читай только envelope v3 `version/status/data/error`, не human prose.

## Цикл

1. Проверь `telegram-cli --output json login`. Если state не `ready`, следуй typed
   `next_action`, но не передавай phone, OTP, 2FA, database key или Web App init data. Для
   `submit_via_protected_channel` попроси владельца вне model terminal выполнить
   `telegram-cli login tty <challenge_id>`, затем повтори status.
2. Возьми минимальный lease:
   `telegram-cli --output json session hold <scope[,scope...]>`. Сохрани `lease_id` из
   `data.lease` и освободи его в finally через
   `telegram-cli --output json session release <lease_id>`.
3. Сначала вызови `workflow list`, затем `workflow describe <name>` только для подходящего
   workflow. Подставь реальные значения в `input_example` и выполни
   `workflow run <lease_id> <name> '<json>'`. Workflow предпочтительнее raw call: он уже
   владеет resolution, pagination, cache/update waits и terminal proof.
4. Если workflow нет, используй `schema search <terms>` и `schema describe <exact-name>`,
   затем один `td call <lease_id> '<json>'`. `@type` нужен только здесь как TDJSON wire
   discriminator; generated Rust registry проверит method и nested objects. Не изобретай
   wrapper и не угадывай поля.
5. После каждого ответа проверь root `status`. `partial`, `complete=false`,
   `reconciliation_required=true`, `pending`, `gap` или `next_action` не означают
   success/absence: выполни указанный continuation, resync либо operator action. Не делай
   blind retry mutation и не объявляй `not_found` по неполной chain.

## Границы

- Не расширяй scopes и не подделывай approval. `admin/destructive/financial/auth_security`
  требуют внешнего exact-plan approval. На `reconciliation_required` не повторяй exact
  operation; используй typed probe/next action или передай владельцу structured outcome.
- Не запускай `login tty` вместо владельца и никогда не помещай secret в args, stdin,
  output, log или memory.
- Для Mini App TDLib workflow доказывает только Telegram-side launch/control. Передай
  browser step отдельному browser harness; не печатай launch URL/init data и не выдавай
  Telegram receipt за DOM/UI proof.
- Не поддерживай локальный каталог API. Загружай только выбранный workflow descriptor или
  schema symbol и возвращай короткий structured digest с completion evidence.
