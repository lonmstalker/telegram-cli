# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] consolidation | W-20260715-039 | Чистка переусложнения и консолидация памяти

- По явному указанию пользователя выполнена чистка: журналы и wiki сжаты до текущего состояния, per-method записи и архивные ротации удалены (история — в git).
- Удалён capability documentation-recognizer engine (`tools/tdlib-registry-gen/src/capability*`, `telegram-core::method_capability`, ~14 000 строк): per-method drift-тесты дублировали schema pin, hash-pinned счётчики были self-referential, модуль-на-семейство нарушал «классификация — данные». Решение: [D-20260715-035](../decisions/decisions.md).
- Отревьюенные знания сохранены машинной выгрузкой перед удалением: 74 supported contract и 116 deferred методов в `docs/capability-notes.md`.
- `plans.md` переписан: правила работы против переусложнения, зоны ответственности, укрупнённые задачи, критерии приёмки с объяснениями. `docs/tdlib-api-coverage.md` обновлён под data-table подход.
- Verification: `cargo test --workspace --all-targets` (12 tests), `cargo clippy -D warnings`, `check-workspace-boundaries.py`, `check-planning-boundary.py`, `check-tdlib-pin.py` — green.
- Открытые границы: [P-20260715-001](../problems/problems.md), [P-20260715-003](../problems/problems.md); P0 остаётся in_progress (Linux target, перенос из tg-analytics).

## [2026-07-15] completed | W-20260715-040 | Закреплён Linux x86_64 TDLib artifact

- Закрыт Tasks-пункт P0 «Определить supported targets»: exact TDLib `1.8.66`/`07d3a0973f5113b0827a04d54a93aaaa9e288348` собран для `x86_64-unknown-linux-gnu` в pinned Debian 12 amd64 builder; artifact хранится вне Git по [D-20260715-004](../decisions/decisions.md).
- Provenance `vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json` закрепляет builder, packages, source preparation, ELF/SONAME/dependencies/exports, runtime version/commit и no-client smoke; общий native gate теперь проверяет оба supported target и 19 trust-boundary negative controls.
- Artifact: SHA-256 `e90ca3c25ad034b7227df918816c227de2b9aef92539c994a3bd41c42d68161b`, 51 863 816 bytes, `ELF64` x86-64, `libtdjson.so.1.8.66`, без RPATH/RUNPATH и без созданных DB-файлов. Bit-for-bit reproducibility не заявлена.
- Внешнее доказательство: [Linux x86_64 native build digest](../raw/2026-07-15-tdlib-1.8.66-native-linux-x86_64.md). Проблема [P-20260715-003](../problems/problems.md) переведена в `resolved`.
- Verification: `python3 scripts/check-tdlib-native-pin.py`; `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`; обязательные workspace/clippy/check scripts/wiki gates перед коммитом.
- Следующий Tasks-пункт P0: перенести только доказанно reusable части `tg-analytics` без NATS/Postgres/analytics orchestration.

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
