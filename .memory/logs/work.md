# Work Journal

Active append-only checkpoints. Решения и проблемы хранятся отдельно и здесь только упоминаются по ID.

## [2026-07-15] consolidation | W-20260715-039 | Чистка переусложнения и консолидация памяти

- По явному указанию пользователя выполнена чистка: журналы и wiki сжаты до текущего состояния, per-method записи и архивные ротации удалены (история — в git).
- Удалён capability documentation-recognizer engine (`tools/tdlib-registry-gen/src/capability*`, `telegram-core::method_capability`, ~14 000 строк): per-method drift-тесты дублировали schema pin, hash-pinned счётчики были self-referential, модуль-на-семейство нарушал «классификация — данные». Решение: [D-20260715-035](../decisions/decisions.md).
- Отревьюенные знания сохранены машинной выгрузкой перед удалением: 74 supported contract и 116 deferred методов в `docs/capability-notes.md`.
- `plans.md` переписан: правила работы против переусложнения, зоны ответственности, укрупнённые задачи, критерии приёмки с объяснениями. `docs/tdlib-api-coverage.md` обновлён под data-table подход.
- Verification: `cargo test --workspace --all-targets` (12 tests), `cargo clippy -D warnings`, `check-workspace-boundaries.py`, `check-planning-boundary.py`, `check-tdlib-pin.py` — green.
- Открытые границы: [P-20260715-001](../problems/problems.md), [P-20260715-003](../problems/problems.md); P0 остаётся in_progress (Linux target, перенос из tg-analytics).

## [2026-07-15] completed | W-20260715-040 | Закреплён Linux x86_64 TDLib artifact

- Закрыт Tasks-пункт P0 «Определить supported targets»: exact TDLib `1.8.66`/`07d3a0973f5113b0827a04d54a93aaaa9e288348` собран для `x86_64-unknown-linux-gnu` в pinned Debian 12 amd64 builder; artifact хранится вне Git по [D-20260715-004](../decisions/decisions.md).
- Provenance `vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json` закрепляет builder, packages, source preparation, ELF/SONAME/dependencies/exports, runtime version/commit и no-client smoke; общий native gate теперь проверяет оба supported target и 19 trust-boundary negative controls.
- Artifact: SHA-256 `e90ca3c25ad034b7227df918816c227de2b9aef92539c994a3bd41c42d68161b`, 51 863 816 bytes, `ELF64` x86-64, `libtdjson.so.1.8.66`, без RPATH/RUNPATH и без созданных DB-файлов. Bit-for-bit reproducibility не заявлена.
- Внешнее доказательство: [Linux x86_64 native build digest](../raw/2026-07-15-tdlib-1.8.66-native-linux-x86_64.md). Проблема [P-20260715-003](../problems/problems.md) переведена в `resolved`.
- Verification: `python3 scripts/check-tdlib-native-pin.py`; `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`; обязательные workspace/clippy/check scripts/wiki gates перед коммитом.
- Следующий Tasks-пункт P0: перенести только доказанно reusable части `tg-analytics` без NATS/Postgres/analytics orchestration.
