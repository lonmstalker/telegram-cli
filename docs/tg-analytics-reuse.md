# Граница повторного использования `tg-analytics`

Этот ledger закрывает выборочный перенос P0. Источник проверен как committed snapshot `tg-analytics@e35c54ce213aa170fb0b411eab614485424b3e60`; dirty working tree источника не использовался и не изменялся. Build/test evidence хранится в [raw digest](../.memory/raw/2026-07-15-tg-analytics-reuse-audit.md).

## Что уже перенесено

- Repo-local Karpathy Wiki: перенесена структура журналов и ротации, не продуктовые записи `tg-analytics`.
- Feature Logic Harness: перенесён и адаптирован документальный pattern в `HARNESS.md` и `docs/feature-logic-harness/`; numeric IDs остаются только документацией.
- Secret bootstrap: `.env.example` и `scripts/with-env-local.sh` закрепляют локальный protected-loader contract; значения `.env.local` не переносились в Git, документы или evidence.

Это phase-neutral assets. Они не считаются реализованным TDLib core/CLI/daemon.

## Проверенные behavior contracts и owning phases

| Source evidence | Принятый reusable contract | Куда адаптируется | Почему не копируется сейчас |
|---|---|---|---|
| `telegram-tdlib::TdlibBackend`, `FakeTdlibBackend`, lifecycle tests | Transport seam и deterministic fake для send/receive/close | P1 `telegram-core` | P1 ещё не начат; локальная реализация должна иметь один ordered receive loop и `@extra` correlation |
| `telegram-tdlib` authorization loop | Явная обработка auth states, redacted credential provider, returning-session reuse | P1 `telegram-core` | Terminal proof здесь строже: `Ready` плюс `getMe` identity; secret provider и wrong-key contract принадлежат P1 |
| TDLib/gateway rate-limit tests | Server wait распознаётся структурно, ожидание bounded, retry metadata сохраняется | P5 scheduler/reliability | Нельзя добавлять scheduler до его потребителя |
| Gateway policy/redaction tests | Default-deny, явный approval для writes/bot/WebApp, recursive redaction | P3 capability data и P5 policy | Source allowlist покрывает лишь выбранные методы; здесь policy строится из всей pinned schema и одной таблицы данных |
| `AgentTelegramRequest`/`Response`, JSONL/compact tests | Один semantic protocol для human/JSON/JSONL и compact agent output | P6 CLI поверх `telegram-protocol` | Source CLI сам владеет TDLib process; здесь CLI только клиент singleton daemon |
| WebApp inspect tests | TDLib выдаёт redacted launch spec, raw URL остаётся local-only, UI проверяет browser runner | P7 Mini Apps | Browser/runtime slice не входит в P0 |

Green source tests доказывают поведение кандидатов, но не принимают будущие фазы. Каждый contract заново закрывается локальными acceptance tests своего owner; файлы/crates целиком не копируются.

## Что не переносится

- NATS, PostgreSQL, node orchestrator, collector/analytics jobs, migrations и deployment topology `tg-analytics`.
- Long-lived `telegram-agent-cli serve` как владелец TDLib DB: он противоречит singleton `telegramd`.
- Source raw-method allowlist и ручные DTO как замена generated full-schema registry.
- Source session/config paths как model-visible CLI arguments; secrets поступают только через protected TTY, file descriptor/file secret или OS keychain.
- Source TDLib schema/native snapshot: этот репозиторий имеет собственные exact pins и provenance.

## Принятая account/session model

1. MVP использует один основной regular-user account; дополнительные accounts возможны только как отдельные profiles.
2. Profile связывает ожидаемую Telegram identity, отдельные canonical DB/files paths и secret references; DB одного profile не разделяется между owners.
3. Ровно один `telegramd` владеет canonical DB path под exclusive OS lock. `telegram-cli` и optional `telegram-mcp` используют protocol/leases и никогда не открывают TDLib DB.
4. Обычный restart переиспользует зашифрованную авторизацию. Login доказан только после `authorizationStateReady` и `getMe`, совпавшего с expected identity.
5. `close` — штатная остановка с ожиданием `authorizationStateClosed`; `logOut` и `destroy` — отдельные destructive workflows.
6. Последний lease разрешает idle close только при отсутствии in-flight workflow/watch/job; resident/scheduled mode удерживает daemon явно.

Архитектурная фиксация: `D-20260715-036`. Runtime и multi-client proof начинаются только в P1/P2.
