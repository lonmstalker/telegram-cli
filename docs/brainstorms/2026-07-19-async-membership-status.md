---
date: 2026-07-19
topic: async-membership-status
---

# Асинхронный статус вступления

## Что строим

`ensure_membership` завершает только отправку join-запроса и сразу возвращает typed receipt.
`request_pending` является успешным известным результатом отправки, но не доказательством
membership. Отдельный read-only `membership_status` получает актуальный статус по chat ID или
invite link и подтверждает поздний переход в `member` после TDLib update без повторного join.

## Почему этот подход

Blocking join не подходит для approval, который может занять произвольное время. Отдельный
persistent job manager пока не нужен: singleton daemon уже принимает TDLib updates, reducer
хранит ordered group status, а существующий events watch умеет удерживать lease и сигнализировать
о supergroup changes. Нужны typed receipt и безопасная status projection поверх этих механизмов.

## Ключевые решения

- Receipt различает `submission_complete` и `membership_complete`.
- `request_pending` даёт root `status=ok`, но `membership_complete=false`.
- Receipt не сериализует raw `ChatJoinResult`; invite URL/token также не возвращается.
- `membership_status` имеет только `read` risk, никогда не вызывает join/open/send.
- Status принимает chat ID или invite link; invite сначала безопасно разрешается, а наружу
  возвращается только optional chat ID и closed membership state.
- Late `updateSupergroup` с member-like status должен менять следующий status result на `member`;
  deterministic test доказывает, что join был отправлен ровно один раз.
- Если invite ещё не разрешается в chat ID, status остаётся `unresolved/complete=false`; это не
  превращается в declined или not-found.

## Открытые границы

- Ordinary admin decline может не иметь отдельного current-user TDLib result; без terminal update
  он не угадывается.
- Background notifications вне запущенного daemon/CLI не добавляются этим slice; поздний status
  при следующем запуске выполняет fresh read probe.

## Следующий шаг

Реализовать core receipt/status, daemon workflow discovery, deterministic async regression и
один sanitized live status текущей заявки.
