# Руководство пользователя Telegram CLI

[English version](user-guide.en.md) · [Пошаговая авторизация](authorization-guide.ru.md)

## Статус руководства

Это руководство описывает запуск из исходного checkout. Готовая установка, systemd/launchd и обновления относятся к P9 и пока не заявлены как завершённые. Первая авторизация с phone/OTP/2FA и повторный запуск того же зашифрованного профиля проверены live; остальные сценарии P10 ещё не завершены.

## Что запускается

`telegramd` — единственный процесс, который владеет TDLib database выбранного профиля. `telegram-cli` подключается к его приватному Unix socket и не открывает database напрямую.

Основные термины:

- **profile** — локальная конфигурация и отдельная TDLib session;
- **daemon** — процесс `telegramd`, владеющий session;
- **lease** — ограниченное по времени разрешение на операции с заданными scopes;
- **workflow** — проверенный сценарий более высокого уровня;
- **raw call** — прямой TDLib request, который всё равно проходит generated validation и default-deny policy.

## 1. Подготовьте checkout

Нужны Rust toolchain и pinned native `tdjson` artifact, соответствующий текущему checkout. Из корня репозитория соберите приложения:

```sh
cargo build --locked -p telegramd -p telegram-cli
```

Если `.env.local` ещё нет, создайте его из примера и сразу ограничьте права:

```sh
cp .env.example .env.local
chmod 600 .env.local
```

Не перезаписывайте уже существующий `.env.local`. Заполните параметры по комментариям в `.env.example`. В частности, нужны Telegram API credentials, пути профиля и TDLib database. Файл, указанный в `TDLIB_DATABASE_KEY_FILE`, должен содержать Base64-encoded database key и иметь mode `0600`.

Phone, OTP, 2FA password, email code и registration data нельзя добавлять в `.env.local`, аргументы команд, stdin или логи. Они вводятся только через protected TTY flow.

## 2. Запустите daemon

В первом терминале:

```sh
scripts/with-env-local.sh -- target/debug/telegramd
```

Оставьте процесс запущенным. Daemon автоматически начнёт штатное завершение после периода без leases и активной работы.

## 3. Проверьте авторизацию

Во втором терминале:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login
```

CLI сам последовательно запросит phone, OTP, 2FA/cloud password и дополнительные поля, если их потребует Telegram. Для устаревшего code challenge он после server-specified timeout сам запросит новый код, если TDLib разрешает следующий способ доставки. Команда завершится только при доказанном `ready` либо явной ошибке/отмене. Все поля видны при вводе; перед cloud password CLI отдельно предложит скрыть echo.

Для machine-status без интерактивных prompts используйте отдельную форму:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json login
```

В machine flow продолжайте только если корневой `status` равен `ok`, а login `state` равен `ready`. Подробности и operator/MCP handoff — в [инструкции по авторизации](authorization-guide.ru.md).

## 4. Выполните первую read-операцию

Получите read lease:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session hold read 60000
```

Сохраните `lease_id` из ответа. Перед запуском workflow можно запросить его актуальный контракт:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe user_profile
```

Получите профиль текущего пользователя, подставив свой `lease_id`:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow run LEASE_ID user_profile '{"target":{"kind":"self"},"include_full_info":true}'
```

Освободите lease после работы:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json session release LEASE_ID
```

## 5. Читайте machine output правильно

JSON envelope имеет `version: 3` и корневой `status`:

- `ok` — операция завершена с заявленным результатом;
- `partial` — результат неполный или требует reconciliation; это не успех;
- `error` — запрос отклонён или не выполнен.

Нулевой exit code сам по себе не заменяет проверку `status`, `complete`, `next_action` и других полей конкретного ответа. После uncertain mutation не повторяйте запрос вслепую.

## Полезные команды

```sh
# Состояние daemon и leases
scripts/with-env-local.sh -- target/debug/telegram-cli --output json status

# Список и описание workflows
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow list
scripts/with-env-local.sh -- target/debug/telegram-cli --output json workflow describe WORKFLOW

# Версия и поиск по pinned schema
scripts/with-env-local.sh -- target/debug/telegram-cli --output json schema version
scripts/with-env-local.sh -- target/debug/telegram-cli --output json schema search QUERY
scripts/with-env-local.sh -- target/debug/telegram-cli --output json schema describe TD_METHOD

# Preview direct request до отправки
scripts/with-env-local.sh -- target/debug/telegram-cli --output json td preview '{"@type":"getMe"}'
```

Для обычной работы предпочитайте curated workflows. Raw call требует lease, точный `@type` и может быть отклонён policy даже для валидного TDLib method.

## Безопасная остановка

Сначала освободите leases. При отсутствии работы daemon сам выполняет `close`, сохраняет авторизацию и завершается. Не используйте `logOut` или `destroy` для обычной остановки: это destructive operations.

## Частые ошибки

- `socket_unavailable` — daemon не запущен, выбран другой profile или private socket недоступен;
- `runtime_unavailable` — TDLib runtime ещё не достиг состояния, нужного команде;
- wrong database key — остановите daemon и исправьте ссылку на правильный key; не начинайте новую phone authorization поверх существующей database;
- `partial` — выполните указанный `next_action` или reconciliation и не выдавайте ответ за полный;
- lease expired — получите новый lease и заново проверьте состояние перед mutation.

Технические контракты находятся в [`docs/`](.). Начните с [CLI session contract](cli-session.md), [workflow routes](cli-workflows.md) и [secure login contract](cli-secure-login.md).
