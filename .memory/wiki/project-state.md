# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap: `product.md`, `plans.md`, `HARNESS.md` (F001–F022), harness-файлы, `docs/tdlib-api-coverage.md`.
- Cargo workspace из шести пакетов; границы под gate `scripts/check-workspace-boundaries.py`; product binaries — fail-closed заглушки.
- Pinned schema: TDLib `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`; 1010 functions, 2168 definitions, 184 updates, 13 auth states; gate `scripts/check-tdlib-pin.py`.
- Strict schema parser в `telegram-core::schema` (12 тестов, без внешних dependencies).
- macOS arm64 и Linux x86_64 `tdjson` с provenance в content-addressed cache; общий gate `scripts/check-tdlib-native-pin.py`, локальная проверка обоих artifacts — с `--require-local-artifact`.
- Ручное capability-ревью: 74 supported contract и 116 deferred методов сохранены в `docs/capability-notes.md`. Recognizer engine удалён ([D-20260715-035](../decisions/decisions.md)); классификация — данные с default-deny.
- Существующая зашифрованная сессия ранее достигала Ready/getMe; database key получен; `.env.local` contract (mode `0600`, protected loader) настроен.
- Canonical GitHub remote: `https://github.com/lonmstalker/telegram-cli.git` (public, принято пользователем).
- P0 accepted: `tg-analytics@e35c54ce213aa170fb0b411eab614485424b3e60` audited from clean archive (97 tests); phase-neutral patterns перенесены, runtime contracts распределены по owner-фазам в `docs/tg-analytics-reuse.md`.
- Account/session model [D-20260715-036](../decisions/decisions.md): один `telegramd` owner на profile, CLI/MCP lease clients, returning auth с `Ready` + `getMe` proof.
- P1 transport [D-20260715-037](../decisions/decisions.md): один backend/receive thread, transport-owned `@extra`, ordered raw event stream; pinned macOS native `getOption version` smoke green.
- P1 authorization [D-20260715-038](../decisions/decisions.md): exhaustive challenge machine, exact QR/phone/code/2FA/email/device/registration requests, stale/duplicate input fail closed, auth values redacted/zeroizing.
- P1 database key [D-20260715-039](../decisions/decisions.md): FD/strict `0600` file/macOS-Linux keychain sources, zeroizing raw bytes, Base64 TDJSON, empty-key preflight deny и wrong-key 401 latch без phone fallback.
- P1 reducer [D-20260715-040](../decisions/decisions.md): transport-order sequence и versioned auth/user/chat/group/file/connection/message-send caches; partial updates require base entity, send terminal states не регрессируют.
- P1 unknown updates [D-20260715-041](../decisions/decisions.md): unknown constructors сохраняются exact raw Value в FIFO sequence; field patches известных objects сохраняют будущие поля.

## Not implemented

- Остальной runtime P1–P10: deadlines/cancellation/startup handshake, daemon, generated registry, capability-таблица, workflows, policy, CLI, MCP, packaging.

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты — вне model-visible interfaces.
- Core key provider готов; wiring в штатный daemon всё ещё открыт как [P-20260715-001](../problems/problems.md).
- Linux artifact boundary закрыта в [P-20260715-003](../problems/problems.md); bit-for-bit reproducibility не заявлена.
- Неотревьюенные методы — default-deny; это валидное состояние, не блокер (см. `plans.md`, «Правила работы»).
- Следующий implementation boundary: шестой Tasks-пункт P1 — deadlines, cancellation, startup `getCurrentState` и runtime version handshake.
