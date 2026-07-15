# Инструкции для агентов проекта

## Язык и источники истины

- Веди human-facing работу на русском языке. Не переводи технические идентификаторы, команды, schema fields и точные error contracts.
- Используй `plans.md` как living implementation plan, `product.md` как product boundary, `HARNESS.md` как feature inventory, а `.memory/wiki/index.md` как вход в долговечную project memory.
- Не выдавай документационный bootstrap за реализованный Rust core/CLI/MCP.

## Правила работы

- Раздел «Правила работы» в `plans.md` обязателен: размер задачи — пункт Tasks фазы (не один TDLib-метод), схема закреплена одним хешем, никаких self-referential тестов, классификация — данные, default-deny валиден, тесты пропорциональны коду.

## Karpathy Wiki

- Для любой нетривиальной задачи используй `.agents/skills/karpathy-wiki` и сначала прочитай `.memory/wiki/index.md`.
- Загружай только связанные topic/raw pages и последние нужные entries активных журналов; не тащи всю wiki в контекст.
- Гранулярность памяти — завершённый пункт Tasks фазы, не отдельный метод или файл. Одна work-entry на пункт; никаких entries про бухгалтерию ротаций и правку ссылок.
- Храни sanitized immutable evidence в `.memory/raw/` только для внешних доказательств (сборка, сеть, live-сессия) — не для пересказа кода; compact reusable synthesis в `.memory/wiki/`.
- Храни выполнение и checkpoints только в `.memory/logs/work.md`.
- Храни долговечные решения только в `.memory/decisions/decisions.md` с ID `D-YYYYMMDD-NNN`. Долговечное решение — архитектурное, переживающее фазу; вывод про один метод решением не является.
- Храни проблемы, риски и их status transitions только в `.memory/problems/problems.md` с ID `P-YYYYMMDD-NNN`.
- В work log ссылайся на `D/P` IDs; не копируй туда полное тело решения или проблемы.
- Старые entries не переписывай. Correction, supersede, resolution или смену статуса добавляй новой entry с тем же связанным ID.
- Каждый durable claim связывай с raw digest, local file, проверочной командой или явной инструкцией пользователя.
- Если wiki расходится с live repo/runtime, доверяй live evidence, запиши correction и затем обнови synthesis/index.

## Ротация памяти

- После добавления entries запускай `python3 scripts/rotate-wiki-journal.py --all`.
- Active work/decision/problem journal не должен превышать 16 000 Unicode-символов или 1 000 строк.
- Ротация переносит только целые старейшие entries в соответствующий `archive/`, оставляя минимум одну свежую active entry.
- Archive shard и существующая строка его checksum index immutable: не редактируй, не перемещай и не удаляй их вручную.
- Проверяй все три журнала командой `python3 scripts/rotate-wiki-journal.py --all --check`.

## `.env.local` и секреты

- `.env.local` — canonical local development secret source. Файл обязан иметь mode `0600` и быть Git-ignored.
- Используй значения только через `scripts/with-env-local.sh -- <command>` или будущий эквивалентный dotenv loader приложения.
- Имена и назначение переменных бери из `.env.example`; не открывай `.env.local` редактором и не читай его через `cat`, `sed`, `rg`, `grep`, `awk` или похожие команды.
- Не запускай для загруженного окружения `env`, `printenv`, `set`, `export -p`, shell tracing (`set -x`) или debug dump, способный раскрыть значения.
- Разрешены только проверки существования файла, mode, `git check-ignore` и запуск loader без вывода окружения.
- Не переноси значения `.env.local` в prompt, stdout/stderr, wiki, logs, decisions, problems, docs, tests, crash reports, artifacts или Git.
- Phone, OTP, 2FA, Passport и Web App init data не являются обычными env-переменными; передавай их через защищённый interactive/brokered flow.

## Безопасность TDLib

- Только будущий singleton daemon может владеть одной TDLib DB; CLI/MCP не должны открывать её напрямую.
- `close` — штатная остановка с сохранением авторизации. `logOut` и `destroy` всегда destructive.
- Неполная request/update chain не может давать доказанный `not_found` или `complete`.
