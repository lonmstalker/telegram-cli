# Текущее состояние проекта

Последняя проверка: 2026-07-15.

## Verified

- Документационный bootstrap создан: product, living plan, HARNESS, TDLib coverage contract и F001–F022.
- Pinned planning baseline описывает TDLib 1.8.66: 1010 functions, 2168 definitions, 184 updates и 13 authorization states.
- Existing encrypted TDLib session ранее достигла Ready/getMe и была закрыта через authorizationStateClosed.
- SSH-доступ и серверный database-key path проверены без вывода значения.
- `.env.local` создан как ignored mode-`0600` source; env contract опубликован без значений, loader проверен.
- Karpathy Wiki использует отдельные work/decision/problem journals и checksum-backed rotation.
- Canonical GitHub remote: `https://github.com/lonmstalker/telegram-cli.git`; public visibility явно принята пользователем.
- P0 начат: Cargo workspace содержит шесть целевых packages, а dependency/target/default-member boundaries защищены executable contract с negative controls.
- До появления runtime все четыре binary entrypoint fail closed; process guard ограничен timeout и очищает всю отдельную process group.
- Initial production schema pin: TDLib `1.8.66`, exact commit `07d3a097...`; vendored schema hash/counts проверяются offline с negative controls.
- Exact macOS arm64 `tdjson` подтверждён crash-safe reviewed rebuild: artifact SHA-256 `5dbd3009...6852e7e`, 27 654 296 bytes; Mach-O/dependencies/exports/version/commit и no-DB smoke проверены.
- Global build lock наследуется всеми watchdog paths; gated handshake, recursive stale recovery и proof-backed finalization проверены parent/inspection `SIGKILL` controls. RSS/tree limits являются sampled thresholds, не kernel hard caps.
- Native binary хранится в ignored content-addressed cache; Git хранит exact policy/recipe/provenance. Одна сборка помечена `reproducibility=not_verified`.
- Strict Rust parser в `telegram-core` разбирает полный pinned corpus без сторонних dependencies: 2168 definitions = 9 builtins + 2159 object constructors, 1010 methods, 745 type families, 184 updates и 13 authorization states. Documentation сохраняется raw и structured, signatures canonical; input cap 2 MiB, type depth cap 32. Independent re-review — Approved.
- Historical owner rule engine и 1010-row owner corpus удалены в `W-20260715-021`: planning IDs не являются runtime taxonomy. Их immutable raw digests сохранены только как история superseded implementation.
- Non-default `tdlib-registry-gen` теперь library-only tooling package с семантическим модулем `capability`; product packages от него не зависят.
- P0.5a capability foundation закрепляет closed account/auth/entitlement/application/DC vocabularies, additive synchronous path, typed bounded runtime DNF и parameter notices. Pure generator требует exact schema/method/signature/documentation evidence, отклоняет распознанные unsupported capability/runtime gate signals и скрытое policy-сужение.
- P0.5b0 evidence baseline связывает current recognizer с exact 193-method signal set и 188-method fail-closed open set. Authorization validation читает все structured documentation tags; `setCustomLanguagePack.@info` больше не теряет pre-authorization contract. Open rows не считаются capability coverage.
- P0.5b1 добавляет closed `ResolvedChatKind`/`ChatKindCondition`, exact four-constructor `ChatType` pin и conditional DNF для шести real methods. `unpinChatMessage` полностью dispositioned; exact open set уменьшен до 187 без расширения recognizer. Capability policy format — `2`; independent reviews — Approved.
- P0.5b2 разворачивает 193-method recognizer в exact 208 source tags/398 signal keys. Explicit consumed-key equality запрещает partial completion; exact normalized lexical exceptions уменьшают open set до 185 без изменения capability format. Два independent reviews — Approved.
- P0.5b3 закрепляет ordered 39-field `messageProperties`, closed 36-value `MessageCapability` и typed `One/Each` subjects. Полные contracts приняты для 29 из 33 schema-derived methods; четыре mixed methods остаются deferred. Open set уменьшен до 156, capability format — `3`; три independent reviews — Approved.
- P0.5b4 exact-match классифицирует `getChatBoostLinkInfo` как lexical non-gate: `internalLinkTypeChatBoost` является типом входной ссылки, не runtime capability. Open set уменьшен до 155; два independent reviews — Approved.
- P0.5b5 закрепляет ordered 32-field `groupCall`, 7-field `groupCallMessage`, closed kind/property vocabulary и 12 exact typed DNF. Два argument-dependent methods остаются deferred; `only by administrators` у mute-new-participants классифицировано как setting-value non-gate. Open set уменьшен до 143, capability format — `4`; три independent reviews — Approved.
- P0.5b6 закрепляет ordered 42-field `supergroupFullInfo`, closed eight-property vocabulary и 5 exact typed DNF. Семь mixed methods остаются deferred; два cross-token `OnlyIfAdministrator` matches классифицированы как exact lexical non-gates. Open set уменьшен до 138, capability format — `5`; Rust/evidence reviews и independent oracle audit — Approved.
- P0.5b7 закрепляет exact four-constructor `OptionValue`, `getOption`/`updateOption`, closed three-name runtime option vocabulary и method-level gate для `setNewChatPrivacySettings`. Два mixed methods остаются deferred; open set уменьшен до 137. После удаления owner fields capability format повышен до `7`; semantic capability contents не изменились.
- P0.5b8 закрепляет в semantic module четыре exact username-management contracts. Explicit DNF требует `supergroup/channel AND owner`; 13 mixed owner methods остаются deferred. Open set уменьшен до 133, capability format остаётся `7`.
- P0.5b9 закрепляет в semantic module два exact invite-link create/replace contracts. Explicit DNF требует basic-group/supergroup/channel kind и administrator `can_invite_users` right; 9 own/other-link methods остаются deferred. Open set уменьшен до 131, capability format остаётся `7`.
- P0.5b10 закрепляет в semantic module четыре exact supergroup/channel setting contracts с kind, administrator/member right и account boundary. Четыре boost/guard-input/ordinary-kind methods остаются deferred; reviewer correction исключила broad claim для gigagroup/monoforum. Open set уменьшен до 127, capability format остаётся `7`.
- P0.5b11 объединяет setting contracts в `capability/chat_settings.rs` и добавляет exact permissions/description/slow-mode DNF. Initial reviewer P2 вернул title/photo в deferred из-за bot/basic-group appointed-admin guard; post-fix review — Approved. Open set уменьшен до 124, capability format остаётся `7`.
- P0.5b12 исправляет существовавший `addChatMember` overclaim: dispatcher допускает только regular user, а `is_direct_messages_group` target запрещён deeper handler. Неполный generic contract удалён; current open set — 125, capability format остаётся `7`.
- Planning boundary gate с семью negative controls запрещает `FeatureId`, numeric planning IDs и owner-manifest taxonomy в runtime/tooling/machine contracts; real discovery проверяет scripts/all root machine files, file/root symlink fail closed.

## Not implemented

- Linux x86_64 TDLib artifact, typed dispositions для 125 runtime-signal methods, reviewed 1010-method capability corpus, risk/prerequisite/retry classification, generated full schema registry, singleton daemon, рабочий product CLI и MCP ещё не созданы; текущие product binaries являются только fail-closed skeleton.
- Stateful request-chain engine, retry/reconciliation, policy, metrics и agent skill остаются планом.

## Active boundary

- Full API означает L0–L2 для всей pinned schema; curated workflows и live proofs учитываются отдельно.
- Секреты находятся вне model-visible interfaces.
- Gateway key wiring остаётся [P-20260715-001](../problems/problems.md).
- Linux target proof остаётся [P-20260715-003](../problems/problems.md); macOS artifact нельзя считать доказательством Linux или bit-for-bit reproducibility.

## Evidence

- [Bootstrap digest](../raw/2026-07-15-project-bootstrap.md)
- [D-20260715-001](../decisions/archive/2026-07-15--2026-07-15-001.md)
- [D-20260715-002](../decisions/archive/2026-07-15--2026-07-15-002.md)
- [W-20260715-005](../logs/archive/2026-07-15--2026-07-15-004.md)
- [D-20260715-003](../decisions/archive/2026-07-15--2026-07-15-002.md)
- [W-20260715-006](../logs/archive/2026-07-15--2026-07-15-004.md)
- [Reviewed native macOS arm64 correction digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md)
- [D-20260715-004](../decisions/decisions.md)
- [D-20260715-005](../decisions/decisions.md)
- [W-20260715-008](../logs/archive/2026-07-15--2026-07-15-006.md)
- [Strict schema parser/inventory digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md)
- [D-20260715-006](../decisions/decisions.md)
- [W-20260715-009](../logs/archive/2026-07-15--2026-07-15-008.md)
- [TDLib owner generator digest](../raw/2026-07-15-tdlib-feature-owner-generator.md)
- [D-20260715-007](../decisions/archive/2026-07-15--2026-07-15-006.md)
- [W-20260715-010](../logs/work.md)
- [TDLib owner corpus digest](../raw/2026-07-15-tdlib-feature-owner-corpus.md)
- [D-20260715-008](../decisions/archive/2026-07-15--2026-07-15-007.md)
- [W-20260715-011](../logs/work.md)
- [TDLib capability generator foundation digest](../raw/2026-07-15-tdlib-capability-generator-foundation.md)
- [TDLib capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md)
- [TDLib ChatKind capability semantics](../raw/2026-07-15-tdlib-chat-kind-capability.md)
- [TDLib per-signal runtime disposition oracle](../raw/2026-07-15-tdlib-runtime-signal-dispositions.md)
- [TDLib MessageProperties capability digest](../raw/2026-07-15-tdlib-message-properties-capabilities.md)
- [TDLib GroupCall capability digest](../raw/2026-07-15-tdlib-group-call-capabilities.md)
- [TDLib SupergroupFullInfo capability digest](../raw/2026-07-15-tdlib-supergroup-full-info-capabilities.md)
- [TDLib runtime boolean option capability digest](../raw/2026-07-15-tdlib-runtime-boolean-options.md)
- [TDLib supergroup username owner capability digest](../raw/2026-07-15-tdlib-supergroup-username-owner-capabilities.md)
- [TDLib chat invite-link creation capability digest](../raw/2026-07-15-tdlib-chat-invite-link-creation-capabilities.md)
- [TDLib supergroup setting-right capability digest](../raw/2026-07-15-tdlib-supergroup-setting-right-capabilities.md)
- [TDLib supergroup setting ordinary-kind correction](../raw/2026-07-15-tdlib-supergroup-setting-ordinary-kind-correction.md)
- [TDLib chat setting-right capability digest](../raw/2026-07-15-tdlib-chat-setting-right-capabilities.md)
- [TDLib addChatMember overclaim correction](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md)
- [TDLib getChatBoostLinkInfo lexical non-gate digest](../raw/2026-07-15-tdlib-chat-boost-link-non-gate.md)
- [D-20260715-009](../decisions/archive/2026-07-15--2026-07-15-008.md)
- [W-20260715-012](../logs/archive/2026-07-15--2026-07-15-011.md)
- [W-20260715-013](../logs/work.md)
- [D-20260715-011](../decisions/archive/2026-07-15--2026-07-15-010.md)
- [D-20260715-012](../decisions/decisions.md)
- [W-20260715-014](../logs/work.md)
- [W-20260715-015](../logs/work.md)
- [D-20260715-013](../decisions/decisions.md)
- [W-20260715-016](../logs/work.md)
- [W-20260715-017](../logs/work.md)
- [W-20260715-018](../logs/work.md)
- [D-20260715-015](../decisions/archive/2026-07-15--2026-07-15-016.md)
- [W-20260715-019](../logs/archive/2026-07-15--2026-07-15-017.md)
- [D-20260715-016](../decisions/decisions.md)
- [W-20260715-020](../logs/work.md)
- [Planning-taxonomy removal correction](../raw/2026-07-15-planning-taxonomy-removal.md)
- [D-20260715-017](../decisions/decisions.md)
- [W-20260715-021](../logs/work.md)
- [D-20260715-018](../decisions/decisions.md)
- [W-20260715-022](../logs/work.md)
- [D-20260715-019](../decisions/decisions.md)
- [W-20260715-023](../logs/work.md)
- [D-20260715-020](../decisions/decisions.md)
- [W-20260715-024](../logs/work.md)
- [D-20260715-021](../decisions/decisions.md)
- [W-20260715-025](../logs/work.md)
- [D-20260715-022](../decisions/decisions.md)
- [W-20260715-026](../logs/work.md)
