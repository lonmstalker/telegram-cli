# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-056 | Message pagination использует cursor proof, не размер page

- `getChatHistory` продолжает chain от минимального message ID страницы и дедуплицирует повтор boundary message. `searchChatMessages` передаёт только returned `next_from_message_id`; short/empty page с новым cursor не terminal.
- `Count`, inclusive `Date` и search cursor `0` (`Exhausted`) дают complete относительно запроса. Повтор/cycle cursor или отсутствие нового history ID останавливают chain как `NoProgress`, но оставляют `complete=false`.
- Pinned `total_count` approximate и не участвует в terminal decision. Message order/raw fields сохраняются, ID дедуплицируется только внутри одного chat workflow.
- Page limit валиден в exact TDLib range `1..=100`; вся chain использует один deadline. History/search reviewed как regular-user `read/safe_read`; mark-read/presence отсутствуют.

## [2026-07-15] accepted | D-20260715-057 | Members/statistics completeness выводится из capability и terminal state

- Members read требует reducer-owned `supergroupFullInfo.can_get_members`; statistics read — cached chat kind и `supergroupFullInfo.can_get_statistics`. Missing prerequisite и capability denial не маскируются как empty/not-found.
- Members pagination продолжает short pages и завершает chain только по requested count или exact `total_count`; пустая page до total — partial `NoProgress`.
- Каждый `statisticalGraphAsync` раскрывается до data/error. Token lineage сохраняется; repeat или graph-call deadline оставляет исходный async node и даёт partial с unresolved initial token.
- `observed_at` обозначает локальный момент получения server snapshot, не event time и не real-time proof. Capability cache sequence входит в result; resource optimization не является неявной частью read.

## [2026-07-15] accepted | D-20260715-058 | Async domain response не заменяет matching terminal update

- Curated download/sticker-upload inputs представлены Rust structs/enums; method и nested TDJSON `@type` формирует core. Local/generated paths остаются local-core boundary и не становятся remote MCP paths.
- File transfer complete только по terminal field текущего response или matching ordered `updateFile` после dispatch boundary. File ID/progress не являются completion proof.
- `sendBotStartMessage` response задаёт correlation key, но workflow завершается только по matching send succeeded/failed; acknowledgement non-terminal, deadline uncertain и не разрешает blind repeat.
- Web App launch URL хранится в redacted/zeroizing `SensitiveString`; matching `updateWebAppMessageSent` коррелируется по launch ID. Scoped lease парно вызывает `closeWebApp`; Telegram proof не выдаётся за browser proof.

## [2026-07-15] accepted | D-20260715-059 | Доказанный update gap очищает только atomic snapshot resync

- TDLib update stream не имеет server sequence, а текущий core receive loop не отбрасывает events. Поэтому elapsed time/arbitrary queue threshold не объявляет gap; owner вызывает explicit marker только при положительном evidence lag/loss. Threshold/metrics принадлежат P5.
- Gap сохраняет последнюю local sequence, переживает последующие updates и блокирует state-dependent reads/mutations structured `ResyncRequired` до dispatch.
- Policy-gated `getCurrentState` отделён response boundary. Новый reducer строится и валидируется отдельно с monotonic local sequence; только successful atomic replacement очищает marker, failure сохраняет старое gapped state.
- Transient caches не переносятся через resync. Missing snapshot prerequisite запускает обычную hydration chain и не превращается в absence proof.

## [2026-07-15] accepted | D-20260715-060 | Один account scheduler владеет queue/rate/flood admission

- Consumer обязан передать explicit `ScopeBudget { max_queued, rate }` для account, chat и каждого generated `RiskClass`; default values не объявляются Telegram truth. Missing method-class budget fail closed при startup.
- Reviewed method получает class только из generated capability data. Только `read` использует parallel read admission; все остальные risks консервативно сериализуются как mutation. Default-deny method не получает ticket.
- Один FIFO state хранит queue, active permits и bounded-by-window dispatch timestamps для всех scopes; per-family scheduler или второй rate framework не создаётся.
- Flood block задаётся explicit scope и никогда не заканчивается раньше server delay. Bounded jitter применяется только внутри configured automatic maximum; более длинный server delay сохраняется полностью с `automatic_delay=None`.

## [2026-07-15] accepted | D-20260715-061 | Retry class берётся только из generated capability data

- `telegram-core::retry` допускает dispatch loop только для exact method с reviewed `safe_read` или `convergent`; `reconcile`, `never`, unknown и default-deny отклоняются до первой попытки.
- Один immutable request передаётся всем attempts и probes. `safe_read` повторяется не раньше supplied delay; если delay не помещается в absolute deadline, retry не выполняется.
- `convergent` после retryable failure сначала проверяет exact desired state: достигнутое состояние завершает операцию, недостигнутое разрешает bounded repeat того же request, неизвестное требует reconciliation.
- Terminal и uncertain outcomes не входят в automatic retry loop. Durable persistence/fingerprint/recovery остаются отдельным следующим пунктом P5 и не симулируются in-memory executor-ом.

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
