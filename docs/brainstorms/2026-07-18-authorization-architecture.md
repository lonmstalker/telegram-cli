---
date: 2026-07-18
topic: authorization-architecture
---

# Устойчивая архитектура authorization

## Что строим

Authorization остаётся одним owner-brokered flow, но получает две явные границы. В `telegramd`
один `AuthorizationCoordinator` владеет core machine, boot-scoped token, resend timing,
pending/uncertain outcome и verified identity readiness от startup до re-auth. В CLI
`LoginDriver` владеет циклом status → prompt → submit/resend → wait/terminal, а socket, TTY и
runtime cancellation/polling подключаются адаптерами.

## Рассмотренные варианты

1. Только разнести существующие функции по файлам. Diff меньше, но дублирование startup broker и
   readiness остаётся; архитектурный риск не закрывается.
2. Единый coordinator плюс тестируемый driver. Это выбранный вариант: меняет ownership, но не
   wire protocol и не расширяет secret/capability boundary.
3. Отдельный async actor/event bus. Даёт больше изоляции, чем нужно текущему sync TDJSON loop, и
   создаёт второй concurrency model без потребителя.

## Ключевые решения

- Production daemon создаёт ровно одну `AuthorizationMachine` внутри coordinator и переносит тот
  же coordinator из startup в `LeaseServer`.
- Только coordinator отвечает, можно ли выдавать leases и какой `AccountKind` доказан. Lifecycle
  выполняет `getMe`/identity proof и сообщает результат coordinator.
- Submit и resend используют один dispatch/outcome path; timeout остаётся `Uncertain`.
- CLI driver не знает про Unix socket или `/dev/tty`; tests управляют broker, prompts и временем
  без sleep и реального TDLib.
- Protocol v4, owner-only prompt/redaction и текущий capability scope не меняются.

## Acceptance

- Нет второй daemon-side auth machine или параллельных `ready/account_kind` полей.
- Startup interactive login, returning login и re-auth используют один coordinator instance.
- Driver tests покрывают multi-step chain, resend, one-shot handoff, cancellation и malformed
  response; production TTY остаётся только owner adapter.
- Workspace tests, clippy `-D warnings`, formatting, boundary и wiki gates зелёные.

## Открытые вопросы

Нет. Platform-specific alternative auth journeys и live expired-code resend остаются отдельными
product/live задачами и не входят в этот behavior-preserving refactor.
