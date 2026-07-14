# HARNESS.md — проектный профиль feature-logic-harness

Профиль адаптирует структуру `HARNESS.md` из `tg-analytics` для продукта Telegram Agent CLI. Он определяет канонические фичи, общие инварианты и правила ведения harness-файлов.

## Actors

- AI-агент: выполняет read workflows и разрешённые операции через CLI или MCP.
- Владелец аккаунта: проходит авторизацию, задаёт scopes и подтверждает опасные действия.
- Оператор/deployer: устанавливает runtime, управляет профилями, ключами, обновлениями и наблюдаемостью.
- Система: daemon, TDLib core, update reducer, scheduler, policy и audit.
- Browser harness: проверяет Mini App UI после безопасного Telegram-side handoff.

## Что считается фичей

Фича — самостоятельная возможность продукта, видимая агенту, владельцу аккаунта или оператору. Один постоянный harness-файл соответствует одной фиче в `docs/feature-logic-harness/<slug>.md`. Повторная работа обновляет существующий файл; временные датированные срезы не создаются.

Implementation enablers — конкретный crate, IPC library, Prometheus exporter или code generator — сами по себе не являются фичами. Они входят в contracts/invariants владеющей фичи.

Новую фичу сначала добавляют в канонический inventory ниже, затем создают harness-файл. ID и внутренние `SRC/C/I/D/SC/A/Q` append-only: их нельзя перенумеровывать или переиспользовать.

## Определение полной поддержки TDLib

Названные пользователем сценарии — приоритетные workflows, а не исчерпывающий scope. Полная поддержка означает:

- все functions закреплённого `td_api.tl` присутствуют в generated raw registry;
- все objects, unions, updates и authorization states могут быть провалидированы, маршрутизированы и сохранены без потери неизвестных полей;
- каждый method назначен ровно одному feature owner и имеет risk, retry/idempotency, prerequisite и capability classification;
- core и CLI имеют полный raw coverage; MCP при наличии использует тот же protocol;
- curated workflows отдельно покрывают stateful операции, где одиночный raw call недостаточен.

Bot-only, Premium/Business, admin-gated, financial и official-app-only методы не исключаются из registry. Их runtime-доступность отражается capability/policy результатом.

## Feature inventory

| ID | Slug | Фича | Harness |
|---|---|---|---|
| F001 | `session-lifecycle` | Единая shared TDLib-сессия, leases, запуск и корректное закрытие | [file](docs/feature-logic-harness/session-lifecycle.md) |
| F002 | `authorization` | Авторизация, QR/phone/2FA/email/device branches и database encryption key | [file](docs/feature-logic-harness/authorization.md) |
| F003 | `tdlib-schema-api` | Полный каталог закреплённой TDLib-схемы и generic call | [file](docs/feature-logic-harness/tdlib-schema-api.md) |
| F004 | `state-and-request-chains` | Ordered update cache, prerequisites, pagination, freshness и completeness | [file](docs/feature-logic-harness/state-and-request-chains.md) |
| F005 | `cli` | Полный CLI surface, discovery, JSON/JSONL, events и cancellation | [file](docs/feature-logic-harness/cli.md) |
| F006 | `mcp` | Опциональный локальный и серверный MCP над тем же broker | [file](docs/feature-logic-harness/mcp.md) |
| F007 | `users-contacts-profile` | Пользователи, контакты, профили и идентичность | [file](docs/feature-logic-harness/users-contacts-profile.md) |
| F008 | `chats-folders-topics` | Чаты, lists, folders, Saved Messages, topics и secret chats | [file](docs/feature-logic-harness/chats-folders-topics.md) |
| F009 | `messages-search-interactions` | Сообщения, история, поиск, drafts, reactions, polls и interactions | [file](docs/feature-logic-harness/messages-search-interactions.md) |
| F010 | `files-and-media` | Upload/download, generated files, media и import/export | [file](docs/feature-logic-harness/files-and-media.md) |
| F011 | `groups-channels-moderation` | Membership, invites, permissions, admins, moderation, forums и boosts | [file](docs/feature-logic-harness/groups-channels-moderation.md) |
| F012 | `bots-and-testing` | Bots, inline, callbacks, games и bot testing | [file](docs/feature-logic-harness/bots-and-testing.md) |
| F013 | `mini-apps` | Web Apps и безопасный browser handoff | [file](docs/feature-logic-harness/mini-apps.md) |
| F014 | `stickers-and-emoji` | Stickers, custom emoji, animations и lifecycle наборов | [file](docs/feature-logic-harness/stickers-and-emoji.md) |
| F015 | `stories-calls-live` | Stories, calls, video chats, group calls и live streams | [file](docs/feature-logic-harness/stories-calls-live.md) |
| F016 | `account-settings` | Privacy, notifications, sessions/devices, security и push | [file](docs/feature-logic-harness/account-settings.md) |
| F017 | `business` | Business account, connected/managed bots, quick replies и business messages | [file](docs/feature-logic-harness/business.md) |
| F018 | `payments-and-digital-assets` | Payments, Premium, Stars, gifts, Passport и affiliate programs | [file](docs/feature-logic-harness/payments-and-digital-assets.md) |
| F019 | `statistics-and-resource-management` | Chat/message/revenue statistics, graphs, storage и network resources | [file](docs/feature-logic-harness/statistics-and-resource-management.md) |
| F020 | `platform-utilities` | Localization, links, themes/backgrounds, proxies, logging и test/custom API | [file](docs/feature-logic-harness/platform-utilities.md) |
| F021 | `reliability-policy-observability` | Retry/reconciliation, limits, policy, audit и operational metrics | [file](docs/feature-logic-harness/reliability-policy-observability.md) |
| F022 | `agent-skill-self-discovery` | Компактный agent skill и on-demand discovery без каталога API в контексте | [file](docs/feature-logic-harness/agent-skill-self-discovery.md) |

## Обязательные инварианты продукта

- Ни один method закреплённой схемы не остаётся без feature owner и policy classification.
- CLI, MCP и внутренние автоматизации используют один protocol/core и не дублируют workflow-логику.
- Только daemon открывает TDLib DB; несколько агентов используют leases одной сессии.
- `close` сохраняет авторизацию; `logOut` и `destroy` всегда явно destructive.
- Секреты не входят в model-visible arguments, stdout, logs, metrics, crash reports или документацию.
- State строится из ordered updates; lag/gap делает состояние partial и запускает resync.
- Незавершённая prerequisite chain не может вернуть доказанное `not_found`.
- Короткая history/search page не считается концом без terminal proof.
- Read retry ограничен и учитывает flood/retry delay; неизвестный результат mutation сначала reconciled.
- TDLib commit, version и schema hash закреплены и сверяются с runtime до рабочих запросов.
- Опасные методы поддерживаются схемой, но не обходят policy даже через raw call.
- Mini App UI проверяется браузером; TDLib отвечает только за account-side launch/control.
- Метрики не используют usernames, chat IDs, телефоны или тексты сообщений как labels.

## Dimensions to force

Каждый harness учитывает применимые dimensions:

- first-time / returning / partially completed authorization;
- local / server / remote client;
- user / bot / Business / Premium / admin rights / official-only capability;
- online / offline / slow network / reconnect / flood wait;
- fresh / cached / stale / partial / pending / uncertain / terminal;
- один агент / несколько агентов / crash клиента / crash daemon;
- read / presence / reversible mutation / send / destructive / financial / account-security;
- idempotent / convergent / non-idempotent / unknown remote outcome;
- complete updates / lagged stream / resync;
- compatible / drifted TDLib runtime schema;
- local path / upload token / remote server file semantics;
- CLI available / MCP disabled / MCP local / MCP remote.

## Контракт feature-файла

Каждый файл следует каноническому шаблону `feature-logic-harness` и содержит как минимум: `Summary`, `Product Context`, `Source Ledger`, `TDLib API Coverage`, `Scope`, `Context Map`, `Actors and Permissions`, `Domain Entities`, `State Model`, `Operations and Data Model`, `Contracts`, `Invariants`, `Dimensions`, `Domain Overlays Used`, `Scenario Cells`, `Assumptions`, `Open Questions`, `Coverage Notes`.

Для TDLib-фич дополнительно обязательны: `Request Graph`, `Completion Proof`, `Cache and Update Semantics`, `Retry and Reconciliation`, `CLI/MCP Exposure`, `Permissions and Account Capabilities`, `Live Verification Boundary`. Сгенерированные списки method/update/type не редактируются вручную.

Первичные файлы имеют `Mode: draft`, пока generated API manifest, implementation contracts и live evidence не закрыли `source_gap`.
