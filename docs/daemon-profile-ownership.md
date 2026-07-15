# Daemon profile ownership contract

Первый P2 slice реализован в `telegramd::ownership`. До любого будущего открытия TDLib DB daemon обязан получить `ProfileDatabaseLock` и удерживать guard весь срок процесса.

## Canonical lock identity

- Profile name не является lock key: authoritative identity — canonical absolute TDLib database directory. Поэтому разные имена profile или symlink aliases одной DB не создают второго owner.
- DB directory должен уже существовать и быть directory. Relative, missing и file paths fail closed до lock.
- Внутри canonical directory создаётся постоянный `.telegramd-owner.lock`. Он не удаляется при штатном exit: stale inode безопасен, а kernel lock исчезает при закрытии descriptor/process exit.
- Owner file открывается `O_CLOEXEC | O_NOFOLLOW | O_NONBLOCK`, должен быть regular single-link file текущего effective user с exact mode `0600`.
- macOS/Linux `flock(LOCK_EX | LOCK_NB)` выдаёт `AlreadyOwned` второму owner. Guard не `Clone`; release — только drop/завершение процесса.

Lock является обязательным межпроцессным gate для cooperating product binaries; CLI/MCP не открывают DB и не могут обойти его. Advisory locks предполагают local filesystem semantics и не защищают от отдельной посторонней программы, сознательно игнорирующей protocol.

## Current daemon entrypoint

Configured lock-owner mode получает `TELEGRAM_PROFILE` и `TDLIB_DATABASE_DIR` из process environment. В local development значения поступают только через protected `.env.local` loader; daemon сам файл не читает. Без обоих полей entrypoint остаётся прежней fail-closed заглушкой.

После успешного lock daemon проверяет pinned native artifact, загружает protected profile key и запускает [shared session lifecycle](daemon-session-lifecycle.md). Lock остаётся живым через `Ready`, lease service и graceful close до observed `authorizationStateClosed`.

## Verification

- Pure kernel test берёт lock через canonical path, отклоняет symlink alias как второй owner и разрешает reacquire только после drop.
- Invalid profile, relative path и non-directory отклоняются до locking.
- Process-level synthetic gate запускает два `telegramd` на одной temporary DB directory: второй завершается с `AlreadyOwned`; после process exit replacement owner получает lock.
- P2 live concurrency gate подтвердил, что четыре loser processes завершаются на lock до native load, пока четыре clients используют одного winner daemon.
