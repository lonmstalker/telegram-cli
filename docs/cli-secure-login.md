# Защищённый CLI login

`telegram-cli login` возвращает только закрытый `LoginState`, optional challenge ID и typed
`next_action`.
`telegram-cli login tty` запускает owner-operated loop для текущего challenge:

1. CLI читает status из singleton daemon.
2. Телефон, OTP, 2FA, email code и registration names вводятся только через
   `/dev/tty` с отключённым echo.
3. Input связывается с challenge ID и отправляется daemon через private profile socket.
4. Daemon преобразует закрытый `LoginInput` в `AuthorizationInput` существующей
   `AuthorizationMachine`; caller-authored TDJSON `@type` здесь отсутствует.
5. CLI ждёт новый challenge либо доказанный `Ready`.

Для MCP/operator handoff используется one-shot форма:

```sh
telegram-cli login tty <challenge_id>
```

Она до prompt сверяет текущий ID, вводит ровно один относящийся к нему secret и возвращает
`LoginSubmitted`; stale ID fail closed. Сам ID не является секретом. Следующий challenge
снова читает MCP через `auth.wait/status`, поэтому один operator prompt нельзя незаметно
переназначить на изменившееся состояние.

Secret не принимается command-line argument, stdin, environment variable или machine
output. `/dev/tty` открывается как character device с `O_NOFOLLOW`, `O_CLOEXEC` и
`O_NONBLOCK`; echo восстанавливается RAII guard-ом при success, error, SIGINT и SIGTERM.
Временные input/frame buffers zeroize при drop, а `Debug` для protected input всегда
redacted. Closed client errors различают недоступный и сломанный secure TTY, не включая
значение или произвольный error text.

Повторная отправка того же challenge запрещена до нового authorization update. Stale ID,
input другого типа и concurrent submission дают закрытый command error. QR challenge не
печатает ссылку: CLI показывает на TTY только просьбу подтвердить уже начатый QR login на
другом устройстве; one-shot форма возвращает status, а loop-форма продолжает ждать transition.

`LoginSubmitted` означает только принятую TDLib request. Terminal completion по-прежнему
требует `authorizationStateReady`, успешный `getMe` и проверку expected identity в daemon.
Первый live phone/OTP/2FA login этим пунктом не выполнялся; он остаётся live-acceptance
границей P10.
