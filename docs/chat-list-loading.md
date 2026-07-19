# Загрузка chat list

`workflows::load_chat_list` поддерживает `Main`, `Archive` и `Folder(id)` без TDJSON в
caller API. Один absolute deadline ограничивает всю цепочку.

Алгоритм следует pinned `loadChats` contract буквально:

1. вызвать policy-gated `loadChats` с положительным `limit`;
2. применить ordered updates до response boundary именно этого call;
3. после `ok` повторить вызов независимо от числа пришедших chats;
4. считать список terminal только после TDLib error `404` (`AllChatsLoaded`).

`getChats` не используется как источник истины. Canonical cache остаётся в
`StateReducer`: `updateNewChat` и position updates меняют raw `chat.positions` в receive
order. `chat_list_positions` строит минимальный typed view и сортирует его по точному
TDLib правилу `(order, chat_id)` descending; `order == 0` не входит в список. Второй
параллельный индекс не хранится.

Результат сохраняет ordered `positions` для совместимости и дополнительно возвращает
компактные `entries`: `chat_id`, `title`, `kind`, `is_pinned`, `order`. `kind` различает
`private`, `basic_group`, `supergroup`, `channel`, `secret` и fail-open-to-data `unknown`.
В `entries` не попадают `last_message`, message content, photo/file objects, usernames или
полный cached `chat`; поэтому именно этот shape используется для channel inventory и live
регресса. Также результат содержит число `loadChats` calls и documented terminal condition.
Gap/resync и общий freshness envelope принадлежат следующим пунктам P4.
