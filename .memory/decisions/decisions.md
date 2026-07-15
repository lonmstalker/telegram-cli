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

## [2026-07-15] accepted | D-20260715-007 | Feature ownership генерируется отдельным fail-closed tool

- Context: P0 требует единственного owner для каждого из 1010 methods, но эта reviewed product classification не должна менять wire protocol, product CLI topology или pure schema parser.
- Decision: classification живёт в non-default `tdlib-registry-gen`, который зависит от `telegram-core` и выпускает owner-only canonical manifest. На feature допускается один unordered rule из positive name atoms; rule строит candidates и закрепляет raw method set count+SHA-256. Ноль candidates и overlap без exact override блокируют весь output; override обязан совпасть с actual candidate set, выбрать его участника и закрепить canonical signature hash. Priority, regex, first-match и fallback отсутствуют.
- Publication boundary: `check` read-only. `generate` получает один fixed-temp lease до input snapshot, работает с bounded real regular files, сохраняет open handles до identity-checked atomic rename и удаляет только owned temp. Tool не запускает network, subprocesses, threads или resident service.
- Evidence: [owner generator digest](../raw/2026-07-15-tdlib-feature-owner-generator.md), `tools/tdlib-registry-gen`, `cargo test --offline -p tdlib-registry-gen --jobs 2`, `scripts/check-workspace-boundaries.py`.
- Alternatives: generator внутри product CLI/core binary отклонён как boundary leak; handwritten generated rows — как второй source of truth; priority/fallback — как способ скрыть overlap; arbitrary rule IDs и regex — как ненужная DSL complexity; unique temp names — как unbounded concurrent footprint.
- Consequences: exact corpus policy и 1010-row artifact проходят отдельный semantic review. До них owner coverage не доказана; capability/risk/retry/codec/router/runtime fields этим решением не закрываются.
- Supersedes: none; extends [D-20260715-006](decisions.md).

## [2026-07-15] accepted | D-20260715-008 | Exact owner corpus закрепляет domain ownership, а не runtime parity

- Context: generator P0.4a мог доказать механическую полноту policy, но первый 1010/1010 draft всё ещё содержал semantic cross-domain ошибки из-за broad camel-name matches.
- Decision: принять exact mapping pinned 1010 methods к одному F001–F022 owner только после schema-derived per-feature hashes, независимого `method + NUL + feature_id + LF` digest и adversarial semantic review. Policy остаётся human-reviewed source, artifact всегда regenerated canonical tool; изменение любого owner требует нового review и обновления oracle.
- Semantic boundary: group-call/live controls принадлежат F015, message rich-text/per-chat lifecycle — F009, auth-state operations — F002, Passport/withdrawal/TON assets — F018, pure statistics — F019, network/app/test utilities — F020. F003/F005/F006/F021/F022 могут иметь ноль direct TDLib owners: cross-cutting/product surface не получает искусственные methods ради ненулевого count.
- Evidence: [owner corpus digest](../raw/2026-07-15-tdlib-feature-owner-corpus.md), `policy/tdlib-feature-owners.json`, `generated/tdlib-feature-owners.json`, corpus tests и read-only `tdlib-registry-gen check`.
- Alternatives: считать любой mechanically green 1010/1010 draft принятым отклонено после semantic audit; handwritten generated rows отклонены как второй source of truth; lexical first-match/priority отклонены решением `D-20260715-007`.
- Consequences: schema/rule/override/owner drift fail closed. Owner-only artifact нельзя использовать как доказательство capability/risk/retry, constructor/update/auth-state registry, codec/router или runtime support.
- Supersedes: none; extends [D-20260715-007](decisions.md).

## [2026-07-15] accepted | D-20260715-009 | Static capability requirements отделены от runtime truth и policy permission

- Context: P0 требует классифицировать account/auth/Premium/Business/application/DC и runtime rights для каждого method, но schema documentation не доказывает текущее состояние account и не разрешает агенту вызов.
- Decision: capability foundation использует closed bounded `CapabilityDescriptor`: exact method-level axes, additive synchronous path, typed DNF runtime evidence и parameter-value notices. Schema/owner/signature/documentation evidence проверяется fail closed; распознанный capability/runtime gate signal вне exact reviewed corpus и любое undocumented policy-сужение блокируют generation.
- Resource boundary: public core constructors и generator разделяют caps 16 clauses, 32 atoms, 32 notices и 16 synchronous values; pure generator не создаёт threads, subprocesses, network или resident state.
- Evidence: [capability foundation digest](../raw/2026-07-15-tdlib-capability-generator-foundation.md), `crates/telegram-core/src/method_capability.rs`, `tools/tdlib-registry-gen/src/capability.rs`, green workspace gates и два independent `Approved` reviews.
- Alternatives: bool flags и free-form predicates отклонены как неисполняемые/непроверяемые; permissive defaults и omission-only validation отклонены из-за скрытого narrowing; runtime account claims в generated artifact отклонены как смешение static requirements с live evidence.
- Consequences: полный 1010-method capability corpus проходит отдельный semantic review и canonical generation. Runtime evaluator, policy permission, risk, prerequisite/retry и live acceptance остаются отдельными слоями и не считаются реализованными этим решением.
- Supersedes: none; extends [D-20260715-006](decisions.md) и [D-20260715-008](decisions.md).

## [2026-07-15] accepted | D-20260715-010 | Real-schema capability grammar закрывается по exact open set

- Context: foundation принимал пять real runtime contracts, но pinned documentation содержит 193 methods с capability-like signals. Попытка сразу заполнить 1010-row policy либо остановилась бы на 188 `SchemaDrift`, либо потребовала бы ослабить fail-closed recognizer.
- Decision: закрепить exact 193-method signal set и exact 188-method open set до расширения grammar. Каждый следующий source-family task обязан уменьшать open set через closed typed model и independent review. Deferred row не считается capability coverage; canonical 1010-method artifact разрешён только после zero-open gate.
- Evidence: [capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md), corpus hashes в `tools/tdlib-registry-gen/src/capability/tests.rs`, red-green authorization test для `setCustomLanguagePack.@info`.
- Alternatives: немедленный handwritten 1010-row artifact отклонён из-за unsupported contracts; free-form expression strings отклонены как непроверяемая semantic surface; один monolithic grammar/corpus commit отклонён как слишком широкий review unit.
- Consequences: capability grammar развивается малыми reviewed commits по source family. Следующий oracle хранит disposition каждого exact signal, а method выходит из open set только после consumption всех своих signals. Runtime capability, input prerequisite, retry и lexical false positive получают раздельные lanes; до этого [P-20260715-005](../problems/problems.md) остаётся open.
- Supersedes: none; extends [D-20260715-009](decisions.md).
