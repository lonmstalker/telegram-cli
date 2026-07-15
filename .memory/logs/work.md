# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

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

## [2026-07-15] completed | W-20260715-059 | Реализован terminal-correct chat-list loader

- Закрыт второй Tasks-пункт P4: `load_chat_list` поддерживает typed Main/Archive/Folder target и повторяет policy-gated `loadChats` до documented TDLib error `404` в пределах одного deadline.
- Raw call возвращает internal correlation boundary; runtime применяет preceding events к reducer. Ordered view читается из canonical `chat.positions`, сортируется по `(order, chat_id)` descending и не дублирует cache вторым индексом.
- Integration backend дал две короткие successful load-порции с position updates и только затем `404`; workflow сделал три calls и вернул `(20, 3), (10, 2)`, доказав, что `ok`/short batch не terminal.
- Capability data добавила только фактический `loadChats` consumer (`read`, `safe_read`, regular user). Contract: [D-20260715-054](../decisions/decisions.md), [`docs/chat-list-loading.md`](../../docs/chat-list-loading.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: full chat workflow с cache wait, optional open lease и full info.

## [2026-07-15] completed | W-20260715-060 | Реализован full chat inspection workflow

- Закрыт третий Tasks-пункт P4: `inspect_chat` принимает ID/username/public link/invite, применяет resolver updates до response boundary, ждёт reducer cache и выбирает exact full-info request по cached `ChatType`.
- Optional open path использует scoped `OpenChatLease`: success test доказал `searchPublicChat -> openChat -> getSupergroupFullInfo -> closeChat`, failure test — `closeChat` после TDLib full-info error.
- Private invite с `chat_id=0` возвращает `MembershipRequired`, `complete=false`; negative test подтверждает отсутствие join/open dispatch. Public link parser отклоняет invite-shaped link вместо смены ветки.
- Capability data дополнена тремя full-info reads и двумя presence calls, без method-family module. Contract: [D-20260715-055](../decisions/decisions.md), [`docs/chat-inspection-workflow.md`](../../docs/chat-inspection-workflow.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: history/search pagination до count/date/no-progress boundary.

## [2026-07-15] completed | W-20260715-061 | Реализована honest history/search pagination

- Закрыт четвёртый Tasks-пункт P4: `chat_history` следует derived minimum-ID cursor, `search_chat_messages` — exact returned cursor; оба принимают bounded count/min-date/page-limit options и один deadline.
- Short history page при limit 100 продолжила второй call и завершилась только по requested count. Search test доказал returned cursor forwarding и partial `NoProgress` на cursor repeat; отдельные cases закрыли date и cursor-0 exhaustion.
- Approximate `total_count` не используется, duplicate boundary IDs не дублируют output, message raw JSON/order сохраняются. Capability data добавила только `getChatHistory`/`searchChatMessages` как regular-user read/safe-read.
- Contract: [D-20260715-056](../decisions/decisions.md), [`docs/message-pagination.md`](../../docs/message-pagination.md).
- Verification перед коммитом: обязательные workspace tests, Clippy, все `scripts/check-*.py` и wiki rotation gate.
- Следующий Tasks-пункт P4: members/statistics capability fields, async graph tokens и freshness rules.

## [2026-07-15] completed | W-20260715-062 | Реализованы members/statistics chains и freshness evidence

- Закрыт пятый Tasks-пункт P4: `supergroup_members` проверяет reducer-owned `can_get_members`, продолжает short pages и различает count/exhausted от partial no-progress.
- `chat_statistics` проверяет `can_get_statistics`, раскрывает recursive async graphs до data/error, сохраняет token lineage и возвращает partial с unresolved token при repeat/deadline.
- Result содержит capability cache sequence, collection `observed_at` и явный `ServerSnapshot`; completion не выдаётся за real-time freshness. Capability data добавила только фактические `getSupergroupMembers`/`getStatisticalGraph` consumers.
- Contract: [D-20260715-057](../decisions/decisions.md), [`docs/members-statistics-workflow.md`](../../docs/members-statistics-workflow.md). Targeted core workflow tests и Clippy green; перед коммитом выполняется полный обязательный gate.
- Следующий Tasks-пункт P4: file/sticker/bot/Web App workflows с ожиданием terminal updates.
