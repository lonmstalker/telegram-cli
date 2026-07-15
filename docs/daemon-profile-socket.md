# Daemon profile socket and election contract

Второй P2 slice реализован в `telegramd::socket`. `DaemonSocket::bind` принимает только живой `ProfileDatabaseLock`, поэтому loser startup election не может удалять или bind-ить socket победителя.

## Namespace и permissions

- Profile name ограничен 1–48 ASCII alphanumeric/`-_.` characters.
- Socket path — `/tmp/telegramd-<effective-uid>-<profile>.sock`. Короткий namespace не зависит от длины canonical TDLib DB path и помещается в macOS/Linux `sockaddr_un`.
- Bind выполняется при временном process umask `0177`, сериализованном и восстановленном RAII guard; до публикации runtime проверяет exact socket mode `0600`.
- Socket entry должна быть current-user Unix socket с одним hard link. Symlink, regular file, чужой owner или другая unsafe entry не удаляются и fail closed.

## Election и stale recovery

1. Daemon сначала получает exclusive canonical DB lock.
2. Если socket отсутствует, победитель bind-ит его.
3. Если current-user socket принимает `connect`, startup возвращает `AlreadyServing` и ничего не удаляет.
4. `ConnectionRefused` означает stale inode после crash/kill; только такая entry удаляется перед bind. Остальные probe/remove errors fail closed.
5. На normal Drop удаляется только pathname с теми же device/inode, которые создал guard; заменённая entry не затрагивается.

Atomicity опирается на canonical DB lock: одновременно socket namespace меняет только один cooperating `telegramd`. CLI/MCP позже используют profile config для поиска socket и никогда не выбирают DB path аргументом.

## Current runtime boundary

Configured daemon держит lock/listener и обслуживает [lease JSONL protocol](daemon-leases.md). TDLib calls, fair request queue и lifecycle runtime ещё не подключены.

## Verification

- Kernel tests подтверждают exact `0600`, живой listener не заменяется, stale socket восстанавливается, regular file сохраняется с fail-closed error.
- Process-level synthetic gate запускает конкурентный daemon, проверяет denial второго owner, оставляет stale socket через process termination и подтверждает успешный replacement bind с mode `0600`.
