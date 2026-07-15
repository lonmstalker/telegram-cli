# P2 daemon lifecycle acceptance digest

Дата: 2026-07-15. Scope: local macOS arm64, pinned TDLib `1.8.66`, existing encrypted returning session. Все account/environment значения загружались только через `scripts/with-env-local.sh`; digest не содержит DB/files/key paths, Telegram identity, API credentials или lease IDs.

## Runtime и normal idle close

- `telegramd` автоматически выбрал target artifact из `vendor/tdlib/native-builds/aarch64-apple-darwin.json`; runtime сверил exact artifact bytes/SHA-256 и TDLib version/commit.
- Captured lifecycle output: `Starting -> Ready -> Draining -> Closed`.
- Между `Ready` и `Closed` daemon выполнил `getMe` identity proof; identity не выводилась. Owner-only `.telegramd-identity` существует с exact mode `0600`.
- `Closed` опубликован только после TDLib `close` response и ordered `authorizationStateClosed`.

## Concurrency и TTL

- Один live winner удерживал canonical DB lock. Четыре одновременно запущенных replacement `telegramd` завершились на `AlreadyOwned` до native runtime load.
- Четыре parallel local agents успешно выполнили `lease_acquire -> lease_release` через один winner socket.
- Отдельный client получил lease с TTL 1000 ms и закрыл connection без release. При daemon idle timeout 500 ms процесс не закрылся до expiry lease, затем завершил normal `Draining -> Closed`; это подтверждает background TTL cleanup после client crash.

## Crash recovery и regression discovery

- Live daemon был принудительно остановлен без graceful close. Replacement удалил только stale current-user socket, получил owner lock и дошёл до `Ready` без phone/QR/OTP/2FA input, затем штатно закрылся через `Closed`.
- Первый long-lived probe обнаружил valid `updateChatLastMessage` без nullable `last_message`; reducer исправлен так, чтобы absence nullable TDJSON field удаляла cached field, сохраняя fail-closed behavior required fields.
- Первый concurrent-client probe обнаружил inherited nonblocking accepted stream на macOS; server теперь переводит accepted connection в blocking mode с прежним 5-second IO timeout. Повторный four-client gate прошёл.
- Первый full-workspace gate обнаружил process-global umask race старого socket slice. Final gate использует private current-user runtime directory exact mode `0700` и socket `0600`; повторные concurrent tests и live crash/stale recovery прошли без global umask.

## Acceptance mapping

- Concurrent starters converge: один owner winner, четыре denied owners, четыре clients одного socket.
- Second owner never opens DB: lock acquisition предшествует config/native load.
- Client crash TTL: connection close без release, expiry и последующий idle close доказаны.
- Daemon crash no login: forced termination и returning `Ready` без challenge доказаны.
- Idle restart same authorization: normal `close -> authorizationStateClosed`, stable identity marker и следующий `Ready/getMe` доказаны.
