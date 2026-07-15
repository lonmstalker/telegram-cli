# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

## [2026-07-15] consolidation | P-20260715-012 | Журнал консолидирован

- По явному указанию пользователя журнал очищен от per-method записей и бухгалтерии ротаций. Полная история — в git. Ниже восстановлены только актуальные открытые проблемы.
- P-20260715-005 (116 методов без typed disposition) упразднена как проблема: по правилу plans.md неотревьюенный метод получает default-deny, ревью добирается пачками и ничего не блокирует. Списки методов — в `docs/capability-notes.md`.

## [2026-07-15] open | P-20260715-001 | Database key не подключён к штатному gateway

- Локальный database encryption key получен и хранится по `.env.local` contract, но штатный запуск пока не принимает его. Закрывается задачей P1 «Database encryption key из file descriptor/file secret/OS keychain».

## [2026-07-15] open | P-20260715-003 | Linux x86_64 native artifact не закреплён

- Закреплён только macOS arm64 `tdjson`. Linux x86_64 artifact с provenance — открытая задача P0; без него не начинается P9.
