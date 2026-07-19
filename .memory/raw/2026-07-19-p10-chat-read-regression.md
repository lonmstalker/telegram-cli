# P10 read-only chat regression — sanitized live evidence

Date: 2026-07-19
Environment class: local macOS arm64, production Telegram DC, existing encrypted regular-user
profile, current workspace binaries.

## Safety boundary

- `.env.local` existence, mode `0600` and Git ignore были проверены без чтения значений.
- Daemon запускался только через `scripts/with-env-local.sh`; второй TDLib owner не создавался.
- Использовался только `read` lease с TTL 60 секунд.
- Join, open/presence, mark-read, send, admin и destructive requests не выполнялись.
- Chat IDs, usernames, titles, invite links, message content и raw TDLib responses в evidence не
  сохранены.

## Observations

1. Собранный из текущего worktree `telegramd` достиг terminal `Ready`; machine login вернул
   envelope v4 с `status=ok`, `state=ready`, без challenge и с `next_action=ready`.
2. Runtime discovery подтвердил `load_chat_list` и `inspect_chat`; exact descriptors были прочитаны
   до выполнения workflows.
3. Main list завершился `status=ok`, `complete=true`, `terminal=all_chats_loaded` после двух
   `loadChats` calls. Получено 11 compact entries: 4 channel, 1 supergroup и 6 private/service.
4. Archive list завершился тем же terminal после двух calls; positions/entries пусты.
5. Compact main/archive result сохранил равенство `positions.len == entries.len`; во всём result
   отсутствовали `last_message`, message `content`, `remote`, `invite_link` и `username`.
6. До compact refactor один доступный supergroup был прочитан через `inspect_chat(open=false)`:
   full info получен, `complete=true`, `used_open_lease=false`; raw result не сохранялся.
7. Read lease освобождён явно. После zero activity daemon прошёл `Draining -> Closed`.

## Reproduction contract

Canonical commands с `LEASE_ID`/fixture placeholders записаны в
[`docs/live-regression.md`](../../docs/live-regression.md), scenarios AUTH-001 и CHAT-001–003.
Machine success определяется только по root status и scenario-specific completion/terminal fields.

## Boundary

Не проверены live: public/invite resolve, folder list, forum topics, presence `openChat`/`closeChat`,
update gap/resync, history/search, members/statistics и любые mutations. Общая P10 остаётся pending.
