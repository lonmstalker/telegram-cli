# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] corrected | D-20260715-044 | Private directory заменяет process-global umask

- Full workspace gate обнаружил, что временный process-global `umask(0177)` влияет на параллельное filesystem IO других threads, даже если собственные socket binds сериализованы.
- Socket namespace исправлен на current-user `/tmp/telegramd-<uid>` exact mode `0700` и `<profile>.sock` exact mode `0600`. Private parent защищает pathname с момента bind; global umask/mutex удалены. Остальные election/stale/inode contracts D-20260715-044 сохранены.

## [2026-07-15] accepted | D-20260715-048 | Exact schema генерирует descriptors, не per-method wrappers

- Единственный strict parser pinned `td_api.tl` остаётся в `telegram-core::schema`; offline tool генерирует static Rust descriptors методов, constructors, fields, types, updates и authorization states. Product packages не зависят от tool.
- Один общий `ValidatedRequest` проверяет pinned method и nested TDJSON fields. Один `TdObject` сохраняет исходный JSON целиком; неизвестные fields и constructors не проецируются и не теряются.
- TDJSON `@type` остаётся только wire discriminator внутри generated registry/общего codec и transport boundary. Per-method Rust modules/functions не генерируются: они дублировали бы один generic dispatch и не убрали бы `@type` из wire protocol.
- Published `tdlib-rs 1.4.0` не принят как product API: он закреплён на TDLib 1.8.61 вместо project pin 1.8.66 и не предоставляет обязательный lossless unknown-object contract. Evidence: [external evaluation](../raw/2026-07-15-p3-rust-bindings-evaluation.md).

## [2026-07-15] accepted | D-20260715-049 | Capability classification — одна data table с default-deny

- Canonical `tools/tdlib-registry-gen/capabilities.json` хранит только вручную reviewed rows: method, closed risk/account/retry classes и проверенное runtime-requirement выражение. Generator не парсит documentation и не классифицирует по имени method.
- Generated `CAPABILITIES` содержит descriptor для каждого pinned method. Отсутствующая data row порождает `DefaultDeny`, а не ошибку, наследование от семейства или guessed read/write class.
- Runtime requirement пока остаётся data expression: parser/evaluator появится только вместе с фактическим live-state policy consumer. Это сохраняет проверенное review evidence без возврата удалённого per-family engine.
- `safe_read` допускает read retry, `convergent` — повтор desired state, `reconcile` требует проверки server state, `never` запрещает automatic retry.

## [2026-07-15] accepted | D-20260715-050 | Один generated raw call вместо per-method API

- Core discovery читает static generated slices: runtime/schema version, capability dispositions, token search и exact symbol/type description не имеют отдельного hand-written catalog.
- Один `td_call` принимает JSON object, валидирует method и nested types, отправляет через daemon-owned `CoreRuntime`/transport и возвращает lossless raw object. Known successful result обязан принадлежать declared result family; TDLib `error` и неизвестный future constructor сохраняются.
- `@type` остаётся wire discriminator внутри общего JSON contract. Тысяча generated Rust functions не добавляет типовой безопасности generic agent input и не меняет TDJSON ABI, поэтому не создаётся.
- На этом checkpoint raw call не подключён к product binary; следующий P3 task встраивает policy check в единственную dispatch function до внешнего wiring.

## [2026-07-15] accepted | D-20260715-051 | Raw policy проверяется внутри единственной dispatch function

- `td_call` принимает trusted `RawPolicy` и после schema validation, но до transport send, требует generated reviewed disposition, matching account kind и granted risk class. Отдельного unchecked raw dispatcher в `raw_api` нет.
- Missing row остаётся `DefaultDeny`; wrong account и missing risk имеют отдельные structured errors. Policy decision не выводится из имени method или request payload.
- Static runtime-requirement expression публикуется как prerequisite, но текущий gate не заявляет live satisfaction. Его evaluator появится только с фактическим state consumer; TDLib error остаётся честным raw result.
- `RawPolicy` пока создаёт trusted product layer, не agent-facing input. Approval provenance и non-forgeability принадлежат P5 до external dangerous-operation wiring.

## [2026-07-15] accepted | D-20260715-052 | Coverage report — generated block из manifest и registry inputs

- Один offline generator за тот же проход обновляет Rust registry и ограниченный marker-блок `docs/tdlib-api-coverage.md`; ручной контекст документа остаётся вне generated region.
- Report раздельно показывает pinned manifest counts и generated registry/core counts для methods, constructors, updates и authorization states, а также reviewed/default-deny disposition. Равенство не достигается копированием одного готового итога во все source columns.
- Schema identity по-прежнему закреплена одним pin/hash gate. Report не вводит per-method hash, hardcoded set test или mutation pinned schema; deterministic gate сравнивает оба generated artifacts с текущими inputs.
- P3 Acceptance опирается на generated equality и behavior tests: round-trip каждого constructor, update lookup/raw fallback, runtime mismatch до рабочего call и policy/default-deny до transport send.

## [2026-07-15] accepted | D-20260715-053 | Chat resolution никогда не скрывает membership mutation

- Public core workflow принимает Rust `ChatTarget`/`MembershipTarget`; TDJSON `@type` создаётся только внутри адаптера к единственному schema-validated `td_call`. Отдельные generated per-method wrappers не возвращаются.
- `resolve` dispatch-ит только `getChat`, `searchPublicChat` или `checkChatInviteLink` с risk `read`. Только explicit `ensure_membership` dispatch-ит `joinChat`/`joinChatByInviteLink` с risk `reversible_mutation` и retry `reconcile`.
- Join success, request pending, guard-bot approval и decline — разные состояния. Pending/approval не считаются complete, TDLib error не превращается в `not_found`, неизвестный future result сохраняется losslessly.
- Первый reviewed workflow slice консервативно разрешён только основному regular-user account; возможный bot scope остаётся deny до отдельного evidence.

## [2026-07-15] accepted | D-20260715-054 | Chat list terminal только по `loadChats` error 404

- Loader повторяет `loadChats` после каждого `ok`, даже если TDLib прислал мало или ни одного нового chat. Только documented error code `404` означает `AllChatsLoaded`; `getChats` не используется как completeness proof.
- Каждый call сохраняет transport correlation boundary. Runtime применяет все preceding updates к единственному reducer до классификации response, поэтому returned list отражает ordered position updates именно завершённой load chain.
- Canonical cache остаётся raw `chat.positions`; typed view сортирует по pinned TDLib правилу `(order, chat_id)` descending и исключает `order == 0`. Отдельный синхронизируемый индекс не добавляется.
- Вся цепочка использует один absolute deadline и общий policy-gated raw dispatch; `loadChats` reviewed как regular-user `read/safe_read`.

## [2026-07-15] accepted | D-20260715-055 | Full chat read hydrates cache и владеет scoped open lease

- Typed `ChatTarget` различает ID, normalized username/public link и explicit invite. Invite-shaped public link fail-closed; private invite без accessible cached chat возвращает `MembershipRequired` и никогда не вызывает join/open.
- Resolver call отдаёт transport boundary; runtime применяет preceding ordered updates и требует authoritative cached Chat перед full read. `ChatType` выбирает `getUserFullInfo`, `getBasicGroupFullInfo` или `getSupergroupFullInfo` без guessed method family.
- Optional `OpenChatLease` dispatch-ит policy-gated `openChat`, затем всегда делает explicit `closeChat`; Drop остаётся fallback для раннего return/panic. Full-info error не пропускает cleanup.
- Full-info methods reviewed как `read/safe_read`, open/close как `presence/convergent`, regular-user only. Wire `@type` остаётся внутри core adapter, target API — Rust enums.

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
