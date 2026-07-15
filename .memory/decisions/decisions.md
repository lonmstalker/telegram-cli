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
