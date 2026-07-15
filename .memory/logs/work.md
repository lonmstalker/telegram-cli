# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] completed | W-20260715-041 | Закрыт выборочный перенос из `tg-analytics`

- Exact committed snapshot `tg-analytics@e35c54ce213aa170fb0b411eab614485424b3e60` проверен из clean temporary `git archive`; dirty source working tree не использовался и не изменялся.
- Source verification: `telegram-tdlib --lib` — 24 tests green; `telegram-agent-gateway` — 73 tests green. Evidence: [reuse audit digest](../raw/2026-07-15-tg-analytics-reuse-audit.md).
- `docs/tg-analytics-reuse.md` фиксирует уже перенесённые phase-neutral wiki/harness/secret-loader patterns, проверенные behavior contracts с owner-фазами и явный отказ от NATS/Postgres/analytics orchestration, source CLI ownership и partial raw registry.
- Принята account/session model [D-20260715-036](../decisions/decisions.md): один `telegramd` owner на profile, CLI/MCP lease clients, `Ready` + `getMe` identity proof, normal `close`, destructive `logOut`/`destroy`.
- P0 Tasks и Acceptance закрыты без преждевременной реализации runtime. Следующий пункт плана: P1 «Прямой TDJSON transport, один receive loop и `@extra` correlation».
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.

## [2026-07-15] completed | W-20260715-042 | Реализован прямой TDJSON transport

- Закрыт первый Tasks-пункт P1: `telegram-core::transport` serializes requests, владеет `@extra`, сопоставляет reversed parallel responses и публикует unmatched values/updates из единственного receive loop.
- `NativeTdJson` динамически использует четыре из пяти pinned TDJSON exports на macOS/Linux; arbitrary library path требует explicit `unsafe`. Shutdown освобождает client resource, но не выдаётся за lifecycle `close`.
- Pure verification: 15 passed, 0 failed, native test ignored by default; тесты меньше production slice и проверяют trust boundaries — reversed correlation/single receive thread, update order, reserved `@extra`, unmatched response и malformed JSON fail-closed.
- Native verification: pinned macOS artifact вернул TDLib version `1.8.66` через real correlated `getOption`; evidence: [native smoke digest](../raw/2026-07-15-tdjson-transport-native-smoke.md).
- Архитектурный contract: [D-20260715-037](../decisions/decisions.md); implementation synthesis: `docs/tdjson-transport.md`.
- Следующий Tasks-пункт P1: полная authorization state machine без изменения уже принятой owner topology.
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py`, native ignored smoke и wiki rotation gate.

## [2026-07-15] completed | W-20260715-043 | Реализована authorization state machine

- Закрыт второй Tasks-пункт P1: `telegram-core::authorization` fail-closed разбирает все pinned `AuthorizationState` constructors и выдаёт explicit parameters/challenge/lifecycle steps без schema hash/count tests.
- Exact requests реализованы для phone, QR, authentication code, 2FA password, email address/code/Apple ID/Google ID и registration; device confirmation link/metadata и email reset timers сохраняются, Premium purchase остаётся явным non-automatic challenge.
- Monotonic challenge IDs блокируют stale input; повторная submission запрещена до state update или explicit failure acknowledgement. `SensitiveString` redacted и zeroizing, `AuthorizationRequest::Debug` не содержит payload.
- `WaitTdlibParameters` пока возвращает только `ParametersRequired`: database key/provider не присвоены этому пункту и остаются следующей задачей. `Ready` не выдаётся за identity proof до `getMe`.
- Verification: `cargo test -p telegram-core --lib` — 18 passed, 0 failed, 1 native test ignored; Clippy `-D warnings` green. Tests проверяют behavior named branches и trust boundaries, а не pinned schema snapshot.
- Архитектурный contract: [D-20260715-038](../decisions/decisions.md); synthesis: `docs/authorization-state-machine.md`.
- Следующий Tasks-пункт P1: database encryption key из FD/file secret/OS keychain, wrong key fail-closed.
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.

## [2026-07-15] completed | W-20260715-044 | Реализован protected database-key provider

- Закрыт третий Tasks-пункт P1: `telegram-core::database_key` загружает non-empty key из owned FD, absolute owner-owned regular file exact mode `0600` с `O_NOFOLLOW`/`O_CLOEXEC` или macOS/Linux OS keychain reference.
- Raw bytes и временный Base64 encoder buffer zeroize-ятся; `Debug`/errors не раскрывают value, path или keychain reference, а sensitive request payload не логируется. Bounded read защищает текущий secret-input boundary от unbounded FD/file output.
- `TdlibParameters` строит exact `setTdlibParameters`; pinned `bytes` codec и empty/wrong-key behavior подтверждены [source digest](../raw/2026-07-15-tdlib-database-key-codec.md). Missing/empty key request не создаёт.
- Wrong key `401` latch-ит authorization machine: phone/QR state не принимается до новой explicit parameters submission. Durable contract: [D-20260715-039](../decisions/decisions.md).
- [P-20260715-001](../problems/problems.md) сужена, но не закрыта: provider готов, штатное `telegramd` wiring принадлежит P2 и ещё отсутствует.
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P1: ordered reducer и caches для auth/user/chat/group/file/connection/message send state.

## [2026-07-15] completed | W-20260715-045 | Реализован ordered reducer и core caches

- Закрыт четвёртый Tasks-пункт P1: `telegram-core::reducer` принимает `TdJsonEvent::Update` в receive order, присваивает monotonic sequence и штампует изменённую cache entry.
- Реализованы versioned caches для authorization, user/full info, chat и field updates, basic/supergroup/full info, file, connection и message-send state. TDJSON int53/int64 strings разбираются по pinned codec.
- Chat partial update без `updateNewChat` base fail-closed; positions/chat lists поддерживают replacement/removal semantics. Message send индексируется по old ID и не допускает regression после succeeded/failed.
- Unknown constructor пока только получает ordered outcome без raw persistence; следующий Tasks-пункт закрывает эту явно задокументированную границу. Durable contract: [D-20260715-040](../decisions/decisions.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P1: неизвестные updates сохраняются raw, без потери.

## [2026-07-15] completed | W-20260715-046 | Неизвестные updates сохраняются lossless raw

- Закрыт пятый Tasks-пункт P1: unknown TDJSON constructors сохраняются целым raw `Value` в FIFO queue с тем же global sequence, без projection/deduplication; consumer получает read slice или ordered drain.
- Known full entity objects сохраняют будущие поля при subsequent exact field patches. Числовое/string/null/nested представление unknown payload проверяется exact equality tests.
- Вместе с единственным transport receive loop и W-045 это закрывает P1 Acceptance «Updates воспроизводятся строго в receive order» для known и unknown updates.
- Durable journal, queue limits и gap recovery не заявлены раньше своих runtime/reliability consumers. Durable contract: [D-20260715-041](../decisions/decisions.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P1: deadlines, cancellation, startup `getCurrentState`, runtime version handshake.

## [2026-07-15] completed | W-20260715-047 | Реализован bounded core runtime, P1 accepted

- Закрыт шестой Tasks-пункт P1: transport поддерживает absolute deadlines и explicit/drop cancellation; `CoreRuntime` выполняет log disable, pinned version/commit handshake, startup `getCurrentState` и response-boundary snapshot reduction.
- Production/test additions соблюдают правило пропорциональности: 398/397 Rust lines после удаления дублирующего native harness; trust-boundary проверки сохранены.
- Pinned native gates свежо подтвердили handshake/current state, secret-output canary и synthetic wrong/missing-key boundary. Protected loader live gate прошёл returning `Ready -> getMe -> close -> Closed` без login input. Evidence: [P1 runtime acceptance](../raw/2026-07-15-p1-runtime-acceptance.md).
- Все P1 Acceptance-критерии закрыты: correlation, receive ordering, returning restart, wrong/missing-key fail-closed и отсутствие secret canary в captured output. Durable contract: [D-20260715-042](../decisions/decisions.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт: P2 singleton `telegramd` и exclusive lock по canonical DB path.

## [2026-07-15] completed | W-20260715-048 | Реализован canonical DB owner lock для telegramd

- Закрыт первый Tasks-пункт P2: configured `telegramd` получает profile/environment config, canonicalize-ит DB directory и удерживает `ProfileDatabaseLock` на постоянном safe owner file до process exit.
- Kernel negative control подтвердил, что real path и symlink alias одной DB не получают два lock; relative/non-directory inputs fail closed. Production/test Rust additions остаются пропорциональными (171/61 lines).
- Process-level synthetic gate: первый daemon удержал temporary DB, второй завершился с `AlreadyOwned`, replacement после exit первого успешно занял lock. Реальная TDLib DB и `.env.local` не использовались.
- Main остаётся честно partial: без config fail-closed, с lock сообщает об отсутствующем service transport и не открывает TDLib до следующих P2 пунктов. Contract: [D-20260715-043](../decisions/decisions.md), synthesis: [`docs/daemon-profile-ownership.md`](../../docs/daemon-profile-ownership.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P2: Unix socket `0600`, atomic startup election, stale-socket recovery.

## [2026-07-15] completed | W-20260715-049 | Реализован private Unix socket и stale recovery

- Закрыт второй Tasks-пункт P2: только holder `ProfileDatabaseLock` может bind-ить short per-UID/profile socket; profile slug bounded, socket создаётся/проверяется exact mode `0600`.
- Stale recovery удаляет только current-user Unix socket после `ConnectionRefused`; живой listener и unsafe/non-socket entry fail closed. Drop не удаляет replacement inode.
- Первый design с socket внутри DB directory отклонён targeted test на реальной macOS `SUN_LEN` boundary; runtime namespace перенесён в короткий `/tmp/telegramd-<uid>-<profile>.sock`, canonical DB lock остаётся election source.
- Process-level synthetic gate подтвердил mode `0600`, denial конкурентного start и replacement bind после SIGTERM/stale inode. Contract: [D-20260715-044](../decisions/decisions.md), synthesis: [`docs/daemon-profile-socket.md`](../../docs/daemon-profile-socket.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P2: lease ID, principal/scopes, TTL, heartbeat, explicit release.

## [2026-07-15] completed | W-20260715-050 | Реализован local lease protocol

- Закрыт третий Tasks-пункт P2: `telegram-protocol` получил stable lease acquire/heartbeat/release types и error codes; configured daemon теперь обслуживает один bounded JSONL request/response на connection.
- `LeaseManager` выдаёт boot-unique IDs, валидирует principal/opaque scopes, ограничивает TTL до 60 seconds, продлевает heartbeat, проверяет principal при renew/release и удаляет expired entries fail closed.
- Real socket test прошёл acquire -> heartbeat -> release; process-level synthetic daemon gate дополнительно подтвердил normalized scopes и `lease_expired`. TDLib и `.env.local` не использовались.
- Principal остаётся честно self-asserted local identity; fair queue, TDLib dispatch и lifecycle timer не заявлены раньше своих следующих Tasks-пунктов. Contract: [D-20260715-045](../decisions/decisions.md), synthesis: [`docs/daemon-leases.md`](../../docs/daemon-leases.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P2: fair per-account queue, bounded concurrent reads, serialized mutations.

## [2026-07-15] completed | W-20260715-051 | Реализован fair per-account admission scheduler

- Закрыт четвёртый Tasks-пункт P2: `AccountScheduler` выдаёт FIFO tickets, допускает contiguous reads до explicit capacity и сериализует mutations при zero active operations.
- Deterministic threaded gate доказал два concurrent reads, mutation-before-late-read fairness и release ordering; отдельный check подтвердил second mutation serialization и drop-cancellation queued ticket.
- Scheduler остаётся data-agnostic: lease protocol не маскируется под TDLib work, method class не угадывается до P3/P5, read capacity не зашита как Telegram limit. Contract: [D-20260715-046](../decisions/decisions.md), synthesis: [`docs/daemon-scheduler.md`](../../docs/daemon-scheduler.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P2: lifecycle states, idle eligibility и graceful `close` до `authorizationStateClosed`.

## [2026-07-15] completed | W-20260715-052 | Реализован shared-session lifecycle и принят P2

- Закрыт пятый Tasks-пункт P2: configured daemon проверяет exact target artifact, загружает protected database key, проходит returning `Ready/getMe`, обслуживает leases и следует `Stopped -> Starting -> Ready -> Draining -> Closed`.
- Idle eligibility требует zero active leases; P2 workflow dispatcher отсутствует, поэтому speculative tracker удалён ponytail-pass и первый P4 consumer обязан добавить фактический in-flight source в ту же check. Accepted socket streams сохраняют bounded blocking IO поверх nonblocking listener; normal stop удаляет socket, отправляет только `close`, ждёт ordered `authorizationStateClosed` и лишь затем освобождает DB owner lock.
- Live runtime выявил и закрыл два regression: absence nullable `updateChatLastMessage.last_message` теперь удаляет cached field, а inherited nonblocking accepted stream больше не роняет concurrent clients. Required update fields остаются fail closed.
- Full workspace gate дополнительно выявил process-global umask race старого socket slice; global umask/mutex удалены в пользу private current-user runtime directory `0700` и socket `0600` ([D-20260715-044](../decisions/decisions.md)).
- Protected Acceptance доказал одного winner для четырёх concurrent owners/clients, client-disconnect TTL, daemon-crash returning restart без login challenge и stable idle restart/identity. Evidence: [P2 daemon lifecycle acceptance](../raw/2026-07-15-p2-daemon-lifecycle-acceptance.md), contract: [D-20260715-047](../decisions/decisions.md), synthesis: [`docs/daemon-session-lifecycle.md`](../../docs/daemon-session-lifecycle.md).
- Все P2 Acceptance-критерии закрыты. Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт: P3 generated registry из pinned schema — validation, descriptors и forward-compatible unknown fields.
