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
