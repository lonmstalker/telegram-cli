# P10 first-login and returning authorization acceptance — 2026-07-18

## Scope

Проверить production TDLib first login через singleton daemon и затем returning restart того же encrypted profile. Phone, OTP, cloud password, account identity и local secret/path values в evidence не включаются.

## Live evidence

1. Владелец завершил одну интерактивную `telegram-cli login` chain через owner `/dev/tty`; agent не читал terminal output со значениями.
2. Singleton daemon достиг `Ready`. В production lifecycle это публикуется только после TDLib `authorizationStateReady`, успешного `getMe` и expected-identity check.
3. Без leases daemon штатно прошёл `Draining -> Closed`; `logOut` и `destroy` не вызывались.
4. Тот же encrypted profile был запущен повторно без phone/OTP input. Daemon снова достиг `Ready`.
5. Fresh machine-status probe вернул:

```json
{"version":3,"status":"ok","data":{"type":"login_status","state":"ready","challenge_id":null,"next_action":"ready"}}
```

## Result and boundary

- First phone/code authorization и returning encrypted restart доказаны live.
- Secret values, Telegram identity и raw TDLib private updates не сохранялись.
- Отдельный expired-code resend path покрыт deterministic core/protocol/daemon tests, но actual Telegram resend после elapsed timeout в этом successful login не наблюдался; он остаётся отдельным live follow-up.
- Остальные P10 scenarios не проверялись этим evidence.
