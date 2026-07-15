# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-043 | Canonical DB directory определяет daemon ownership

- Profile name не является уникальным lock key: `telegramd` canonicalize-ит существующий absolute DB directory, поэтому symlink/path aliases и разные profile labels одной DB сходятся на одном owner lock.
- Постоянный owner file внутри DB открывается fail-closed как current-user regular single-link `0600` file без symlink traversal. Non-blocking macOS/Linux `flock` берётся до любого будущего TDLib runtime и удерживается живым descriptor.
- Lock file после exit не удаляется; kernel ownership освобождается вместе с descriptor/process. Socket election, PID/status metadata и leases не смешиваются с этим primitive и принадлежат следующим Tasks-пунктам P2.

## [2026-07-15] accepted | D-20260715-044 | DB owner lock выбирает единственный private profile socket

- Socket identity строится из effective UID и validated bounded profile slug в коротком `/tmp` namespace; canonical DB lock остаётся authoritative startup election и должен быть получен раньше socket mutation.
- Unix socket создаётся с restrictive startup umask и exact mode `0600`. Existing live listener, symlink/non-socket, foreign owner и неоднозначная probe error fail closed и не удаляются.
- Только current-user socket с `ConnectionRefused` считается stale. Normal Drop удаляет path лишь при совпадении captured device/inode; crash оставляет inode для следующего bounded recovery.

## [2026-07-15] accepted | D-20260715-045 | Lease identity живёт один daemon boot и продлевается heartbeat

- Lease ID объединяет boot epoch и monotonic counter; state in-memory и намеренно не переносится между daemon restarts. Stale client ID поэтому не совпадает с новым lease.
- Local principal пока self-asserted внутри same-user `0600` socket; heartbeat/release требуют exact principal match. Scopes остаются opaque sorted data, а не преждевременной risk taxonomy P5.
- Client выбирает bounded TTL `1..=60000` ms; heartbeat продлевает исходный TTL, release удаляет сразу, expired entry fail-closed удаляется при lease operation. Background idle consumer принадлежит lifecycle Tasks-пункту P2.

## [2026-07-15] accepted | D-20260715-046 | FIFO mutation закрывает gate для поздних reads

- Один `AccountScheduler` принадлежит одному account profile. Monotonic tickets задают FIFO; contiguous read prefix допускается до explicit non-zero limit, без global cross-account queue.
- Mutation входит только с головы при zero active operations. Queued mutation блокирует более поздние reads, поэтому постоянный read traffic не starve-ит writes; одновременно active ровно одна mutation.
- Admission class поступит из P3/P5 capability data, а measured read limit — из deployment/policy consumer. Scheduler не угадывает class по method name и не фиксирует Telegram limits.

## [2026-07-15] accepted | D-20260715-047 | Ready требует pinned runtime, protected key и stable getMe identity

- `telegramd` получает canonical DB lock до config/native load, проверяет target artifact по exact provenance SHA-256/bytes и один владеет `CoreRuntime`. Protected Base64 file key поступает только через process environment loader; unattended startup принимает только returning authorization.
- `Ready` наступает после `authorizationStateReady` и `getMe`. Optional configured user ID имеет приоритет; при его отсутствии первый successful run создаёт owner-only `.telegramd-identity` `0600`, а дальнейший mismatch fail closed без вывода identity.
- Idle timeout — deployment setting с local default 30 seconds. В P2 workflows отсутствуют по построению, поэтому zero active leases разрешает удалить socket; первый P4 workflow consumer обязан войти в ту же eligibility check. Затем daemon отправляет только `close`, ждёт `authorizationStateClosed`, останавливает transport и освобождает owner lock. Client leases и boot IDs не переносятся через restart; encrypted authorization и profile identity переносятся.

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
