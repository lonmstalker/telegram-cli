# Пошаговая авторизация Telegram CLI

[English version](authorization-guide.en.md) · [Руководство пользователя](user-guide.ru.md)

## Граница безопасности

Phone, OTP, email и registration data видны при вводе в owner terminal. Перед cloud password CLI спрашивает `Скрыть cloud password? [y/N]:`: Enter/`n` оставляет ввод видимым, `y` выключает echo. Не передавайте authorization values агенту, в чат, flags, stdin, environment, логи или JSON. `telegram-cli login` читает значения напрямую из `/dev/tty` и отправляет их daemon по защищённому локальному каналу.

`challenge_id` не является секретом. Это opaque boot-scoped token текущего шага: он меняется
при каждом authorization update и после restart/profile switch. Token связывает введённое
значение с одним challenge и защищает от stale submission.

## Перед началом

Соберите приложения, настройте `.env.local` и запустите daemon, как описано в [руководстве пользователя](user-guide.ru.md). Один profile должен принадлежать только одному `telegramd`.

Если используется существующая TDLib database, сначала убедитесь, что подключён именно её правильный encryption key. Wrong key требует исправления key reference, а не новой phone authorization.

## Обычная авторизация одной командой

Для пользователя весь flow выполняется одной командой:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login
```

CLI читает текущий state и сам проходит последовательность phone → OTP → 2FA/cloud password → email/registration, если эти шаги потребует Telegram. Между шагами не нужно копировать `challenge_id` или повторно запускать команду. Если server-specified timeout текущего кода уже прошёл и TDLib сообщает следующий способ доставки, CLI сам запрашивает новый код до prompt и ждёт новый challenge. Ввод виден по умолчанию; только для cloud password владелец сам выбирает echo. Команда возвращается при доказанном `ready`, явной ошибке или `Ctrl+C`.

Разделы ниже описывают machine-status, диагностику и one-shot handoff для MCP/operator. Для обычного ручного login они не нужны.

## 1. Получите текущее состояние

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli --output json login
```

Смотрите на `data.state`, `data.challenge_id` и `data.next_action`. Типичный ответ может выглядеть так:

```json
{
  "version": 4,
  "status": "partial",
  "data": {
    "type": "login_status",
    "state": "phone_number",
    "challenge_id": "auth-0123456789abcdef0123456789abcdef-0000000000000001",
    "next_action": "submit_via_protected_channel"
  }
}
```

Строка token здесь только пример. Всегда используйте значение из свежего ответа своего daemon.

Для любого login state кроме `ready` корневой `status` намеренно равен `partial`: авторизация ещё не завершена.

## 2. Выполните `next_action`

| `next_action` | Что делать |
| --- | --- |
| `wait` | Ничего не вводить; немного подождать и снова запросить status. |
| `submit_via_protected_channel` | Обработать один challenge через `login tty <challenge_id>`; registration запросит имя и фамилию. |
| `confirm_other_device` | Подтвердить login в уже авторизованном Telegram client, затем снова проверить status. |
| `ready` | Авторизация доказана; secret input больше не нужен. |
| `restart_daemon` | Штатно перезапустить daemon и снова запросить status. |

Состояние `parameters` обрабатывается daemon из защищённой конфигурации: не вводите API credentials через TTY. Состояния `logging_out`, `closing` и временные переходы обычно требуют `wait`. Состояние `closed` требует перезапуска daemon.

## 3. Введите один challenge через TTY

Подставьте актуальный `challenge_id`:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login tty auth-0123456789abcdef0123456789abcdef-0000000000000001
```

Команда проверит ID до показа prompt, обработает один challenge и вернёт управление. Phone, OTP, email и registration отображаются в owner terminal и остаются в его scrollback. Для cloud password владелец выбирает visible/hidden echo непосредственно перед вводом. Успешная отправка означает только `login_submitted`, а не завершённую авторизацию. Снова выполните команду из шага 1 и обработайте новый state.

Если ID уже устарел, команда завершится fail closed. Не повторяйте старое значение: запросите новый status и используйте новый `challenge_id`.

## 4. Пройдите последовательность до `ready`

Конкретная последовательность определяется Telegram и может включать:

1. `phone_number` или `premium_purchase` — номер аккаунта для текущего phone challenge;
2. `code` — OTP из Telegram/SMS;
3. `password` — Telegram 2FA password, если включён;
4. `email_address` или `email_code` — если Telegram запросил email verification; при разрешении
   state owner prompt также предлагает Apple/Google token, а для email code — явный resend;
5. `registration` — только для создания нового аккаунта; CLI показывает ToS, требует явного
   согласия и отдельно спрашивает, уведомлять ли контакты (default — нет);
6. `ready` — финальное состояние после `getMe` и проверки ожидаемой identity.

В machine/operator flow после каждого secret input заново получайте status. Не предполагайте, что следующий шаг всегда OTP или что `login_submitted` означает успех.

Обычный интерактивный цикл сам перечитывает состояние и запрашивает следующие значения:

```sh
scripts/with-env-local.sh -- target/debug/telegram-cli login
```

Поле TDLib `timeout` — не обещанный срок жизни самого OTP. Оно задаёт, когда разрешено вызвать `resendAuthenticationCode`; resend также требует ненулевой `next_type`. Поэтому daemon хранит время наблюдения текущего code challenge. После timeout основной human flow делает один автоматический resend, ждёт новый challenge и только затем просит свежий код. Если Telegram отклонил код раньше timeout, CLI предлагает ввести его ещё раз; когда timeout уже прошёл, он сначала пытается запросить новый.

Legacy alias `login tty` сохраняется для совместимости. Используйте interactive form только в настоящем терминале. В CI, pipe или session без `/dev/tty` protected input должен завершиться ошибкой, а не перейти на stdin.

## 5. QR login

В `phone_number`/`premium_purchase` CLI предлагает QR вместо номера и отправляет typed
`requestQrCodeAuthentication`. При состоянии `qr_code` owner-only link печатается только в
`/dev/tty`; в machine output/MCP его нет. Откройте ссылку на уже авторизованном устройстве и
ждите `ready`.

## 6. Подтвердите завершение

Финальный ответ должен иметь `status: "ok"`, `state: "ready"`, `challenge_id: null` и `next_action: "ready"`:

```json
{
  "version": 4,
  "status": "ok",
  "data": {
    "type": "login_status",
    "state": "ready",
    "challenge_id": null,
    "next_action": "ready"
  }
}
```

Daemon публикует `ready` только после TDLib `authorizationStateReady`, успешного `getMe` и
проверки ожидаемой account identity. Auth-loss отзывает текущие leases; новый Ready повторяет
identity proof.

## 7. Проверьте returning authorization

Освободите leases, дождитесь штатного `close`, затем запустите тот же profile снова. Он должен вернуться в `ready` без повторного phone/OTP input. Для обычной остановки не вызывайте `logOut` или `destroy`.

## Диагностика

- `secure_tty_unavailable` или `secure_tty_failed` — команда не получила безопасный `/dev/tty`; откройте настоящий интерактивный terminal;
- stale/invalid challenge — повторите `login`, возьмите текущий token и не используйте старый secret;
- `login_code_resend_unavailable` означает, что timeout ещё не прошёл либо Telegram не предоставил `next_type`; текущий код всё ещё можно ввести;
- `login_code_resend_rejected` означает, что запрос нового кода дошёл до TDLib, но Telegram его отклонил; не повторяйте запрос циклически;
- wrong database key — остановите daemon и восстановите правильную ссылку на key вне model-visible каналов;
- `socket_unavailable` — запустите daemon с тем же `TELEGRAM_PROFILE`;
- `runtime_unavailable` — authorization ещё не готова для запрошенной операции;
- `unknown`, неожиданный переход или повторяющийся `partial` — остановитесь и сохраните только sanitized state metadata для диагностики; не копируйте secret input.

## Текущая acceptance-граница

Brokered phone/OTP/2FA flow покрыт contract tests и проверен live: first login достиг `ready`, daemon штатно закрылся, а тот же encrypted profile вернулся в `ready` без повторного phone/OTP. Actual expired-code resend остаётся отдельным live follow-up; остальные сценарии общей P10 также ещё не завершены.
