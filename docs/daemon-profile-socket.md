# Daemon profile socket and election contract

Второй P2 slice реализован в `telegramd::socket`. `DaemonSocket::bind` принимает только живой `ProfileDatabaseLock`, поэтому loser startup election не может удалять или bind-ить socket победителя.

## Namespace и permissions

- Profile name ограничен 1–48 ASCII alphanumeric/`-_.` characters.
- Runtime directory — `/tmp/telegramd-<effective-uid>`: current-user directory exact mode `0700`; symlink, foreign owner, non-directory или другой mode fail closed. Socket path внутри — `<profile>.sock`, поэтому namespace не зависит от длины canonical TDLib DB path и помещается в macOS/Linux `sockaddr_un`.
- Private directory делает socket недоступным другим users с момента bind без process-global `umask`. После bind runtime задаёт и проверяет exact socket mode `0600`.
- Socket entry должна быть current-user Unix socket с одним hard link. Symlink, regular file, чужой owner или другая unsafe entry не удаляются и fail closed.

## Election и stale recovery

1. Daemon сначала получает exclusive canonical DB lock.
2. Если socket отсутствует, победитель bind-ит его.
3. Если current-user socket принимает `connect`, startup возвращает `AlreadyServing` и ничего не удаляет.
4. `ConnectionRefused` означает stale inode после crash/kill; только такая entry удаляется перед bind. Остальные probe/remove errors fail closed.
5. На normal Drop удаляется только pathname с теми же device/inode, которые создал guard; заменённая entry не затрагивается.

Atomicity опирается на canonical DB lock: одновременно socket namespace меняет только один cooperating `telegramd`. Private directory также исключает process-wide `umask` race с параллельным filesystem IO. CLI, MCP и Web App runner используют profile config для поиска socket и никогда не выбирают DB path аргументом.

## Общий client boundary

`crates/telegram-client` — единственный I/O client этого namespace. Перед каждым connect он через `symlink_metadata` требует current-user directory exact mode `0700` и current-user Unix socket exact mode `0600` с `nlink == 1`; wire-типы остаются в I/O-free `telegram-protocol`. CLI, MCP и Web App runner хранят только mapping `ClientErrorCode` в собственный UX.

Граница защищена grep-level gate `scripts/check-daemon-client-single-home.py`: consumer-приложениям запрещены собственные `socket_path`, `validate_socket` и прямой `UnixStream::connect`. `apps/telegramd` исключён из client-side scan как сервер namespace: его `UnixListener` и stale-socket probes относятся к election/recovery, а не к consumer client.

Различия существующих consumers закреплены параметрами, а не унифицированы неявно: CLI читает одну JSON line с timeout 35 секунд, MCP читает JSON до EOF с тем же timeout, Web App runner использует timeout 5 секунд и требует newline в пределах 16 KiB. Ошибка connect после успешной metadata-проверки остаётся `TransportFailed` для CLI и `SocketUnavailable` для runner.

## Current runtime boundary

Configured daemon держит lock/listener, обслуживает [lease JSONL protocol](daemon-leases.md) только после `Ready/getMe` и удаляет socket перед TDLib close dispatch; полный порядок описан в [session lifecycle contract](daemon-session-lifecycle.md).

## Verification

- Kernel tests подтверждают directory `0700`, socket `0600`, живой listener не заменяется, stale socket восстанавливается, regular file сохраняется с fail-closed error.
- Client tests подтверждают те же metadata predicates, private JSONL exchange и раздельные EOF/bounded-line framing contracts.
- Process-level synthetic gate запускает конкурентный daemon, проверяет denial второго owner, оставляет stale socket через process termination и подтверждает успешный replacement bind с mode `0600`.
- Live crash/restart gate подтвердил stale recovery поверх encrypted returning session; concurrent clients подтвердили blocking accepted streams под nonblocking lifecycle listener.
- `python3 scripts/check-workspace-boundaries.py` запускает daemon-client single-home guard вместе с остальными structural checks.
