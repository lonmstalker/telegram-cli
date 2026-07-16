# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-062 | Durable pending предшествует mutation dispatch

- `OperationFingerprint` domain-separated SHA-256 строится из exact validated request. Owner-only JSONL хранит только fingerprint, `pending/succeeded/failed/uncertain` и optional reconciliation evidence digest, без method/payload/Telegram identifiers.
- `begin` fsync-ит `pending` до разрешения side effect. Terminal/timeout transitions также fsync-ятся до возврата; invalid complete record, unsafe file или запрещённая state transition fail closed.
- Torn final record без newline отбрасывается как недоказанная запись. Любой persisted `pending` при startup durable переводится в `uncertain`; `begin` после pending/uncertain возвращает только `ReconcileRequired`.
- Reconciliation `Applied` завершает `succeeded`, `Unknown` сохраняет `uncertain`, а только evidence-backed `NotApplied` даёт `failed`, из которого explicit new begin может повторить exact fingerprint.

## [2026-07-15] accepted | D-20260715-063 | Lease scope — closed typed subset owner policy

- Stable protocol определяет восемь `RiskScope`: read, presence, send, reversible mutation, admin, destructive, financial, auth/security. Unknown wire value не становится opaque grant.
- Owner-configured `TELEGRAM_RISK_SCOPES` задаёт maximum; missing/empty configuration консервативно даёт только read. Requested lease scopes сортируются/deduplicate и должны быть непустым subset ceiling.
- Действующий unexpired lease с matching principal строит `RawPolicy` из trusted account kind и своих typed scopes. Agent не передаёт method risk: exact `RiskClass` по-прежнему берётся только из generated capability data.
- Risk grant является необходимым, но не достаточным для dangerous dispatch. Exact preview/hash/external human approval остаётся следующим независимым gate.

## [2026-07-15] accepted | D-20260715-064 | External signer подтверждает exact high-risk plan

- `admin`, `destructive`, `financial` и `auth_security` request требуют `ApprovedPlan` внутри единственного raw dispatch дополнительно к account/risk scope. Остальные classes продолжают governed scope policy без fabricated approval flag.
- Preview публикует method/risk/retry и domain-separated SHA-256 exact validated request. Raw values не выводятся; внешний operator/broker показывает exact plan через protected channel и подписывает `domain || plan_hash || expiry || nonce`.
- Daemon хранит только Ed25519 public key. Strict signature, exact hash, expiry и boot-local nonce replay проверяются до выдачи unforgeable one-shot `ApprovedPlan`; matching dispatch atomically consumes его.
- Signing key и signer UI не являются repo/daemon/agent surface. Cross-crash duplicate barrier остаётся durable operation journal: receipt не заменяет `pending/uncertain/reconciliation` chain.

## [2026-07-15] accepted | D-20260715-065 | Telemetry имеет fixed schema без Telegram labels

- Один in-process snapshot хранит только bounded numeric counters/gauges для latency/outcome, queue, retry/flood, update lag, freshness и leases. Arbitrary label map и payload attachment отсутствуют.
- Audit event извлекает из validated request только generated method/risk; persisted schema дополнена closed outcome, durations, retry count и reconciliation flag, но не содержит request, IDs, error text или произвольный context.
- Audit JSONL принадлежит canonical daemon owner, имеет exact `0600`, no-follow/one-link/current-uid validation и durable append. Exporter/status surface читает этот snapshot позже и не создаёт второй telemetry engine.

## [2026-07-15] accepted | D-20260715-066 | CLI остаётся protocol-only клиентом private daemon socket

- `telegram-cli` не зависит от `telegram-core`, не загружает TDLib и не открывает DB. Session commands сериализуют closed `DaemonRequest` и принимают closed `DaemonResponse` единственного owner.
- Local client вычисляет profile socket только из validated profile name и effective UID, затем требует current-user directory `0700` и socket `0600`/one-link/current-user до connect.
- `session hold` выдаёт bounded lease с closed typed scopes; daemon повторно проверяет scopes, owner ceiling, TTL и principal. Heartbeat/signal lifetime расширит тот же command без второго lease client.

## [2026-07-15] accepted | D-20260715-067 | CLI raw parity обеспечивается одним daemon call route

- CLI schema commands не линкуют registry: daemon сериализует version/capability/schema descriptors прямо из generated core data. Search/describe не имеют отдельного command catalog.
- Один `TdCall` protocol variant несёт matching lease/principal и arbitrary JSON value в единственный core validator/policy/transport path. Все pinned methods достигают gate; default-deny и missing account/risk/approval возвращаются closed codes.
- `@type` остаётся explicit только у raw TDJSON escape hatch. Curated commands формируют discriminator внутри core; per-method Rust/CLI wrappers не генерируются и не дублируют 1010-method surface.

## [2026-07-15] accepted | D-20260715-068 | Один workflow route адаптирует все core state machines

- `workflow list/run` — единственная CLI/protocol форма для curated workflows. Daemon хранит небольшой closed route list и strict owned inputs; pagination/cache/update semantics остаются только в `telegram_core::workflows`.
- Каждый run требует matching lease/principal и передаёт core один daemon-derived `RawPolicy`. Unknown workflow/input fields fail closed до TDLib dispatch.
- `open_web_app` route не сериализует `SensitiveString` launch URL: daemon выполняет scoped open/wait/close и возвращает только terminal receipt/`require_same_origin`. Browser handoff не расширяет model-visible output.

## [2026-07-15] accepted | D-20260715-069 | Login и events protocol публикуют только закрытые metadata

- Login status выводится из typed `AuthorizationMachine` step; TDJSON `@type` остаётся внутри core boundary, а challenge values и secrets не входят в protocol. Во время interactive challenge daemon сохраняет DB ownership/private socket, но raw/workflow dispatch остаётся закрыт до `Ready -> getMe -> expected identity`.
- Event broker хранит bounded окно из 1024 записей `sequence/kind`. Payload updates, message content, Web App data и authorization values не сериализуются.
- Resume использует explicit cursor. Потеря retention window, future cursor или updates, применённые внутри workflow chain без broker observation, выражаются `gap`, а не восстановленным задним числом событием.

## [2026-07-15] accepted | D-20260715-070 | Machine output имеет один versioned envelope

- `telegram-protocol` владеет envelope v1 с закрытым root status `ok/partial/error`; human renderer остаётся неверсированным presentation layer. One-shot compact JSON и JSONL сериализуют один и тот же envelope.
- Daemon вычисляет workflow `complete` из typed core result до сериализации. `complete=false` и event gap дают root `partial`, поэтому client не распознаёт domain prose или произвольные payload fields.
- Ошибки разделены на closed `command/lease/client` domains без arbitrary text. Exit codes стабильны по классам input, availability, daemon rejection и protocol/output failure; partial остаётся exit 0, но явно виден machine status.

## [2026-07-15] accepted | D-20260715-071 | Event stream владеет heartbeat и cleanup переданного lease

- Human/JSONL `events watch` повторяет существующий one-shot cursor request, публикует только первый baseline и непустые/gap batches, а heartbeat schedule берёт треть TTL из authoritative daemon response. Отдельный streaming transport не создаётся.
- SIGINT/SIGTERM handler выполняет только atomic store. Client loop освобождает lease обычным protocol request до `cancelled`; broken pipe использует тот же cleanup без повторной записи в закрытый output. TTL остаётся fallback для SIGKILL/crash.
- Explicit JSON сохраняет one-shot snapshot и не присваивает ownership caller-held lease. Cancellation относится к watch loop; уже dispatch-нутая mutation не объявляется отменённой.

## [2026-07-15] accepted | D-20260715-072 | Login secret проходит только owner-operated TTY challenge

- `login` остаётся безопасным status route, а `login tty` — единственным CLI route для phone/OTP/2FA/email/registration input; secret не принимается flags, stdin, env или output.
- CLI открывает `/dev/tty` как no-follow nonblocking character device, отключает echo RAII guard-ом и после SIGINT/SIGTERM возвращает termios обычным control flow. Protected strings и read/request buffers zeroize, debug surfaces redacted.
- Daemon broker сохраняет существующую `AuthorizationMachine`, принимает только exact current challenge ID и typed input, блокирует pending/stale submission и не добавляет второй auth state machine. TDJSON `@type` формируется внутри core authorization request.
- `LoginSubmitted` не является completion proof: только subsequent `Ready -> getMe -> expected identity` открывает raw/workflow dispatch. Live first-login остаётся P10 gate.

## [2026-07-15] accepted | D-20260715-073 | Agent skill загружает API только через runtime discovery

- Repo-local `telegram-cli` skill содержит только один lifecycle: status, minimal lease, workflow-first discovery, schema/raw fallback, structured continuation и release. TDLib method/workflow input catalog в skill отсутствует.
- Daemon добавляет `workflow describe <name>` с exact JSON input example из bounded workflow inventory. Agent загружает descriptor только выбранного route; raw fallback использует generated schema search/describe и caller `@type` только как TDJSON discriminator.
- Machine decisions читают только envelope v1. `partial/complete=false/pending/gap/next_action` требуют continuation, resync или operator action; prose, blind mutation retry, self-approval и owner secret ввод запрещены.
- Offline cold-context traces покрывают history, statistics, sticker prerequisite, bot terminal send и Mini App browser handoff. `tiktoken 0.12.0` измерил skill как 774 cl100k/633 o200k tokens при limit 1500; live variance остаётся P10 evidence.

## [2026-07-15] accepted | D-20260715-074 | User profile output закрывает private fields до protocol

- `user_profile` разрешает self, numeric ID и public username через existing generated calls; неизвестный username сначала проходит `searchPublicChat`/ordered `updateUser`, а non-user chat не становится user profile.
- Output состоит из selected public/state fields. Phone, birthdate, private note и business info заменяются closed `unavailable|redacted`; значение не достигает daemon protocol, telemetry или memory.
- `update_profile_name` проверяет current `getMe`, не dispatch-ит достигнутое состояние и после `setName` ждёт exact newer `updateUser`. Deadline после dispatch возвращает `uncertain/complete=false`, не automatic repeat.
- Capability data добавляет только фактических consumers `getMe`, `getUser`, `setName`; остальные contact/profile methods остаются full raw/default-deny surface без guessed contract.

## [2026-07-15] accepted | D-20260715-075 | Topic completion определяется cursor и server-state probe

- Main/archive/folder chat lists используют один `loadChats` engine и reducer positions; отдельная folder state machine не создаётся. Resolve остаётся read-only, optional open lifetime парно закрывается существующим lease workflow.
- `getForumTopics` следует всей returned cursor triple. Short page не доказывает terminal; zero cursor означает exhausted, repeated cursor — `no_progress/complete=false`.
- Close/reopen topic — desired-state mutation: совпавшее состояние не dispatch-ится, а accepted или timed-out request считается verified только после matching `getForumTopic`. Иначе receipt остаётся uncertain без blind retry.
- Curated inputs скрывают TDJSON discriminator; `@type` остаётся только внутри core и в universal raw call. Остальные folder/chat/topic методы остаются generated raw/default-deny, без per-method family layer.

## [2026-07-15] accepted | D-20260715-076 | Message read-state explicit, send terminal update-driven

- History/search требуют cached chat и не вызывают presence methods по умолчанию. `mark_read=true` dispatch-ит `viewMessages` только после complete page; partial chain не создаёт скрытый read-state side effect.
- Для chat с `has_protected_content` поле message content заменяется closed redaction marker до daemon protocol. Constructor сохраняется для machine routing, payload не сериализуется.
- Text send строит nested generated-validated TDJSON внутри core. Temporary message ID связывает ordered send terminal update; response/terminal timeout возвращает uncertain и никогда не запускает blind resend.
- Edit/delete/forward/reaction/poll и остальные message families остаются full raw/default-deny surface до отдельного capability review, без per-method workflow modules.

## [2026-07-15] accepted | D-20260715-077 | File completion требует terminal state и scoped local path

- Async download/upload завершены только по response или matching ordered `updateFile` с terminal flag. Известный actual/expected size сверяется с transferred bytes; отсутствие размера маркируется отдельно, mismatch не становится complete.
- Download offset/limit остаётся resume primitive. Cancel — desired-state operation: already inactive не dispatch-ится, request/timeout завершается только после `getFile` inactive probe, иначе uncertain.
- Local/generated source canonicalized на daemon и обязан быть regular file внутри configured `TDLIB_FILES_DIR`; outside, missing root и symlink escape fail before TDLib. ID/remote source не превращается в filesystem path.
- Remote artifact provider остаётся Q001/P9; arbitrary client path не используется как server path. Остальные file families остаются generated raw/default-deny без отдельного file manager.

## [2026-07-15] accepted | D-20260715-078 | Chat administration использует exact plan поверх общего raw gate

- Read resolve, explicit membership и members pagination остаются отдельными existing workflows. Pending invite не становится membership proof; gap или отсутствие fresh right блокируют admin mutation.
- Title configuration — bounded planner/apply pair: plan фиксирует current/desired/sequence и hash exact generated `setChatTitle`; apply заново проверяет snapshot/right/hash и завершается только newer matching `updateChatTitle`, иначе uncertain без повтора.
- External receipt проходит protocol к daemon, но capability расходуется только common `td_call` для matching schema-validated request. Daemon хранит public verifier весь boot; signing key и self-approval surface отсутствуют.
- Прочие group/channel/moderation методы остаются достижимы через generated raw API и default-deny до consumer-driven review; per-method Rust family layer не создаётся.

## [2026-07-15] accepted | D-20260715-079 | Bot test коррелирует ordered reply и не раскрывает callback payload

- Test boundary — reducer sequence, записанный до trigger. Pass требует terminal send и subsequent incoming message от exact bot в exact chat; timeout continuous window — terminal failed test, gap/send uncertainty — incomplete.
- Reply output сохраняет IDs, content constructor и callback count, но не text/button labels/data. Callback выбирается recorded message/row/column, а payload формируется только внутри core.
- `getCallbackQueryAnswer` вызывается один раз: TDLib 502 — доказанный bot timeout, local response deadline — uncertain. Автоматический повтор отсутствует.
- Declarative spec Q001 и широкая cleanup-команда не вводятся: exact outbound/reply IDs задают owned artifact set, а остальные bot families остаются generated raw/default-deny.
