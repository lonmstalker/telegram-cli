# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap: `product.md`, `plans.md`, `HARNESS.md` (F001–F022), harness-файлы, `docs/tdlib-api-coverage.md`.
- Cargo workspace из шести пакетов; границы под gate `scripts/check-workspace-boundaries.py`; `telegramd` — working shared-session owner, остальные product binaries пока fail-closed.
- Pinned schema: TDLib `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`; 1010 functions, 2168 definitions, 184 updates, 13 auth states; gate `scripts/check-tdlib-pin.py`.
- Strict schema parser в `telegram-core::schema` (12 тестов, без внешних dependencies).
- macOS arm64 и Linux x86_64 `tdjson` с provenance в content-addressed cache; общий gate `scripts/check-tdlib-native-pin.py`, локальная проверка обоих artifacts — с `--require-local-artifact`.
- Ручное capability-ревью: 106 reviewed и 904 default-deny методов сохранены в canonical data table и `docs/capability-notes.md`. Recognizer engine удалён ([D-20260715-035](../decisions/decisions.md)); классификация — данные с default-deny.
- Свежий protected live gate существующей зашифрованной сессии прошёл `WaitTdlibParameters -> Ready -> getMe -> close -> Closed` без нового login input; `.env.local` contract (mode `0600`, protected loader) соблюдён.
- Canonical GitHub remote: `https://github.com/lonmstalker/telegram-cli.git` (public, принято пользователем).
- P0 accepted: `tg-analytics@e35c54ce213aa170fb0b411eab614485424b3e60` audited from clean archive (97 tests); phase-neutral patterns перенесены, runtime contracts распределены по owner-фазам в `docs/tg-analytics-reuse.md`.
- Account/session model [D-20260715-036](../decisions/decisions.md): один `telegramd` owner на profile, CLI/MCP lease clients, returning auth с `Ready` + `getMe` proof.
- P1 transport [D-20260715-037](../decisions/decisions.md): один backend/receive thread, transport-owned `@extra`, ordered raw event stream; pinned macOS native `getOption version` smoke green.
- P1 authorization [D-20260715-038](../decisions/decisions.md): exhaustive challenge machine, exact QR/phone/code/2FA/email/device/registration requests, stale/duplicate input fail closed, auth values redacted/zeroizing.
- P1 database key [D-20260715-039](../decisions/decisions.md): FD/strict `0600` file/macOS-Linux keychain sources, zeroizing raw bytes, Base64 TDJSON, empty-key preflight deny и wrong-key 401 latch без phone fallback.
- P1 reducer [D-20260715-040](../decisions/decisions.md): transport-order sequence и versioned auth/user/chat/group/file/connection/message-send caches; partial updates require base entity, send terminal states не регрессируют.
- P1 unknown updates [D-20260715-041](../decisions/decisions.md): unknown constructors сохраняются exact raw Value в FIFO sequence; field patches известных objects сохраняют будущие поля.
- P1 runtime [D-20260715-042](../decisions/decisions.md): один абсолютный deadline, cancellable pending responses, log disable before secret-shaped calls, pinned version/commit handshake и `getCurrentState` snapshot boundary. Native wrong/missing-key, secret canary и returning-session gates green.
- P1 accepted: transport correlation, authorization, protected key, ordered/lossless state и bounded startup runtime закрывают все Acceptance-критерии фазы.
- P2 ownership [D-20260715-043](../decisions/decisions.md): configured `telegramd` canonicalize-ит absolute DB directory и удерживает safe `0600` non-blocking OS lock; symlink aliases/второй process отклоняются, после exit lock reacquire-ится.
- P2 socket [D-20260715-044](../decisions/decisions.md): owner-lock winner использует private `/tmp/telegramd-<uid>` `0700` и bind-ит `<profile>.sock` exact mode `0600`; live/unsafe entries fail closed, current-user refused socket восстанавливается как stale.
- P2 leases [D-20260715-045](../decisions/decisions.md): bounded JSONL socket protocol выдаёт boot-unique lease ID, хранит principal/opaque scopes и TTL, поддерживает matching-principal heartbeat/release и fail-closed expiry.
- P2 scheduler [D-20260715-046](../decisions/decisions.md): per-profile FIFO tickets допускают bounded contiguous read prefix, mutation только при zero active и не позволяют late read обогнать queued mutation.
- P2 lifecycle [D-20260715-047](../decisions/decisions.md): daemon проверяет pinned native artifact, загружает protected key, требует returning `Ready/getMe` с stable owner-only identity binding и закрывается по zero lease/workflow activity только через `close -> authorizationStateClosed`.
- P2 accepted: concurrent process/client gate, client-crash TTL, daemon-crash returning restart и normal idle restart закрывают все Acceptance-критерии фазы. Live evidence: [P2 daemon lifecycle acceptance](../raw/2026-07-15-p2-daemon-lifecycle-acceptance.md).
- Первый пункт P3 закрыт: exact generated Rust registry содержит descriptors pinned methods/constructors/types/updates/auth states; общий validator проверяет nested requests, а `TdObject` сохраняет неизвестные fields/constructors losslessly ([D-20260715-048](../decisions/decisions.md)).
- Второй пункт P3 закрыт: одна JSON capability-таблица хранит reviewed risk/account/runtime/retry rows; generated `CAPABILITIES` покрывает каждый method и оставляет отсутствующие rows `DefaultDeny` ([D-20260715-049](../decisions/decisions.md)).
- Третий пункт P3 закрыт: `raw_api` предоставляет verified version, capabilities, token schema search, symbol/type describe и один validated/lossless `td_call` поверх `CoreRuntime` ([D-20260715-050](../decisions/decisions.md)).
- Четвёртый пункт P3 закрыт: единственный `td_call` требует `RawPolicy` и отклоняет unreviewed method/account/risk mismatch до transport; runtime requirements остаются честным prerequisite, не guessed proof ([D-20260715-051](../decisions/decisions.md)).
- Пятый пункт P3 и Acceptance закрыты: generated coverage block раздельно показывает manifest и registry/core counts, constructor/update/auth coverage и reviewed/default-deny disposition ([D-20260715-052](../decisions/decisions.md)).
- P3 accepted: exact registry, capability data, universal raw API, policy-before-send и generated coverage report закрывают все Acceptance-критерии фазы.
- Первый пункт P4 закрыт: Rust target enums разделяют read-only `resolve` и explicit `ensure_membership`; join outcomes сохраняют pending/approval/declined без ложного membership proof ([D-20260715-053](../decisions/decisions.md)).
- Второй пункт P4 закрыт: chat-list loader повторяет `loadChats` до documented `404`, применяет ordered updates через response boundary и выдаёт positions по `(order, chat_id)` descending ([D-20260715-054](../decisions/decisions.md)).
- Третий пункт P4 закрыт: chat inspection нормализует username/public link/invite, ждёт authoritative cache, выбирает full-info call по `ChatType` и парно закрывает optional open lease ([D-20260715-055](../decisions/decisions.md)).
- Четвёртый пункт P4 закрыт: history/search pager продолжает short pages, следует returned/derived cursors и различает complete count/date/exhausted от partial no-progress ([D-20260715-056](../decisions/decisions.md)).
- Пятый пункт P4 закрыт: members pager проверяет reducer-owned capability и method-specific terminal condition; statistics walker раскрывает async graph tokens до data/error и сохраняет partial/lineage/freshness evidence ([D-20260715-057](../decisions/decisions.md)).
- Шестой пункт P4 закрыт: typed file/sticker transfer, bot start и scoped Web App workflows ждут matching terminal updates; Web App URL redacted/zeroizing, close paired ([D-20260715-058](../decisions/decisions.md)).
- Седьмой пункт P4 и Acceptance закрыты: explicit update gap блокирует state-dependent workflows, policy-gated `getCurrentState` atomically заменяет reducer и только тогда очищает marker ([D-20260715-059](../decisions/decisions.md)).
- P4 accepted: prerequisite resolution, method-specific pagination terminal rules, scoped open/close и send terminal update waits подтверждены behavior tests.
- Первый пункт P5 закрыт: один account scheduler применяет explicit account/chat/generated-risk queue/rate budgets и flood blocks с bounded jitter; unreviewed methods fail before queue ([D-20260715-060](../decisions/decisions.md)).
- Второй пункт P5 закрыт: core retry executor допускает только generated `safe_read` и `convergent`; read ждёт supplied delay, convergent перед повтором того же request проверяет desired state, а uncertain outcome не повторяется ([D-20260715-061](../decisions/decisions.md)).
- Третий пункт P5 закрыт: owner-only durable journal fsync-ит fingerprint `pending/succeeded/failed/uncertain`, восстанавливает interrupted dispatch как uncertain и разрешает новый begin только после terminal или reconciled `NotApplied` proof ([D-20260715-062](../decisions/decisions.md)).
- Четвёртый пункт P5 закрыт: protocol/lease используют восемь typed risk scopes; owner ceiling default-deny ограничивает requests, а generated method risk и действующий lease строят pre-dispatch `RawPolicy` ([D-20260715-063](../decisions/decisions.md)).
- Пятый пункт P5 закрыт: high-risk request получает exact SHA-256 plan preview и dispatch-ится только с unexpired one-shot Ed25519 capability внешнего signer; daemon имеет только public key ([D-20260715-064](../decisions/decisions.md)).
- Шестой пункт P5 закрыт: fixed-shape snapshot покрывает latency/outcome, queue, retry/flood, update lag, freshness и leases; audit JSONL хранит только generated method/risk и closed operational fields, а payload/identifiers отсутствуют по schema ([D-20260715-065](../decisions/decisions.md)).
- P5 accepted: timeout/restart требует reconciliation до нового write, safe-read выдерживает supplied delay, approval невозможно подделать daemon-side, secret-output и telemetry canary tests green.
- Первый подпункт CLI commands P6 закрыт: `telegram-cli` через validated private profile socket выполняет session status/hold/release и остаётся protocol-only client без TDLib/DB ownership ([D-20260715-066](../decisions/decisions.md)).
- Второй подпункт CLI commands P6 закрыт: version/capabilities/search/describe и один `td call` проходят daemon-owned generated discovery/validator/policy; CLI не содержит registry или per-method wrappers ([D-20260715-067](../decisions/decisions.md)).
- Третий подпункт CLI commands P6 закрыт: один discoverable workflow route строго преобразует owned JSON inputs во все 13 реализованных core workflows; CLI не дублирует их state machines ([D-20260715-068](../decisions/decisions.md)).
- Четвёртый подпункт CLI commands P6 закрыт: login status строится из typed authorization machine без challenge values, а bounded events route отдаёт только sequence/kind/cursor/gap metadata и явно маркирует потерю наблюдения ([D-20260715-069](../decisions/decisions.md)).
- Пятый Tasks-пункт P6 закрыт: human default и machine envelope v1 разделяют `ok/partial/error`, daemon добавляет typed workflow completeness, а client/command/lease failures имеют closed codes и стабильные exit categories ([D-20260715-070](../decisions/decisions.md)).
- Шестой Tasks-пункт P6 закрыт: human/JSONL `events watch` heartbeat’ит lease, продолжает cursor и при signal/pipe cancellation освобождает lease вне signal handler; JSON остаётся one-shot snapshot ([D-20260715-071](../decisions/decisions.md)).
- Седьмой Tasks-пункт P6 закрыт: `login tty` вводит phone/OTP/2FA/email/registration только через protected `/dev/tty`, связывает typed input с daemon challenge и не принимает secrets в flags/stdin/output ([D-20260715-072](../decisions/decisions.md)).
- Восьмой Tasks-пункт и Acceptance P6 закрыты: compact skill использует JSON envelope и on-demand `workflow describe`/schema discovery; пять cold-context traces покрывают history/statistics/sticker/bot/Mini App handoff ([D-20260715-073](../decisions/decisions.md)).
- P6 accepted: raw/workflow CLI parity, prose-free machine decisions и bounded cold-agent skill подтверждены protocol tests и eval artifact.
- Первый подпункт P7/F007 закрыт: user resolver/profile view редактирует private fields, а `setName` завершается только по matching ordered `updateUser` ([D-20260715-074](../decisions/decisions.md)).
- Второй подпункт P7/F008 закрыт: main/archive/folder используют один terminal-correct loader; forum topics следуют returned cursor, а close/reopen завершается только после desired-state probe ([D-20260715-075](../decisions/decisions.md)).
- Третий подпункт P7/F009 закрыт: history/search не меняют read state без explicit flag, protected content редактируется, а text send завершается matching send update или uncertain без повтора ([D-20260715-076](../decisions/decisions.md)).
- Четвёртый подпункт P7/F010 закрыт: file transfer ждёт terminal update и сверяет известный размер; cancel reconciles state, local/generated path confined to daemon files root ([D-20260715-077](../decisions/decisions.md)).
- Пятый подпункт P7/F011 закрыт: fresh right + exact external plan защищают title configuration; matching update доказывает completion, а membership/pending/no-progress boundaries переиспользуют existing workflows ([D-20260715-078](../decisions/decisions.md)).
- Шестой подпункт P7/F012 закрыт: pre-action sequence и exact bot/chat фильтр коррелируют redacted reply; callback payload остаётся в core, timeout/uncertainty не повторяются ([D-20260715-079](../decisions/decisions.md)).

## Not implemented

- Оставшиеся подпункты P7/F013–F022 и фазы P8–P10: domain workflows, optional MCP, packaging и live acceptance.

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты — вне model-visible interfaces.
- Protected key provider подключён к штатному daemon; [P-20260715-001](../problems/problems.md) resolved в P2.
- Linux artifact boundary закрыта в [P-20260715-003](../problems/problems.md); bit-for-bit reproducibility не заявлена.
- Неотревьюенные методы — default-deny; это валидное состояние, не блокер (см. `plans.md`, «Правила работы»).
- Следующий implementation boundary: P7/F013 Mini Apps.
