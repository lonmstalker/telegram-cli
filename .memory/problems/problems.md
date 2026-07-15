# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

## [2026-07-15] open | P-20260715-001 | Штатный gateway не принимает database key file

- Evidence: `docs/feature-logic-harness/authorization.md` и baseline в `plans.md` фиксируют successful direct TDJSON access и отсутствие database-key wiring в штатном gateway.
- Impact: стандартный development path не может открыть существующую encrypted TDLib DB через согласованный file reference.
- Reproduction boundary: не повторять login и не выводить key; проверять только через existing encrypted session и protected file provider.
- Status: open; implementation отсутствует.
- Next check: в P1/F002 добавить file/keychain secret provider, negative wrong-key test и returning Ready/getMe/Closed acceptance.
- Related decisions: [D-20260715-001](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-002 | Exact TDLib native artifact не закреплён

- Evidence: `plans.md` требует exact native build; `vendor/tdlib/manifest.json` и `python3 scripts/check-tdlib-pin.py` пока доказывают только source/schema/license identity и не содержат target-specific artifact hash/provenance.
- Impact: объединённый P0 task нельзя закрыть; runtime link/version handshake и reproducibility для macOS arm64/Linux x86_64 не доказаны.
- Status: open; matching native artifact отсутствует в repository source of truth.
- Next check: выполнить bounded build exact commit отдельно для каждого target, сохранить artifact digest/build provenance вне тяжёлых build trees и добавить offline verifier.
- Related decisions: [D-20260715-003](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-002 | Exact macOS arm64 TDLib artifact закреплён

- Evidence: `vendor/tdlib/native-builds/aarch64-apple-darwin.json`, [native digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64.md), green `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`.
- Resolution: exact TDLib `1.8.66` dylib собран из pinned commit, опубликован content-addressed hash и проверен через Mach-O/dependency/export/version/commit/no-DB smoke.
- Status: resolved для macOS arm64; прежний общий gap разделён, Linux x86_64 перенесён в `P-20260715-003`.
- Remaining boundary: одна сборка не доказывает bit-for-bit reproducibility.
- Related decisions: [D-20260715-003](../decisions/decisions.md), [D-20260715-004](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-003 | Linux x86_64 native artifact не закреплён

- Evidence: committed native provenance существует только для `aarch64-apple-darwin`; Linux artifact/provenance отсутствует.
- Impact: supported-target checkbox и P9 reproducible-build acceptance для Linux остаются open; macOS proof нельзя экстраполировать на server target.
- Status: open; Linux build не запускался и не заявляется.
- Next check: отдельный bounded Linux x86_64 build с exact source/policy, target-specific dependency audit и artifact smoke.
- Related decisions: [D-20260715-004](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-004 | Native build crash recovery допускал orphan и storage leak

- Evidence: independent review первой macOS build обнаружил parent-`SIGKILL` окна в process-group ownership, archive/OpenSSL input snapshots, inspection lease propagation и `.work-*` finalization; первый digest и `W-20260715-007` фиксируют только pre-correction состояние.
- Impact: повторный build нельзя было безопасно запускать: orphan watchdog/target или stale scratch до 4 GiB могли пережить owner, а повторные аварии увеличивали footprint.
- Status: open на review checkpoint; штатный success-path cleanup недостаточен как crash proof.
- Next check: TDD negative controls для parent death, real scratch recovery, inspection death, fragmented handshake и reap finalization; затем одна bounded rebuild и новый provenance.
- Related decisions: [D-20260715-004](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-004 | Native build crash recovery доказан negative controls

- Evidence: [reviewed rebuild correction digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md), `scripts/test-tdlib-native-parent-death-guard.py`, `scripts/test-tdlib-native-stale-work-recovery.py`, `scripts/test-tdlib-native-inspection-parent-death.py` и green post-build review.
- Resolution: inherited global lease удерживает single-owner boundary до cleanup всех cooperative watchdogs; gated target handshake, recursive guard-state validation и proof-backed reap finalization закрывают найденные crash windows. Malformed/symlink/ambiguous state остаётся fail closed.
- Status: resolved для implemented macOS builder; после rebuild scratch/process leftovers `0`, content cache `1`, total `target` 42 MiB.
- Remaining boundary: sampled resource thresholds не являются kernel isolation; Linux artifact остаётся [P-20260715-003](problems.md), bit-for-bit reproducibility не доказана.
- Related decisions: [D-20260715-005](../decisions/decisions.md).

## [2026-07-15] resolved correction | P-20260715-004 | Archive snapshot и OpenSSL hash validation разделены

- Corrects: evidence wording `archive/OpenSSL input snapshots` в open entry не означало две private copies.
- Exact evidence: source archive имеет private immutable snapshot; exact resolved OpenSSL Cellar archives остаются на canonical path и проходят pre/post bytes/SHA-256 validation.
- Status: `P-20260715-004` остаётся resolved; correction сужает durable claim и не меняет crash-recovery result.
- Related decisions: [D-20260715-005](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-005 | 188 pinned runtime-signal methods не имеют typed disposition

- Evidence: [capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md); exact 193-method signal set SHA-256 `cbe074...8706`, exact 188-method open set SHA-256 `c9e513...0a34`; corpus test требует `SchemaDrift` для каждой open row.
- Impact: canonical 1010-method capability artifact и P0 capability checkbox заблокированы. Замена unsupported contracts на permissive `always` дала бы ложное расширение API.
- Status: open; baseline измерен, typed source families ещё не реализованы.
- Next check: добавить exact per-signal disposition oracle и bounded `ChatKind` atom для conditional chat rights, затем closed `MessageProperties` facts; после каждого reviewed task пересчитать exact open set. Закрытие требует zero-open gate и independent semantic review.
- Related decisions: [D-20260715-009](../decisions/decisions.md), [D-20260715-010](../decisions/decisions.md).
