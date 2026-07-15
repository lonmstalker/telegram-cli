# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] completed | W-20260715-053 | Реализован exact generated TDLib registry

- Закрыт первый Tasks-пункт P3: offline `tdlib-registry-gen` использует существующий strict parser и генерирует static Rust descriptors всей pinned schema без per-method modules или второго parser.
- `ValidatedRequest` рекурсивно проверяет method, known fields, scalar/vector и concrete/abstract object types. `TdObject` сохраняет исходный JSON целиком и сопоставляет known constructor без потери неизвестных fields/constructors.
- Behavior tests сравнивают parser и generated registry без hardcoded counts/hash, проходят lookup и nested validation, затем round-trip каждого generated constructor и неизвестного future constructor. Deterministic generator gate добавлен в `scripts/check-tdlib-registry.py`.
- Готовый `tdlib-rs 1.4.0` проверен и отклонён по exact-version/forward-compatibility boundary; [D-20260715-048](../decisions/decisions.md), [evaluation digest](../raw/2026-07-15-p3-rust-bindings-evaluation.md), [registry contract](../../docs/tdlib-generated-registry.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: единая capability-таблица данных; всё неотревьюенное остаётся default-deny.

## [2026-07-15] completed | W-20260715-054 | Capability classification перенесена в одну data table

- Закрыт второй Tasks-пункт P3: reviewed contracts из `docs/capability-notes.md` перенесены в canonical `tools/tdlib-registry-gen/capabilities.json` с risk, account scope, runtime requirements и retry/idempotency class.
- Generator валидирует closed vocabularies, duplicate/unknown method rows и непустые requirements. Каждый schema method получает generated descriptor; отсутствие reviewed row даёт `DefaultDeny` без guessed classification.
- Behavior checks доказывают alignment `CAPABILITIES`/`METHODS`, representative reviewed read и unreviewed deny; generator negative control отклоняет unknown/duplicate rows. Никаких hardcoded count/hash тестов или per-family modules нет.
- Durable contract: [D-20260715-049](../decisions/decisions.md); human reference: [`docs/capability-notes.md`](../../docs/capability-notes.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: `version`, `capabilities`, schema search/describe и universal `td call` в core.

## [2026-07-15] completed | W-20260715-055 | Реализован universal core discovery/call

- Закрыт третий Tasks-пункт P3: `telegram_core::raw_api` даёт verified runtime/schema version, generated capabilities, case-insensitive token search, symbol/type describe и один generic `td_call`.
- `td_call` принимает только `ValidatedRequest`, использует transport-owned correlation, возвращает lossless `TdObject` и отклоняет known successful constructor вне declared result family. Unknown method останавливается до transport send.
- Tests используют generated registry и существующий startup transport mock: поиск/describe/default-deny, runtime version и successful `getMe` dispatch проверены без per-method wrapper catalog.
- Durable contract: [D-20260715-050](../decisions/decisions.md); implementation contract: [`docs/core-raw-api.md`](../../docs/core-raw-api.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: policy применяется до raw dispatch.

## [2026-07-15] completed | W-20260715-056 | Policy встроена до raw dispatch

- Закрыт четвёртый Tasks-пункт P3: единственная `raw_api::td_call` теперь требует `RawPolicy`; schema validation и generated capability lookup выполняются до любого transport send.
- Reviewed method допускается только при matching account kind и granted risk. Unreviewed row, wrong account и missing risk fail closed отдельными `PolicyError`; runtime requirement не выдаётся за live proof.
- Trust-boundary tests подтверждают positive reviewed read и отсутствие backend send при default-deny; pure policy checks покрывают account/risk negative paths.
- Durable contract: [D-20260715-051](../decisions/decisions.md); implementation contract: [`docs/core-raw-api.md`](../../docs/core-raw-api.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P3: coverage report генерируется из manifest/registry в `docs/tdlib-api-coverage.md`.

## [2026-07-15] completed | W-20260715-057 | Generated coverage report закрыл P3 Acceptance

- Закрыт пятый Tasks-пункт P3: `tdlib-registry-gen` обновляет bounded generated block в `docs/tdlib-api-coverage.md` вместе с Rust registry, а один gate проверяет deterministic equality обоих artifacts.
- Report сопоставляет независимые manifest и generated counts для methods, constructors, updates и authorization states; отдельно показывает capability coverage, reviewed rows и валидный `DefaultDeny` remainder.
- P3 Acceptance подтверждают существующие behavior gates: universal core equality, round-trip всех constructors, lossless update fallback, early runtime mismatch и policy-before-send. Update registry получил прямую проверку принадлежности каждого generated имени семейству `Update`.
- Durable contract: [D-20260715-052](../decisions/decisions.md); implementation evidence: [`docs/tdlib-api-coverage.md`](../../docs/tdlib-api-coverage.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт: P4 разделяет pure `resolve` и explicit `ensure_membership`.

## [2026-07-15] completed | W-20260715-058 | Resolve отделён от explicit membership

- Закрыт первый Tasks-пункт P4: `telegram_core::workflows` предоставляет typed `resolve` и `ensure_membership`; caller не формирует TDJSON `@type`, а оба пути используют общий validated/policy-gated raw call.
- Read resolver покрывает known chat ID, public username и invite preview и тестом не dispatch-ит join methods. Membership dispatch возможен только через отдельную функцию и сохраняет success/request-pending/approval-required/declined/unknown outcomes.
- Capability data дополнена пятью фактическими workflow methods: три `read/safe_read` resolver calls и два `reversible_mutation/reconcile` join calls; account scope консервативно regular-user only.
- TDLib error возвращается sanitized structured error без invite link и не становится ложным `not_found` или membership proof. Contract: [D-20260715-053](../decisions/decisions.md), [`docs/chat-resolution-membership.md`](../../docs/chat-resolution-membership.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: повторный `loadChats`, ordered position cache и documented terminal condition.
