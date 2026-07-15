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

## [2026-07-15] open update | P-20260715-005 | Exact open set уменьшен до 187 methods

- Evidence: [ChatKind capability digest](../raw/2026-07-15-tdlib-chat-kind-capability.md); exact 193-method signal set не изменился, supported set вырос до 6 с SHA-256 `ea3222...99a9`, open set теперь 187 с SHA-256 `beea6c...3c03`.
- Transition: `unpinChatMessage` получил complete typed disposition через five-branch `ChatKind` DNF и больше не входит в open set. Остальные 187 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Status: open; zero-open gate не достигнут.
- Next check: добавить exact per-signal disposition artifact и следующую closed source family (`MessageProperties`/object-field facts), не смешивая runtime capability с prerequisite/retry/lexical lanes.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-011](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Per-signal oracle принят, exact open set уменьшен до 185 methods

- Evidence: [per-signal disposition digest](../raw/2026-07-15-tdlib-runtime-signal-dispositions.md); exact 193-method scan развёрнут в 208 sources и 398 keys. Terminal complete set содержит 8 methods; open-set SHA-256 `b4b68de...009c8`.
- Transition: exact `getChatBoostFeatures` и `getChatBoostLevelFeatures` lexical vocabulary признана non-gate; explicit consumed-key equality теперь не допускает partial completion. Остальные 185 methods остаются deferred и дают `SchemaDrift`.
- Status: open; zero-open gate не достигнут.
- Next check: добавить exact `MessageProperties` schema vocabulary и typed quantified message facts без premature consumption mixed contracts.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | MessageProperties family уменьшила open set до 156 methods

- Evidence: [MessageProperties capability digest](../raw/2026-07-15-tdlib-message-properties-capabilities.md); schema-derived family exhaustive разделена на 29 complete и 4 deferred methods, 59 keys consumed, 11 mixed keys сохранены deferred.
- Transition: exact ordered vocabulary, source text, identifier space и `One/Each` cardinality terminally disposition 29 methods. Terminal complete set теперь 37; open-set SHA-256 `e3ce3e31e2f024513cb1f04e5d4f116b05e31eca6483302532da1395197b8e54`.
- Status: open; zero-open gate не достигнут, 156 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать group-call/full-info/option/admin/object-field source families, сохраняя prerequisite/retry и mixed invocation lanes раздельными.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-013](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | ChatBoost link vocabulary уменьшила open set до 155 methods

- Evidence: [exact lexical digest](../raw/2026-07-15-tdlib-chat-boost-link-non-gate.md); один `ChatBoostReference` key terminally classified без capability claim.
- Transition: `getChatBoostLinkInfo` выходит из open set; terminal complete set 38, open-set SHA-256 `4ed02dd1adbb3c87c61b4f6fccc009e331670c22fa7ac0c406e782d917ef9c1b`.
- Status: open; 155 methods остаются deferred. Next: typed group-call/object-field families.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | GroupCall family уменьшила open set до 143 methods

- Evidence: [GroupCall capability digest](../raw/2026-07-15-tdlib-group-call-capabilities.md); schema-derived family exhaustive разделена на 12 complete и 2 argument-dependent methods, 38 keys consumed, 1 setting-semantics key terminally non-gate, 6 keys сохранены deferred.
- Transition: exact kind/property/cardinality DNF terminally disposition 12 methods. Supported typed set теперь 47, terminal complete set 50, open-set SHA-256 `a6e5b3c9d53a657e7ee3f9f4f5ed4bad7043292418b08849273d406f513b3a12`.
- Status: open; zero-open gate не достигнут, 143 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать full-info/option/admin/object-field source families; runtime evaluator отдельно обязан fail closed на stale/unknown group-call-message evidence.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-014](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | SupergroupFullInfo family уменьшила open set до 138 methods

- Evidence: [SupergroupFullInfo capability digest](../raw/2026-07-15-tdlib-supergroup-full-info-capabilities.md); schema-derived family exhaustive разделена на 5 complete и 7 mixed methods, 12 keys consumed, 2 cross-token false positives terminally non-gate, 18 keys сохранены deferred.
- Transition: exact property/target DNF terminally disposition five methods. Supported typed set теперь 52, terminal complete set 55, open-set SHA-256 `a2028d7acb1055b4c5fc5a0fda69cf4a8c09200feea2fd3d386596e24fc9aa67`.
- Status: open; zero-open gate не достигнут, 138 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать option/admin/object-field source families; runtime evaluator отдельно обязан fail closed на stale/unknown full-info evidence.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-015](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Runtime boolean option family уменьшила open set до 137 methods

- Evidence: [runtime boolean option digest](../raw/2026-07-15-tdlib-runtime-boolean-options.md); exact family разделена на one complete method-level gate и two mixed deferred methods. Один key consumed, семь mixed keys сохранены deferred.
- Transition: `setNewChatPrivacySettings` получает exact typed option requirement. Supported typed set теперь 53, terminal complete set 56, open-set SHA-256 `c05b282773cfd9ecaa1e8ab0c24a0ad08d7589a1fbf05a08901fe355db6c959e`.
- Status: open; zero-open gate не достигнут, 137 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать admin/object-field/mixed source families; runtime evaluator отдельно обязан fail closed на wrong-typed, missing или generation-stale option evidence.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-016](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-006 | Planning taxonomy загрязнила core и generated contracts

- Evidence: `telegram_core::FeatureId`, owner generator/CLI, 1010-row policy/artifact и capability owner field existed through `W-20260715-020`; user correction rejected this architecture.
- Impact: executable model depended on arbitrary plan numbering, duplicated schema identity and added 20k lines without product runtime semantics.
- Status: open at correction start; W021 owner work stopped before commit.
- Next check: remove all runtime taxonomy surfaces, add fail-closed repository gate and independent review.
- Related decisions: superseded `D-20260715-007`/`D-20260715-008`.

## [2026-07-15] resolved | P-20260715-006 | Runtime contracts отвязаны от planning inventory

- Evidence: [planning-taxonomy removal correction](../raw/2026-07-15-planning-taxonomy-removal.md), green `python3 scripts/check-planning-boundary.py`, 69 Rust tests, Clippy and repeat implementation audit.
- Resolution: numeric type, owner engine/CLI/policy/artifact и capability owner field удалены; schema identity и семантические modules являются current boundary.
- Status: resolved; seven negative controls покрывают matcher, real discovery, root/script formats и file/root symlink fail-closed.
- Remaining boundary: documentation IDs остаются навигацией; full registry/runtime всё ещё open и не заявляется.
- Related decisions: [D-20260715-017](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Username owner family уменьшила open set до 133 methods

- Evidence: [supergroup username owner digest](../raw/2026-07-15-tdlib-supergroup-username-owner-capabilities.md); exact owner-signal family исчерпывающе разделена на 4 new complete, 1 prior complete и 13 mixed/deferred methods.
- Transition: четыре username-management methods получают exact `ChatKind AND ChatOwner` DNF. Supported typed set теперь 57, terminal complete set 60, open-set SHA-256 `cd2b13cc68f18956f113592b505ec4469c564e3f7ce4298e7e4093b172e5a914`.
- Status: open; zero-open gate не достигнут, 133 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать следующие exact semantic families; runtime evaluator обязан fail closed на stale/unknown owner evidence.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-018](../decisions/decisions.md).
