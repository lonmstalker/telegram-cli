# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

## [2026-07-15] consolidation | P-20260715-012 | Журнал консолидирован

- По явному указанию пользователя журнал очищен от per-method записей и бухгалтерии ротаций. Полная история — в git. Ниже восстановлены только актуальные открытые проблемы.
- P-20260715-005 (116 методов без typed disposition) упразднена как проблема: по правилу plans.md неотревьюенный метод получает default-deny, ревью добирается пачками и ничего не блокирует. Списки методов — в `docs/capability-notes.md`.

## [2026-07-15] open | P-20260715-001 | Database key не подключён к штатному gateway

- Локальный database encryption key получен и хранится по `.env.local` contract, но штатный запуск пока не принимает его. Закрывается задачей P1 «Database encryption key из file descriptor/file secret/OS keychain».

## [2026-07-15] open | P-20260715-003 | Linux x86_64 native artifact не закреплён

- Закреплён только macOS arm64 `tdjson`. Linux x86_64 artifact с provenance — открытая задача P0; без него не начинается P9.

## [2026-07-15] resolved | P-20260715-003 | Linux x86_64 native artifact закреплён

- TDLib `1.8.66` собран exact pinned builder для `x86_64-unknown-linux-gnu`; artifact SHA-256 `e90ca3c25ad034b7227df918816c227de2b9aef92539c994a3bd41c42d68161b`, provenance — `vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json`.
- `python3 scripts/check-tdlib-native-pin.py --require-local-artifact` проверяет оба supported target, Linux ELF identity, SONAME, dependencies, TDJSON exports, runtime version/commit и отсутствие DB-файлов в no-client smoke.
- Bit-for-bit reproducibility остаётся незаявленной границей, но не является acceptance-критерием P0.

## [2026-07-15] narrowed | P-20260715-001 | Core provider готов, daemon wiring ещё отсутствует

- P1 protected provider и `setTdlibParameters` integration готовы: FD/file/keychain sources, empty-key preflight deny и wrong-key 401 latch проверены synthetic tests.
- Проблема остаётся открытой до P2: product binaries всё ещё fail-closed заглушки, поэтому штатный `telegramd` пока не выбирает profile secret reference и не доказывает returning `Ready`. Исходная фраза «закрывается задачей P1 provider» уточнена по live repository truth.

## [2026-07-15] narrowed | P-20260715-001 | Core returning path доказан, product daemon всё ещё не wired

- P1 `CoreRuntime` и protected loader свежо доказали returning `Ready`, `getMe` и normal Closed без нового login; wrong/missing-key native boundary также green.
- Проблема остаётся открытой уже только на product boundary P2: `telegramd` пока не выбирает profile key reference, не владеет runtime/DB и не предоставляет lifecycle через protocol.
