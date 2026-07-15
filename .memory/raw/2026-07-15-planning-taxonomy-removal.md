# Planning-taxonomy removal correction

Дата проверки: 2026-07-15.

## Причина correction

Документационный inventory `F001`…`F022` был ошибочно превращён в executable taxonomy: `telegram_core::FeatureId`, owner rule engine/CLI, 1010-row owner policy/artifact и поле capability contract. Это не выражало предметную модель Telegram/TDLib и добавляло ручную классификацию поверх стабильной schema identity.

Пользователь явно уточнил границу: делить реализацию по именованным предметным файлам/модулям допустимо; переносить номера planning sections в код нельзя.

## Исправленная архитектура

- Planning IDs остаются только навигацией `HARNESS.md`, `plans.md` и feature harness.
- Raw method/constructor identity — exact TDLib name плюс canonical signature/schema evidence.
- Runtime/tooling modules имеют семантические имена: `schema`, `method_capability`, `capability`; будущие bounded contexts также именуются по домену, а не по номеру плана.
- `tdlib-registry-gen` стал library-only tooling package; owner engine, publication CLI, policy и generated artifact удалены.
- Pure capability generator принимает только pinned schema bytes и capability policy bytes. Невалидируемый opaque vendor-manifest input/evidence удалён по review finding; schema pin проверяется отдельным gate.
- Capability policy format повышен `6` -> `7`; method rows больше не содержат planning owner field и связаны напрямую с method/signature/documentation hashes.

## TDD и review corrections

- Первый architecture gate упал на core enum, owner engine/tests, policy/artifact и capability field.
- Compile-red после упрощения API зафиксировал четыре call sites старой трёхаргументной `generate`; production API затем сокращён до schema + policy.
- Отдельная временная `scripts/.planning-boundary-red.py` доказала false negative первого checker: он ошибочно вернул green, потому что не сканировал scripts.
- Исправленный checker сканирует Rust, scripts, все root machine files, `.cargo` и `.github`, имеет byte cap и семь filesystem/matcher negative controls. File и inspected-root symlink отклоняются fail closed.
- Independent reviewer нашёл opaque manifest evidence, узкий discovery scope, file/root symlink bypass и stale harness claims; все findings исправлены до commit. После memory audit и отдельных root `build.rs`/symlink-root repro финальный whole-diff verdict — `APPROVED`, findings отсутствуют.

## Свежие проверки

- `cargo fmt --all -- --check` — green.
- `CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2 cargo test --locked --offline --workspace --all-targets --jobs 2 --quiet` — 69 passed, 0 failed.
- `CARGO_BUILD_JOBS=2 cargo clippy --locked --offline --workspace --all-targets --jobs 2 --quiet -- -D warnings` — green.
- `python3 scripts/check-workspace-boundaries.py` — green, 4 negative controls.
- `python3 scripts/check-planning-boundary.py` — green, 7 negative controls.
- Schema/native provenance/skeleton/diff gates — green.
- Correction удаляет более 20 000 строк owner-taxonomy implementation; `target` 150 MiB; `cargo`, `rustc`, `telegramd`, `tdjson`, `tdlib-registry-gen` processes отсутствуют.

## Граница доказанного

Correction не реализует full 1010-method registry, оставшиеся 137 runtime-signal dispositions, runtime evaluator, daemon/authorization/CLI или live acceptance. Исторические owner digests остаются immutable evidence superseded implementation и не описывают current architecture.
