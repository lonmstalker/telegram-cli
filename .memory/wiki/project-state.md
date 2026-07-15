# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap: `product.md`, `plans.md`, `HARNESS.md` (F001–F022), harness-файлы, `docs/tdlib-api-coverage.md`.
- Cargo workspace из шести пакетов; границы под gate `scripts/check-workspace-boundaries.py`; `telegramd` — working shared-session owner, остальные product binaries пока fail-closed.
- Pinned schema: TDLib `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`; 1010 functions, 2168 definitions, 184 updates, 13 auth states; gate `scripts/check-tdlib-pin.py`.
- Strict schema parser в `telegram-core::schema` (12 тестов, без внешних dependencies).
- macOS arm64 и Linux x86_64 `tdjson` с provenance в content-addressed cache; общий gate `scripts/check-tdlib-native-pin.py`, локальная проверка обоих artifacts — с `--require-local-artifact`.
- Ручное capability-ревью: 74 supported contract и 116 deferred методов сохранены в `docs/capability-notes.md`. Recognizer engine удалён ([D-20260715-035](../decisions/decisions.md)); классификация — данные с default-deny.
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

## Not implemented

- Остаток P4 и P5–P10: chat/detail и остальные workflows, reliability/policy expansion, CLI, MCP и packaging.

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты — вне model-visible interfaces.
- Protected key provider подключён к штатному daemon; [P-20260715-001](../problems/problems.md) resolved в P2.
- Linux artifact boundary закрыта в [P-20260715-003](../problems/problems.md); bit-for-bit reproducibility не заявлена.
- Неотревьюенные методы — default-deny; это валидное состояние, не блокер (см. `plans.md`, «Правила работы»).
- Следующий implementation boundary: третий Tasks-пункт P4 — chat resolve/link/invite, cache wait, optional `openChat` lease и full info.
