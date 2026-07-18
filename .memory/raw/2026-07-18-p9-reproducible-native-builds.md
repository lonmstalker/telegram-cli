# P9 reproducible native builds — sanitized evidence

- Date: 2026-07-18.
- Scope: первый Tasks-пункт P9, bit-for-bit reproducibility pinned TDLib `1.8.66` для `aarch64-apple-darwin` и `x86_64-unknown-linux-gnu`.
- Source identity: commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.

## Artifact records

- macOS arm64: SHA-256 `80d17ed3da7ea209b42789ef18319099b9489819b6a78495530777c91efbeba7`, `27 571 720` bytes. Canonical provenance records two independent exact-recipe builds with identical output.
- Linux x86_64: SHA-256 `e90ca3c25ad034b7227df918816c227de2b9aef92539c994a3bd41c42d68161b`, `51 863 816` bytes. Canonical provenance records two independent pinned-container builds with identical output.
- Canonical evidence: `vendor/tdlib/native-builds/aarch64-apple-darwin.json` and `vendor/tdlib/native-builds/x86_64-unknown-linux-gnu.json`.

## Fail-closed build contract

- Rebuild берёт expected digest только из exact committed provenance с совпадающими source/target fields.
- Newly inspected artifact публикуется только при byte-for-byte совпадении SHA-256 с reference digest.
- macOS recipe добавляет stable source/build prefix maps; Linux остаётся в pinned builder image.
- Provenance `reproducibility.status=verified` требует ровно две independent builds и exact claim; negative control подменяет status на `not_verified`.

## Fresh verification

- `python3 scripts/check-tdlib-native-pin.py` — green в provenance-only mode, targets `2`, negative controls `19`.
- Native build guards — green: tar/process-group/lock/gate-order/handshake, commit provenance, input snapshots, inspection parent-death, parent-death cleanup и stale-work recovery.
- `--require-local-artifact` в этом checkpoint не использован как evidence: Docker daemon недоступен, поэтому live inspection обоих cached artifacts не был повторён. Reproducibility claim опирается на committed independent-build provenance и fail-closed digest comparison, а не на этот неполный запуск.

Секреты, host paths и Telegram identity в digest не сохранялись.
