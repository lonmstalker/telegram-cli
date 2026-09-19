---
name: telegram-cli
description: "Безопасная работа с Telegram через singleton telegramd: session lease, on-demand discovery, curated workflows, universal TDLib call, partial/next_action и Mini App handoff. Используй при работе с Telegram через установленный telegram-cli: чаты, сообщения, файлы, боты и управление аккаунтом."
---

# Telegram CLI

Работай только через `telegram-cli`; не открывай TDLib DB, не запускай второй TDLib owner и
не вызывай native TDLib напрямую. Для machine decisions всегда добавляй `--agent` и
читай envelope v4 `version/status/data/error`, не human prose.

## Первый запуск

1. `telegram-cli --agent doctor` проверяет installation/profile без сети и DB.
   `profile_not_configured` → попроси владельца выполнить `telegram-cli setup` в его
   терминале. Для существующего development profile владелец может выполнить
   `scripts/with-env-local.sh -- telegram-cli setup --import-env` из checkout.
   Не читай profile.json, .env.local, database-key и TDLib DB.
2. `telegram-cli --agent login` лениво запускает daemon и возвращает state/next_action.
   Если не `ready`, передай владельцу `telegram-cli login`; для одного конкретного
   challenge — `telegram-cli login tty <challenge_id>`. Повтори status после его входа.
3. Выбирай аккаунт через `--profile NAME` перед командой; не угадывай другой профиль
   после ошибки. Сохранённая авторизация используется при следующих запусках.

## Цикл задачи

1. `telegram-cli --agent workflow list`, затем `workflow describe <name>` только для
   подходящего workflow. Discovery работает без аккаунта/сети.
2. Подставь реальные значения из задачи в `input_example` и выполни:
   `telegram-cli --agent run <name> '<json>'`. Для сложного JSON допустим stdin:
   `... run <name> -`. `run` сам берёт и освобождает read lease.
3. Для side effects нужна явная авторизация пользователя и минимальные scopes:
   `telegram-cli --agent --scopes read,send run <name> '<json>'`. Если daemon отклоняет
   scope, передай владельцу ошибку; не меняй конфигурацию и не расширяй доступ сам.
4. Если workflow нет, используй `schema search <terms>` и `schema describe <exact-name>`,
   затем `call '<json>'`. `@type` нужен как TDJSON discriminator. Не угадывай поля.
5. Проверяй root `status` и результат: `partial`, `complete=false`,
   `reconciliation_required=true`, `pending`, `gap` или `next_action` не означают
   success/absence. Продолжи указанную chain/resync или передай operator action.

## Продвинутый режим

- Для нескольких связанных операций: `session hold <scopes>` → `lease_id` из
  `data.lease` → `workflow run <lease_id> <name> '<json>'` / `td call <lease_id> '<json>'`
  → `session release <lease_id>` в finally. Lease ограничен 60 секундами.
- Для exact-plan approval используй `td preview` и существующие approval arguments
  продвинутых команд. Упрощённые `run/call` не обходят approval gate.
- `events watch <lease_id> [cursor]` поддерживает `--output jsonl`; `--agent` даёт один
  JSON-ответ. На gap используй `resync_after_gap`; lost events нельзя считать проверенными.
- История/поиск: count 1..1000, page_limit 1..100. Workflow начинает выборку с последних сообщений; большой streaming export пока не реализован.
  Не увеличивай count сверх лимита и не объявляй ограниченную выборку полным архивом.
- Exit 0 включает partial; 2 input, 3 unavailable/config, 4 rejected, 5 protocol/output,
  6 cancelled. После потери ответа на mutation неизвестно, был ли side effect:
  `response_lost` и `response_too_large` также могут означать выполненную операцию без receipt;
  не повторяй вызов, используй reconciliation или оператора. Lease cleanup выполняется
  также на ошибке, при аварии процесса оставшийся lease истекает по TTL.

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
