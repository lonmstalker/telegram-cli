# Telegram CLI

Rust CLI для работы агентов с обычным Telegram-аккаунтом: один `telegramd` владеет
зашифрованной TDLib-сессией, CLI выполняет проверенные workflows через приватный Unix
socket. Владелец входит один раз; последующие команды используют сохранённый профиль.

Есть поиск и описание чатов, история и поиск сообщений, отправка, файлы, управление
чатами, bot testing и другие workflows. Точный каталог доступен через `workflow list`
и `workflow describe`. Raw TDLib API проверяется по закреплённой схеме; методы без
policy review запрещены по умолчанию. Опциональный MCP использует тот же daemon.

## Установка

**Без сборки — macOS 11+ arm64 (Apple Silicon):**

```sh
curl -fsSL https://github.com/lonmstalker/telegram-cli/releases/latest/download/install-release.sh | sh
```

Устанавливаются **CLI, daemon, готовая TDLib и глобальные скиллы Codex + Claude Code**.
Rust, Cargo, Python, Git и Homebrew не нужны. Загрузчик проверяет SHA-256 архива до
распаковки, затем bundled daemon проверяет закреплённую TDLib. Нужны только `curl`,
`tar` и `shasum`/`sha256sum` — на macOS они уже есть. Проверяемые архивы и установщик
доступны в [GitHub Releases](https://github.com/lonmstalker/telegram-cli/releases).

Бинарники устанавливаются в `~/.local/bin`. Если этот каталог ещё не в `PATH`, добавьте
`export PATH="$HOME/.local/bin:$PATH"` в `~/.zshrc` (или конфигурацию своего shell)
и откройте новый терминал. Затем выполните `telegram-cli setup` один раз.
Скрипт не меняет shell configuration и не запускает вход в аккаунт.

Можно сначала скачать и прочитать установщик, затем выполнить
`sh install-release.sh --version v0.1.2 --skill both`.
`--prefix /absolute/path` меняет корень установки;
`--skill codex|claude|both|none` выбирает скиллы, default — `both`.
Для ручной/offline установки распакуйте bundle и выполните
`./telegram-cli/install.sh --skill both`.

Первый готовый bundle — **только macOS arm64**. Linux x86_64 GNU/glibc поддерживается
исходниками; публикация и проверка готового Linux bundle остаются отдельным шагом.

**Из исходников** (Rust 1.95.0 и Python 3):

```sh
git clone --branch v0.1.2 https://github.com/lonmstalker/telegram-cli.git
cd telegram-cli
./install.sh --native /absolute/path/to/pinned/libtdjson --skill both
```

Если pinned artifact уже есть в `target/tdlib-native`, достаточно
`./install.sh --skill both`. Скрипт собирает только `telegram-cli` и `telegramd`,
двумя jobs, и устанавливает их в `~/.local/bin`; библиотеку — в
`~/.local/lib/telegram-cli`. `--prefix /absolute/path` меняет корень установки.
Добавьте его `bin` в `PATH`, если команда не находится. Shell startup files не меняются.

Rust-сборка не собирает TDLib. Нужен готовый pinned `libtdjson`; случайная версия
из Homebrew/системы не подойдёт. Для получения native artifact см.
[native build script](scripts/build-tdlib-native.py), manifests в
[`vendor/tdlib/native-builds`](vendor/tdlib/native-builds). Native-сборка — отдельный
дорогой шаг, выполняемый явно; Linux собирается на Linux build host.

## Один раз: настройка и вход

Выполняйте в собственном терминале:

```sh
telegram-cli setup
```

CLI запросит API ID/hash с [my.telegram.org](https://my.telegram.org/apps), создаст
случайный database key, запустит daemon и проведёт через текущие шаги Telegram:
phone/QR, код, 2FA при необходимости. API credentials и код не передаются через
аргументы CLI, stdin или чат с агентом. Native-библиотека после установки находится
автоматически; при запуске из checkout CLI спросит её абсолютный путь.

Для входа без QR выполните `telegram-cli login phone`: CLI запросит номер, код и при
необходимости пароль 2FA. Незавершённая QR-попытка отменяется перед вводом номера;
профиль, API ID/hash и database key сохраняются. Если аккаунт уже имеет статус `ready`,
повторный вход не нужен. Продолжить текущий способ входа можно командой `telegram-cli login`.

Профиль находится в `~/.config/telegram-cli/default/`: каталог `0700`, `profile.json`
и `database-key` — `0600`, отдельные `database/` и `files/`. Для другого аккаунта
используйте `--profile work` перед командой. `TELEGRAM_CONFIG_DIR` задаёт абсолютный
корневой каталог профилей. Повторный `setup` продолжает вход и сохраняет прежний ключ.

Уже настроили `.env.local`? Импортируйте только документированные настройки через
защищённый loader, сохраняя существующую DB и ключ:

```sh
scripts/with-env-local.sh -- telegram-cli setup --import-env
```

Импорт нового профиля требует owner TTY; расширенные scopes импортируются только после явного выбора владельца. Неявного запуска из env без сохранённого профиля нет. После импорта агенту loader и credentials не нужны. Подробности owner TTY, QR и challenge
handoff — в [руководстве авторизации](docs/authorization-guide.ru.md).

## Работа агента

```sh
telegram-cli --agent doctor
telegram-cli --agent login
telegram-cli --agent workflow list
telegram-cli --agent workflow describe user_profile
telegram-cli --agent run user_profile '{"target":{"kind":"self"},"include_full_info":true}'
telegram-cli --agent run load_chat_list '{"list":{"kind":"main"},"limit":30}'
```

`--agent` всегда выдаёт один JSON envelope v4 и не читает TTY. `login` в этом режиме
возвращает только состояние и `next_action`. Daemon запускается по необходимости;
после простоя закрывает TDLib через `close`, сохраняя авторизацию. `run` и `call` сами
берут минимальный lease и освобождают его после ответа или ошибки. Default scope —
`read`; разрешения daemon из профиля остаются верхней границей.

```sh
telegram-cli --agent schema search getChat
telegram-cli --agent schema describe getChat
telegram-cli --agent call '{"@type":"getChat","chat_id":123}'
printf '%s' '{"target":{"kind":"self"}}' | telegram-cli --agent run user_profile -
```

Help, version, schema discovery, workflow list/describe и `td preview` работают
**без аккаунта, DB и сети**. `schema version` описывает pin и явно возвращает
`runtime_verified:false`, а не доказательство подключения к Telegram.

Для отправки нужен явно разрешённый `send`: владелец включает его в
`TELEGRAM_RISK_SCOPES` сохранённого профиля, агент запрашивает `--scopes read,send`
перед `run`. Опасные scopes (`admin`, `destructive`, `financial`, `auth_security`)
также требуют внешнего exact-plan approval. Они доступны через продвинутые команды
`session hold`, `workflow run`, `td preview`, `td call`; см. `--help` и
[policy contract](docs/capability-notes.md). Никакие scopes не включаются установщиком.

Всегда проверяйте `status` и поля результата. `partial`, `complete:false`, `gap`,
`pending`, `reconciliation_required` требуют продолжения или действия владельца.
Транспортная ошибка после mutation не доказывает, что операция не произошла:
**не повторяйте её вслепую**. `response_lost` (exit 5) и `response_too_large` (exit 4) могут означать уже выполненную операцию без полученного receipt; нужен reconciliation. Exit 0 означает полученный ответ, включая `partial`.

| Exit | Значение |
|---|---|
| 0 | Получен ответ; проверьте `status` |
| 2 | Неверные аргументы/JSON/profile |
| 3 | Нет профиля, daemon или безопасного транспорта |
| 4 | Команда/policy/lease отклонены |
| 5 | Ошибка protocol/output |
| 6 | Отмена |

## Skill

Установщики поддерживают `--skill codex`, `claude`, `both` или `none`.
У `install-release.sh` default — `both`; у локального `install.sh` — `none`.
Отдельно, для текущего проекта или глобально:

```sh
telegram-cli init                     # .agents/skills/telegram-cli
telegram-cli init --global            # ~/.agents/skills/telegram-cli
telegram-cli init --global --claude   # ~/.claude/skills/telegram-cli
```

Skill учит discovery, выбору workflow, проверке completion и передаче входа владельцу.
Он не содержит API-каталога, credentials или hooks, перехватывающих команды.

## Разработка и проверки

```sh
cargo fetch --locked                 # один раз заполнить dependency cache
python3 scripts/check.py fast        # boundaries, pin, fmt, cargo check
python3 scripts/check.py verify      # + clippy, tests, fake-daemon agent flow
python3 scripts/check.py release     # offline Rust release, jobs=2, без LTO/native build
python3 scripts/test-install.py      # release + pinned native, изолированный HOME
scripts/package-release.sh /absolute/path/telegram-cli-aarch64-apple-darwin.tar.gz
```

Harness запускает проверки последовательно; второй harness в том же checkout
завершается, не конкурируя за сборку. Native/live gates запускаются отдельно.
MCP не входит в default build; его проверка: `cargo test --locked -p telegram-mcp`.

Для release assets используйте имена `telegram-cli-<target>.tar.gz` и
`telegram-cli-<target>.tar.gz.sha256` (sidecar создаётся упаковщиком автоматически).
При публикации также приложите `scripts/install-release.sh` как `install-release.sh`.
Checksum защищает от повреждения архива; доверие к релизу обеспечивается GitHub/HTTPS,
отдельной криптографической подписи пока нет.

Архитектура: CLI → `telegram-client` → Unix socket → `telegramd` → `telegram-core`
→ TDLib. У CLI нет зависимости от core/TDLib. Один daemon сериализует workflows
одного аккаунта; параллельные долгие операции могут ждать и достигнуть client timeout.
IPC framing неблокирующий, с deadline и ограниченными буферами.

История/поиск возвращают не более 1000 сообщений за вызов; это bounded page workflow,
не streaming export. Event queue ограничена 1024 событиями/16 MiB сериализованного JSON;
переполнение останавливает transport и daemon; следующая команда запускает его заново с той же DB. Unknown-update retention — 1024/8 MiB,
eviction отмечает gap; `resync_after_gap` восстанавливает доступный state, но не историю
потерянных событий. Bot test при gap не выдаёт успешный terminal proof.

`logOut`/`destroy` и удаление профиля сбрасывают доступ; обычное завершение — `close`.
Причина неудачного старта остаётся в `daemon.log` профиля (`0600`, ограничение при открытии 64 KiB); `doctor` показывает путь, но не читает лог. Не передавайте raw log агенту. Перед переустановкой дождитесь idle `Closed`: смешанные версии активного daemon и CLI не являются поддержанным upgrade flow. Не удаляйте DB/ключ для исправления ошибки входа. Backup делайте после `Closed`;
автоматические upgrades/rollback, systemd/launchd и Linux release bundle остаются
отдельной работой. Статусы: [plans.md](plans.md), [HARNESS.md](HARNESS.md),
[live regression](docs/live-regression.md), [English guide](docs/user-guide.en.md).
