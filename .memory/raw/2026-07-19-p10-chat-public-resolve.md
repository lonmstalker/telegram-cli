# P10 CHAT-004 public-link resolve — sanitized live evidence

Date: 2026-07-19
Environment class: local macOS arm64, production Telegram DC, existing encrypted regular-user
profile, current workspace binaries.

## Safety boundary

- Singleton daemon достиг returning `Ready`; второй TDLib owner не создавался.
- Использовался только `read` lease с TTL 60 секунд.
- Выполнялись curated `inspect_chat(open=false)` и `resolve_chat`; `ensure_membership`, open,
  mark-read, send, admin и destructive requests не выполнялись.
- Default-deny raw method после отказа policy не обходился.
- Chat IDs, usernames, public/invite links, titles, descriptions и raw responses не сохранены.

## Observations

1. Compact CHAT-001 inventory содержал четыре channel fixtures.
2. Read-only inspection всех четырёх завершился `status=ok`, `complete=true`, без open lease.
3. Три независимо найденные публичные ссылки завершили `resolve_chat` с `status=ok`,
   `complete=true`; каждый returned chat ID exact совпал с соответствующим fixture из CHAT-001.
4. Три похожих публичных кандидата не были приняты: они разрешились в другой chat ID либо
   завершились terminal error.
5. Для одного channel публичная ссылка не подтверждена; ссылка похожего по названию публичного
   канала была отвергнута по exact ID mismatch.

## Reproduction contract

Команда с `LEASE_ID`/`PUBLIC_LINK` placeholders и exact terminal checks записана в
[`docs/live-regression.md`](../../docs/live-regression.md), scenario CHAT-004. Успех требует
`status=ok`, `complete=true` и exact chat-ID match с CHAT-001; совпадения названия недостаточно.

## Boundary

CHAT-004 public-link resolve принят для трёх fixtures. Invite preview/join, folder/forum,
presence open/close, history/search, members/statistics и mutations не проверялись. Общая P10
остаётся pending.
