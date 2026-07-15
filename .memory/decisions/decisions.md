# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] consolidation | D-20260715-035 | Журнал консолидирован, capability classification переведена на данные

- По явному указанию пользователя журналы очищены от per-method записей: они дублировали git history и wiki digests и не являлись долговечными решениями. Полная история — в git (branch `codex/implement-plans`).
- Ниже восстановлены только долговечные решения. Per-method decisions D-20260715-007…034 упразднены: их полезное содержимое сохранено в `docs/capability-notes.md`.
- Новое решение: классификация методов TDLib — данные (одна таблица), не код. Documentation-recognizer engine и per-method drift-тесты удалены; неотревьюенные методы получают default-deny. Правила закреплены в `plans.md` («Правила работы») и `docs/tdlib-api-coverage.md`.

## [2026-07-15] restated | D-20260715-001 | Раздельная memory model

- Отдельные work/decision/problem журналы с rotation (`scripts/rotate-wiki-journal.py`); sanitized evidence в `.memory/raw/`; секреты не попадают в память ни в каком виде.

## [2026-07-15] restated | D-20260715-002 | Публичный GitHub remote

- Canonical `origin` — `https://github.com/lonmstalker/telegram-cli.git`; public visibility явно принята пользователем.

## [2026-07-15] restated | D-20260715-003 | Schema pin — exact commit

- Производственный pin — exact TDLib commit `07d3a0973f5113b0827a04d54a93aaaa9e288348` (1.8.66), никогда moving `master`. Gate: `scripts/check-tdlib-pin.py`, единственный источник drift-защиты.

## [2026-07-15] restated | D-20260715-004 | Native binary вне Git

- Собранный `tdjson` хранится в ignored content-addressed local cache; Git хранит exact policy/recipe/provenance (`vendor/tdlib/native-builds/`). Gate: `scripts/check-tdlib-native-pin.py`.

## [2026-07-15] restated | D-20260715-005 | Crash ownership для native build

- Глобальный build lock наследуется всеми watchdog paths; gated target и proof-backed recovery определяют владение при crash. Реализация в `scripts/build-tdlib-native.py` и связанных guard-тестах.

## [2026-07-15] restated | D-20260715-006 | Schema parser — pure strict subset

- Parser закреплённой схемы живёт в `telegram-core::schema`, без внешних dependencies; policy classification отделена от AST.

## [2026-07-15] restated | D-20260715-017 | Planning IDs — только документация

- Номера F001–F022 из `HARNESS.md` не появляются в executable code и machine-readable contracts. Gate: `scripts/check-planning-boundary.py`.

## [2026-07-15] accepted | D-20260715-036 | Один daemon-owner на account profile

- MVP использует один основной regular-user account; дополнительные accounts изолируются отдельными profiles с собственными canonical DB/files paths и secret references.
- Ровно один `telegramd` владеет DB profile под exclusive OS lock. CLI/MCP — lease clients одного protocol и не открывают TDLib напрямую.
- Returning authorization переиспользуется; успешный login требует `authorizationStateReady` и `getMe` identity proof. Штатная остановка — `close` с ожиданием `authorizationStateClosed`; `logOut`/`destroy` остаются destructive.
- Из `tg-analytics` принимаются только проверенные behavior contracts по owner-фазам; source CLI ownership, NATS/PostgreSQL/analytics orchestration и ручной partial raw registry не переносятся. Disposition: `docs/tg-analytics-reuse.md`.

## [2026-07-15] accepted | D-20260715-037 | Один TDJSON receive loop, `@extra` принадлежит transport

- `telegram-core` владеет ровно одним receive loop на TDJSON client; send commands и native receive выполняются на его одном backend thread, updates выходят через один ordered event stream.
- Caller не задаёт `@extra`: transport выделяет ID, регистрирует pending до send, удаляет echoed ID из ответа и явно отделяет unmatched response от update.
- Native library загружается только как exact ABI artifact через узкий `unsafe` loader. Transport shutdown не подменяет lifecycle `close`; graceful close/DB ownership остаются у daemon P2.

## [2026-07-15] accepted | D-20260715-038 | Авторизация — challenge machine с protected inputs

- Каждый observed TDLib authorization state создаёт monotonic challenge generation; принимается только input подходящего типа с текущим ID, одна submission за раз. Retry после TDLib error требует явного `submission_failed`, новый update инвалидирует старый input.
- Secret/PII auth values хранятся в redacted zeroizing wrapper; auth request не печатает payload в `Debug`. Model-visible surface позже получает только challenge ID/kind/status.
- Все pinned authorization states обрабатываются явно. `Ready` не является полным login proof без `getMe` expected-identity check; parameters/database key остаются отдельным protected provider boundary следующего Tasks-пункта.

## [2026-07-15] accepted | D-20260715-039 | Database key — protected source и fail-closed parameter gate

- Core принимает database key только как owned file descriptor, strict owner-owned `0600` regular file без symlink traversal или OS keychain reference; raw bytes и временный Base64 encoder buffer живут в zeroizing storage, а request type redacted и не логируется.
- Missing/empty key запрещён до `setTdlibParameters`: pinned TDLib иначе подставляет internal default key. `bytes` кодируются Base64 по pinned ClientJson codec, а не как plain JSON string.
- TDLib error 401 latch-ит parameters generation: authorization machine не принимает phone/QR state до явной повторной подачи protected key. Profile выбирает source reference; default backend остаётся packaging decision, DB ownership — только у будущего daemon.

## [2026-07-15] accepted | D-20260715-040 | Один transport-order sequence для core caches

- `StateReducer` принимает updates непосредственно из ordered `TdJsonEvent` stream; один monotonic sequence штампует outcome и изменённую cache entry. Unmatched responses/fatal events в update order не входят.
- Core caches сохраняют raw TDJSON objects и применяют только exact schema field patches. Partial user/chat update без гарантированного base entity fail-closed; message-send terminal state не регрессирует.
- Unknown constructor уже занимает место в sequence, но raw persistence намеренно остаётся отдельным следующим Tasks-пунктом. Gap/resync и freshness не подменяются обычным sequence и остаются своим owner phases.

## [2026-07-15] accepted | D-20260715-041 | Unknown updates остаются raw в том же sequence

- Unknown constructor не дедуплицируется и не проецируется: reducer сохраняет целый TDJSON `Value` вместе с global `UpdateSequence` в FIFO queue; read и drain сохраняют receive order.
- Field patches известных cached entities меняют только exact schema fields и не удаляют неизвестные поля full object. Это обеспечивает forward-compatible data preservation без runtime taxonomy по planning IDs.
- Queue in-memory и не подменяет durable journal/backpressure/gap recovery: эти свойства появятся только вместе с daemon/reliability consumers.

## [2026-07-15] accepted | D-20260715-042 | Core startup bounded одним deadline и snapshot boundary

- `CoreRuntime` первым отключает TDLib internal logs, затем до DB parameters и рабочих calls сверяет runtime `version`/`commit_hash` с pinned manifest. Mismatch fail-closed завершает startup.
- Вся startup chain использует один абсолютный deadline. Pending response можно явно отменить; drop/timeout удаляет correlation из receive-loop registry, а late response остаётся unmatched.
- Matched response публикует transport-order boundary. Для `getCurrentState` события до boundary заменяются authoritative snapshot, а более поздние updates сохраняются и продолжают тот же ordered reducer stream.
- Core resource shutdown не является account lifecycle: normal `close` с ожиданием `authorizationStateClosed`, singleton DB ownership и profile secret wiring остаются P2.

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
