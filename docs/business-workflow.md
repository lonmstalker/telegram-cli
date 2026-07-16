# Telegram Business workflow

F017 оставляет полный Business surface в generated registry и добавляет два curated route
для самого рискованного connection-scoped пути:

```text
business_connection
send_business_text
```

Оба требуют explicit `connection_id`; safe default и cross-connection cache отсутствуют.
`business_connection` возвращает только enabled/read/reply capability. Daemon определяет
`regular_user` или `bot` из готового TDLib `getMe` object и применяет generated account
scope до dispatch, поэтому bot-only API не скрывается и не открывается regular account.

`send_business_text` сначала перечитывает exact connection и не dispatch-ит при disabled
connection или без `can_reply`. В result нет текста сообщения или customer metadata. При
успехе сверяются chat/message ID. При timeout выполняется только
`getBusinessConnection`: исчезнувшее право даёт `capability_lost`, сохранившееся —
`uncertain`; повторного send нет.

Synthetic SC001 использует два connection с одинаковым chat ID и проверяет, что каждый
request сохраняет свой explicit ID. SC002 теряет response, затем видит disconnect и
подтверждает ровно один send. Live Business entitlement остаётся P10/Q001.
