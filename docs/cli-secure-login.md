# Защищённый CLI login

Обычный human-вызов `telegram-cli login` запускает owner-operated loop и сам проходит все
текущие challenges до terminal state. Machine-вызов `telegram-cli --output json login`
возвращает только закрытый `LoginState`, optional opaque challenge token и typed `next_action`, не
показывая prompts. Legacy alias `telegram-cli login tty` запускает тот же human loop.

Цикл реализован отдельным `LoginDriver<Broker, Prompter, Runtime>`. Driver знает только закрытые
protocol requests/responses и переходы status → prompt → submit/resend → wait/terminal. Unix
socket, owner `/dev/tty`, cancellation и polling clock подключены адаптерами; deterministic tests
проходят multi-step chain, exact one-shot handoff, cancellation и malformed prompt без реального
TDLib, socket, TTY или sleep.

Интерактивный цикл:

1. CLI читает status из singleton daemon.
2. Телефон, OTP, 2FA, email code и registration names вводятся только через
   `/dev/tty`. Phone/OTP/email/registration видны по умолчанию. Перед cloud password
   владелец выбирает echo: Enter/`n` — visible, `y` — hidden.
3. Input связывается с opaque boot-scoped challenge token и отправляется daemon через private
   profile socket. Token меняется при каждом authorization update и после restart/profile switch.
4. Daemon преобразует закрытый `LoginInput` в `AuthorizationInput` существующей
   `AuthorizationMachine`; caller-authored TDJSON `@type` здесь отсутствует.
5. Для code challenge daemon учитывает TDLib `timeout` от момента наблюдения состояния. Если
   timeout прошёл и `next_type` существует, human loop один раз вызывает typed
   `resendAuthenticationCode`, ждёт новый challenge и только затем показывает OTP prompt.
6. Email-code challenge разрешает явный owner-requested resend без phone-code timeout.
7. CLI ждёт новый challenge либо доказанный `Ready`.

Для MCP/operator handoff отдельно используется one-shot форма:

```sh
telegram-cli login tty <challenge_id>
```

Она до prompt сверяет текущий ID, вводит ровно один относящийся к нему secret и возвращает
`LoginSubmitted`; stale token fail closed. Сам token не является секретом. Следующий challenge
снова читает MCP через `auth.wait/status`, поэтому один operator prompt нельзя незаметно
переназначить на изменившееся состояние.

Ни phone, ни secret не принимаются как command-line argument, stdin, environment variable
или machine output. Phone echo разрешён только в owner terminal и остаётся в его scrollback;
CLI/daemon его отдельно не печатают и не логируют. `/dev/tty` открывается как character
device с `O_NOFOLLOW`, `O_CLOEXEC` и `O_NONBLOCK`; для выбранного hidden password echo восстанавливается
RAII guard-ом при success, error, SIGINT и SIGTERM.
Временные input/frame buffers zeroize при drop, а `Debug` для protected input всегда
redacted. Closed client errors различают недоступный и сломанный secure TTY, не включая
значение или произвольный error text.

После dispatch outcome различается как `NotSent`, `DefinitiveRejected` или `Uncertain`.
Response timeout не очищает pending submission: blind replay запрещён до fresh authorization
update либо late-response reconciliation. Definitive input rejection (`400`) у code может
запросить новый код только по описанному TDLib resend rule; у 2FA password CLI выводит
«Пароль отклонён, попробуйте ещё раз» и заново получает owner prompt, не replay-ит прежний secret.
`429/500` не запускают resend. `timeout` не трактуется как TTL OTP: он только открывает resend,
а сам resend требует `next_type`.

В phone/premium state владелец выбирает phone или QR. QR link передаётся отдельным owner-only
prompt через private socket и печатается только в `/dev/tty`; JSON/MCP status по-прежнему
содержит только state/token/action. Email branches аналогично предлагают разрешённые TDLib
Apple/Google token alternatives. Registration сначала показывает ToS, требует явного согласия,
затем отдельно спрашивает notification choice; безопасный default даёт
`disable_notification=true`. Decline не вызывает `registerUser`.

`LoginSubmitted` и `LoginCodeResent` имеют machine `status:"partial"`. Terminal completion
по-прежнему требует `authorizationStateReady`, успешный `getMe` и проверку expected identity
в daemon. Любой auth-loss сбрасывает verified readiness и отзывает leases; следующий Ready
повторяет `getMe`/identity proof. Первый live phone/OTP/2FA login и returning login выполнены
2026-07-18; открытой live-границей остаётся Telegram-side expired-code resend.

## Текущее ограничение parallel login

`LoginSubmit` синхронно занимает single-thread daemon serve loop до 30 секунд (`AUTH_CALL_TIMEOUT`).
У CLI socket timeout 35 секунд, чтобы один owner client получил ответ с запасом на framing/transport.
Этот запас корректен только при одном одновременном login client: второй клиент ждёт тот же serve
loop и не получает отдельной гарантии 35 секунд. Parallel login не является поддержанным contract.
