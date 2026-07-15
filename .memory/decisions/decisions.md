# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-001 | Раздельная memory model и secret boundary

- Context: исходный Karpathy Wiki объединял действия, решения и проблемы в одном log; пользователь потребовал раздельное хранение и ротацию.
- Decision: хранить work, decisions и problems в трёх active journals с отдельными checksum archives; raw digests оставить immutable, wiki — compact synthesis.
- Rotation: active journal ограничен 16 000 Unicode-символов и 1 000 строк; generic script переносит только целые старейшие entries и оставляет минимум одну свежую.
- Secret boundary: `.env.local` является canonical local secret source, используется только через `scripts/with-env-local.sh`, никогда не читается или логируется агентом.
- Evidence: [bootstrap digest](../raw/2026-07-15-project-bootstrap.md), явная инструкция пользователя, repo patterns `tg-analytics` и `my-harness`.
- Alternatives: один общий log отклонён из-за смешения concerns; ручная monthly rotation отклонена из-за отсутствия checksum/index verification.
- Consequences: каждый checkpoint должен ссылаться на `D/P` IDs; archive shards и index rows immutable.
- Supersedes: none.

## [2026-07-15] accepted | D-20260715-002 | Публичный GitHub remote

- Context: первоначально запрашивался private remote, но созданный пользователем `lonmstalker/telegram-cli` имеет visibility `PUBLIC`.
- Decision: сохранить репозиторий публичным и использовать его как canonical `origin`.
- Evidence: явное подтверждение пользователя «он публичный, это ок»; `gh repo view lonmstalker/telegram-cli --json nameWithOwner,visibility,url,defaultBranchRef` вернул `PUBLIC`.
- Alternatives: изменение visibility на `PRIVATE` отклонено пользователем как ненужное.
- Consequences: в Git можно публиковать только sanitized artifacts; `.env.local` и другие secrets обязаны оставаться ignored и untracked.
- Supersedes: первоначальное требование private visibility из запроса на создание remote.

## [2026-07-15] accepted | D-20260715-003 | Exact initial production schema pin

- Context: F003/Q001 требовал выбрать initial production TDLib schema identity до parser/runtime implementation; moving `master` не даёт воспроизводимости.
- Decision: закрепить TDLib `1.8.66` exact commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`; manifest и vendored `td_api.tl` являются repository source of truth, а обновление требует явного reviewed diff.
- Evidence: [immutable source digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md), `vendor/tdlib/manifest.json`, `python3 scripts/check-tdlib-pin.py`.
- Alternatives: следование moving `master` отклонено как невоспроизводимое; внешний generated snapshot отклонён без повторной exact provenance-проверки.
- Consequences: parser, registry и runtime handshake используют этот commit/hash; native artifact остаётся отдельным незакрытым P0 proof.
- Supersedes: none.

## [2026-07-15] accepted | D-20260715-004 | Native artifact provenance без binary в Git

- Context: exact TDLib artifact нужен runtime и drift gates, но platform-specific dylib не должен раздувать Git; fixed cache path делает artifact/provenance пару уязвимой к crash между replace.
- Decision: хранить в Git exact source/build policy, recipe fingerprints и closed target provenance; binary хранить локально в ignored content-addressed path по SHA-256, публиковать provenance последним и под lock оставлять максимум один referenced artifact. GitHub archive получает проверяемый synthetic detached `HEAD`, потому что upstream CMake иначе генерирует `GITDIR-NOTFOUND`.
- Evidence: [native macOS arm64 digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64.md), `vendor/tdlib/native-builds/aarch64-apple-darwin.json`, `python3 scripts/check-tdlib-native-pin.py --require-local-artifact`.
- Alternatives: глобальный Homebrew TDLib отклонён из-за version drift и чужого runtime; commit dylib в Git отклонён из-за platform-specific binary weight; fixed artifact path отклонён из-за non-atomic pair update; claim о reproducible build отклонён после одной сборки.
- Consequences: CI без binary доказывает только `provenance-only`; local/runtime gate обязан требовать artifact; Linux x86_64 и bit-for-bit reproducibility остаются отдельными незакрытыми proofs.
- Supersedes: none; extends [D-20260715-003](decisions.md).

## [2026-07-15] accepted | D-20260715-005 | Crash-safe ownership native scratch и child processes

- Context: первая native build доказывала штатный cleanup, но independent review обнаружил окна parent `SIGKILL`: watchdog мог пережить owner без authoritative storage lease, inspection child не удерживал build lock, а marker-first reap finalization могла оставить невосстановимый scratch.
- Decision: global `.build.lock` является единственным authority lease и наследуется каждым build/preflight/inspection watchdog, но не target; target запускается только после durable state и complete parent handshake. Scratch имеет random identity marker, startup reaper работает только под global lock, recursively проверяет guard states и завершает удаление через sibling proof tombstone.
- Resource semantics: RSS/tree/process/log thresholds являются sampled watchdog limits; документация не называет их kernel-enforced hard caps. Build concurrency остаётся `jobs=2`, recovery не создаёт новый scratch до завершения cleanup.
- Evidence: [reviewed rebuild correction digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md), `scripts/process-group-watchdog.py`, `scripts/process-group-target-gate.py`, crash/recovery negative controls.
- Alternatives: PID/mtime ownership отклонён из-за reuse/race; per-work lease и whole-build janitor не выбраны, потому что inherited global lock закрывает cooperative child lifetime проще; unmarked reap deletion отклонён как destructive ambiguity.
- Consequences: parent death не допускает concurrent builder до child cleanup; malformed or live recovery state блокирует новый build; externally killed watchdog с live PGID fail closed вместо destructive guess.
- Supersedes: уточняет crash/resource часть [D-20260715-004](decisions.md), не меняя content-addressed artifact policy.

## [2026-07-15] accepted | D-20260715-006 | Strict TDLib schema parser отделён от policy classification

- Context: P0 требует доказуемый inventory exact `td_api.tl`, но `telegram-protocol` является wire-only boundary, а универсальный MTProto TL compiler расширил бы grammar и dependency surface без текущей потребности.
- Decision: pure parser pinned TDLib subset живёт в `telegram-core`, не имеет filesystem/network/third-party dependencies и fail closed отклоняет синтаксис вне exact subset. AST сохраняет source order, raw/structured documentation и canonical signatures; inventory отделяет builtins, constructors, methods, type families, updates и authorization states. Feature ownership/capability/risk/retry остаются отдельным reviewed classification layer.
- Resource boundary: direct input ограничен 2 MiB, рекурсивная вложенность типов — 32; parser не создаёт Cargo target или process и использует только bounded in-memory work.
- Evidence: [strict parser digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md), `crates/telegram-core/src/schema.rs`, `cargo test -p telegram-core --lib --jobs 2`.
- Alternatives: parser в `telegram-protocol` отклонён как нарушение wire boundary; новый parser crate/third-party parser-generator отклонён как лишний; универсальная TL grammar отклонена в пользу reviewed change при реальном schema update.
- Consequences: любой pin update обязан пройти hash/count gate и strict parser review; schema inventory нельзя выдавать за owner manifest, codec/router parity или runtime implementation.
- Supersedes: none; extends [D-20260715-003](decisions.md).
