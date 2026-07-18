# P10 authorization returning check

Дата: 2026-07-17. Scope: локальный macOS arm64 singleton `telegramd`, существующая encrypted returning session и machine-readable `telegram-cli` envelope v3. Digest не содержит Telegram identity, DB/files/key paths, API credentials, challenge IDs или lease IDs.

## Preconditions

- `.env.local` существует с mode `0600` и Git-ignored; значения загружались только через `scripts/with-env-local.sh` и не выводились.
- Target provenance: TDLib `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`, current macOS artifact SHA-256 `80d17ed3da7ea209b42789ef18319099b9489819b6a78495530777c91efbeba7`.
- До запуска daemon `telegram-cli --output json login` вернул закрытый machine error `socket_unavailable`; CLI не создавал второй TDLib owner.

## Offline authorization boundary

- 3 authorization-machine tests прошли phone/QR/code/password/email/registration/parameters/terminal branches, stale and mismatched inputs и redaction.
- 3 database-key tests прошли protected descriptor/file loading, exact `0600`, Base64 request shape, wrong-key latch и negative filesystem cases.
- Daemon broker redaction, identity bind/mismatch, CLI closed-request/envelope и MCP metadata-only auth tests прошли отдельно.
- Итого точечного auth-среза: 11 passed, 0 failed.

## Live returning path

- Первый daemon start использовал старый уже собранный binary, в который до provenance change был встроен прежний `include_str!`; он fail closed с `NativeRead(NotFound)`. Локальная macOS пересборка `telegramd` обновила embedded provenance; Linux build не запускался.
- Пересобранный singleton daemon прошёл `Starting -> Ready`. До secret-shaped parameters TDLib output содержал только pinned version/commit и `WaitTdlibParameters`; дальше logging был отключён core runtime.
- `telegram-cli --output json login` вернул `status=ok`, `state=ready`, `challenge_id=null`, `next_action=ready`. Daemon публикует этот state только после `authorizationStateReady`, успешного `getMe` и expected-identity binding.
- Один read lease был выдан и освобождён через CLI. После zero activity daemon штатно прошёл `Draining -> Closed`, то есть `close -> authorizationStateClosed` завершён.

## Boundary

- Existing authorization reuse доказан свежо через product daemon/CLI path.
- First phone/OTP/2FA login не выполнялся; `telegram-cli login tty` агент не запускал. Этот подпункт P10 требует отдельного disposable/new profile и operator-owned protected TTY input без `logOut` существующей сессии.
- P10 остаётся pending; этот digest не является acceptance всей authorization scenario.
