# Daemon session lifecycle contract

Последний P2 slice подключает один pinned TDLib runtime к lock-owner `telegramd`. Daemon принимает lease requests только после returning authorization proof и освобождает canonical DB lock только после normal `close -> authorizationStateClosed` либо process crash.

## Startup и identity

1. `TELEGRAM_PROFILE` и absolute `TDLIB_DATABASE_DIR` выбираются process configuration; daemon первым получает `ProfileDatabaseLock`.
2. Target-specific `libtdjson` берётся из pinned provenance cache path либо из `TDJSON_LIBRARY_PATH` packaging override. До unsafe ABI load проверяются exact byte length и SHA-256.
3. Database key загружается как Base64 file secret из `TDLIB_DATABASE_KEY_FILE`; `.env.local` читает только protected loader, не daemon.
4. `CoreRuntime` проходит pinned version/commit handshake, отправляет `setTdlibParameters` и принимает только returning `authorizationStateReady`. Любой phone/QR/OTP/2FA branch останавливает unattended startup с `InteractiveAuthorizationRequired`.
5. `Ready` публикуется только после успешного `getMe`. Optional `TELEGRAM_EXPECTED_USER_ID` сверяется непосредственно; без него первый успешный startup создаёт `.telegramd-identity` как current-user regular single-link file `0600`, а дальнейшие starts требуют exact match. Identity не попадает в lifecycle output.

## Ready, activity и idle

State order фиксирован: `Stopped -> Starting -> Ready -> Draining -> Closed`.

- `TELEGRAM_IDLE_TIMEOUT_MS` — положительный deployment setting; local default сейчас 30 seconds.
- Background expiry удаляет leases даже после client disconnect. Любой active lease сбрасывает idle timer.
- В P2 workflow dispatcher ещё отсутствует, поэтому workflow activity равна zero по построению. Первый P4 consumer обязан добавить свой реальный in-flight counter в ту же idle eligibility check; отдельный speculative tracker заранее не создаётся.
- Socket listener poll nonblocking, но accepted JSONL connection переводится в blocking mode с 5-second IO deadline. Это сохраняет bounded loop и устраняет macOS `WouldBlock` race нескольких клиентов.
- При начале draining socket удаляется до `close`, поэтому новые clients не получают lease на закрывающейся сессии и могут bounded-retry новый startup после release owner lock.

## Normal close и crash

- Normal idle stop отправляет только TDLib `close`, проверяет response `ok`, продолжает ordered receive loop до `authorizationStateClosed`, затем останавливает transport и освобождает lock. `logOut` и `destroy` отсутствуют в automatic lifecycle.
- Client crash не владеет server resource: lease остаётся до bounded TTL и затем истекает background loop.
- Daemon crash освобождает kernel lock и может оставить stale socket; следующий owner восстанавливает socket и encrypted returning session без нового login input.
- Profile identity marker живёт рядом с DB и переживает normal/crash restart; lease IDs, наоборот, принадлежат одному daemon boot.

## Доказанная граница

Protected live gate на существующей зашифрованной session подтвердил `Starting -> Ready -> Draining -> Closed`, `getMe` identity binding, TTL после client disconnect, restart после forced process termination и повторный `Ready` без challenge. Concurrent process gate подтвердил одного lock winner, четыре denied owners и четыре параллельных lease clients одного daemon. Sanitized evidence: [P2 daemon lifecycle acceptance](../.memory/raw/2026-07-15-p2-daemon-lifecycle-acceptance.md).

CLI lazy-start/status commands, generic TDLib dispatch, workflow activity source и resident/scheduled deployment принадлежат последующим фазам и здесь не заявлены.
