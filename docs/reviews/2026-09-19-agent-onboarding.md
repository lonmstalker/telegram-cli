# Agent onboarding: review and verification

Работа в существующем `lonmstalker/telegram-cli`, база `c6a8d953d22e55740bde9038d3f6a5c2fb41b48d`, ветка `codex/agent-onboarding`.

## Независимое ревью качества кода

Вызван `claude -p --model fable --effort medium` с отключёнными tools/MCP, без сохранения
сессии. Запрос явно включал correctness, Rust readability/idioms, избыточную сложность,
CLI UX, безопасность, границы ресурсов, соответствие документации и тестов поведению.
Модель читала присланный source snapshot и diff, не запускала проверки. Полученные
замечания проверены по коду; исправления затем проверены локальными регрессиями.

| Замечание | Результат |
|---|---|
| IPC deadline расходуется во время чужого workflow | Исправлено: framing отделён от выполнения; время handler компенсируется всем ожидающим клиентам. Регрессия с handler >5 секунд. |
| Проигравший parallel startup ждёт лишь 2 секунды | Исправлено: полный startup deadline сохраняется. Fake daemon с lock и 2.5-second cold start проверяет два CLI одновременно. |
| Неявный env fallback и импорт без owner TTY | Fallback удалён. Новый setup требует TTY до записи; расширенные scopes/key импортируются только после выбора владельца, default read. Сам UID не является sandbox между агентом и владельцем. |
| Event overflow требует resync на мёртвом transport | Проверен lifecycle: disconnect propagates error и завершает daemon. Документация уточнена: restart для transport overflow, resync для retention gap. |
| Connect failure и потеря ответа неразличимы | Исправлено: connect → socket_unavailable; ошибки после dispatch → response_lost. Не считать mutation невыполненной. |
| Oversized response приводит к молчаливому EOF | Исправлено: маленький response_too_large, явно требующий reconciliation. Ошибка настройки одного accepted socket больше не завершает daemon. |
| Старый daemon после upgrade; symlink CLI | Symlink исправлен canonicalize. Поддержанный upgrade требует idle Closed; автоматический mixed-version upgrade/rollback остаётся в P9 и не заявлен реализованным. |
| Ошибка старта теряется; orphan key блокирует setup | Owner-only daemon.log сохраняет диагностику; корректный orphan key переиспользуется после проверки metadata/содержимого. |
| Лишний разбор flags/сложность IPC | Удалены повторный split_output и global any(--agent); разбор централизован. IPC разбит по стадиям; использован общий TTL. Request queue overload отличается от stopped transport. Дополнительные сериализации для учёта bytes остаются bounded implementation, оптимизировать по профилю. |
| Недостающие проверки/нестабильный restart test | Добавлены owner PTY, no-env-fallback, concurrent cold start, stale socket, deadline и oversized receipt. Sleep при остановке заменён наблюдением завершения процесса; installer signal trap завершает выполнение. |

## Usage вызова Claude

- Actual model: `claude-fable-5-1`, effort `medium`.
- Duration: 223.286 seconds API/CLI metadata (224 seconds wall clock).
- Fresh input: 2 tokens; cache creation: 84,704; cache read: 531.
- Всего input с cache: 85,237; output: 17,449, включая 13,091 thinking.
- Всего input + output: **102,686 tokens**.
- `total_cost_usd`: **2.56668275**, `costBasis: list`; это расчёт CLI, не процент subscription usage.

## Проверки и границы

- Rust unit tests default workspace: 181 passed, 3 native/live tests ignored.
- MCP: 6 tests passed; workspace compiles with Rust 1.95.0.
- `scripts/check.py verify`: boundaries, schema pin, fmt, clippy -D warnings, tests,
  fake-daemon integration, shell syntax и immutable wiki checks.
- `scripts/test-agent-cli.py`: offline discovery, protected setup через owner PTY,
  reuse/restart без credentials в agent env, concurrent startup, no replay после потери ответа,
  cleanup и skill install. Реальный Telegram в этих тестах не вызывается.
- Source/bundle installer проверяется `scripts/test-install.py` в отдельном HOME/prefix;
  используется настоящий pinned macOS artifact, авторизация не запускается.
- Linux/native rebuild, новый live login, реальное изменение Telegram-аккаунта,
  автоматический upgrade/rollback и публикация release не выполнялись.

Полная реализация остаётся в существующем workspace: CLI не зависит от TDLib/core,
политики, authorization state machine, identity binding, approvals и idempotency не заменены.
