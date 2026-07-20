# P10 CHAT-006 open/close acceptance

- Date: 2026-07-21 (Europe/Moscow).
- Scope: returning owner session, current-worktree `telegramd`/`telegram-cli`, один public
  supergroup fixture из terminal main chat list.
- Privacy: fixture ID/title, TDLib raw response и private/message payload не выводились и не
  сохранялись.

## Preflight

- `.env.local` существует, имеет mode `0600` и Git-ignored; значения не читались.
- Owner ceiling в файле не менялся. Только live daemon process получил явный минимальный
  non-secret override `TELEGRAM_RISK_SCOPES=read,presence` после protected loader.
- `cargo build --locked -p telegramd -p telegram-cli` завершился успешно.

## Sanitized live trace

```text
stage=login_ready
stage=workflow_discovered
stage=workflow_described
stage=lease_acquired
stage=fixture_selected
stage=paired_cleanup_acked
stage=lease_released
{"version":4,"status":"ok","workflow":"inspect_chat","complete":true,"result_complete":true,"used_open_lease":true,"full_info_kind":"supergroup","visibility":"public"}
telegramd: Draining
telegramd: Closed
```

Root/result `complete=true` и `used_open_lease=true` появляются только после correlated
`closeChat` `ok`; поэтому live success доказывает `openChat -> full info -> closeChat` pairing.
Join, send, message history и raw TDLib bypass не выполнялись.

## Deterministic failure evidence

До исправления два новых regressions были red: response timeout `openChat` не отправлял cleanup,
а full-info timeout оставлял `closeChat` response непрочитанным из-за общего истёкшего deadline.
После исправления `cargo test -p telegram-core chat_inspection_ -- --nocapture` даёт четыре
passed cases: success pairing, full-info TDLib error, timeout ответа `openChat` с одним
compensating close и full-info timeout с cleanup acknowledgement. В обеих timeout-ветках
`closeChat` ACK consumed и не остаётся unmatched; поздний response исходного timed-out request
этот backend не моделирует.
