# Telegram CLI Wiki

Начинай долговечную работу с этой страницы и открывай только нужные ссылки.

## Canonical project sources

- [Product boundary](../../product.md)
- [Living plan](../../plans.md)
- [Feature inventory](../../HARNESS.md)
- [TDLib coverage contract](../../docs/tdlib-api-coverage.md)
- [Current project state](project-state.md)

## Memory streams

- [Active work journal](../logs/work.md)
- [Active decision journal](../decisions/decisions.md)
- [Active problem journal](../problems/problems.md)
- [Work archive](../logs/archive/index.md)
- [Decision archive](../decisions/archive/index.md)
- [Problem archive](../problems/archive/index.md)
- [Bootstrap source digest](../raw/2026-07-15-project-bootstrap.md)
- [TDLib 1.8.66 schema pin digest](../raw/2026-07-15-tdlib-1.8.66-schema-pin.md)
- [TDLib 1.8.66 macOS arm64 first-build digest](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64.md) — historical pre-review evidence.
- [TDLib 1.8.66 macOS arm64 reviewed rebuild correction](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — current artifact/resource truth.
- [TDLib strict schema parser/inventory digest](../raw/2026-07-15-tdlib-schema-parser-inventory.md) — reviewed P0.3 parser facts and boundaries.
- [TDLib feature-owner generator digest](../raw/2026-07-15-tdlib-feature-owner-generator.md) — historical evidence удалённой planning-taxonomy implementation.
- [TDLib feature-owner corpus digest](../raw/2026-07-15-tdlib-feature-owner-corpus.md) — historical evidence удалённого owner corpus.
- [Planning-taxonomy removal correction](../raw/2026-07-15-planning-taxonomy-removal.md) — current boundary: planning IDs только в документации, runtime contracts keyed by schema names/signatures.
- [TDLib capability generator foundation digest](../raw/2026-07-15-tdlib-capability-generator-foundation.md) — closed bounded static model и fail-closed generator до полного 1010-method corpus.
- [TDLib capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md) — exact 193-method signal set, 188-method open set и all-tag authorization correction.
- [TDLib ChatKind capability semantics](../raw/2026-07-15-tdlib-chat-kind-capability.md) — exact `ChatType` pin, six reviewed conditional contracts и 187-method open set.
- [TDLib per-signal runtime disposition oracle](../raw/2026-07-15-tdlib-runtime-signal-dispositions.md) — exact 208 sources/398 keys, partial-consumption invariant и 185-method open set.
- [TDLib MessageProperties capability semantics](../raw/2026-07-15-tdlib-message-properties-capabilities.md) — exact 39-field vocabulary, 29 typed contracts, four deferred mixed methods и 156-method open set.
- [TDLib getChatBoostLinkInfo lexical non-gate](../raw/2026-07-15-tdlib-chat-boost-link-non-gate.md) — exact type-name vocabulary exception и 155-method open set.
- [TDLib GroupCall capability semantics](../raw/2026-07-15-tdlib-group-call-capabilities.md) — exact kind/property/cardinality DNF, setting-value non-gate и 143-method open set.
- [TDLib SupergroupFullInfo capability semantics](../raw/2026-07-15-tdlib-supergroup-full-info-capabilities.md) — exact property/target DNF, cross-token lexical non-gates и 138-method open set.
- [TDLib runtime boolean option capability semantics](../raw/2026-07-15-tdlib-runtime-boolean-options.md) — exact option vocabulary, one method-level gate, two deferred mixed methods и 137-method open set.
- [TDLib supergroup username owner capability semantics](../raw/2026-07-15-tdlib-supergroup-username-owner-capabilities.md) — four exact `ChatKind AND ChatOwner` contracts, exhaustive owner-signal partition и 133-method open set.
- [TDLib chat invite-link creation capability semantics](../raw/2026-07-15-tdlib-chat-invite-link-creation-capabilities.md) — two exact three-kind administrator-right contracts, nine mixed methods deferred и 131-method open set.
- [TDLib supergroup setting-right initial digest](../raw/2026-07-15-tdlib-supergroup-setting-right-capabilities.md) — historical pre-review evidence; current counts superseded.
- [TDLib supergroup setting ordinary-kind correction](../raw/2026-07-15-tdlib-supergroup-setting-ordinary-kind-correction.md) — four exact contracts, ordinary-only method deferred и 127-method open set.
- [TDLib chat setting right capabilities](../raw/2026-07-15-tdlib-chat-setting-right-capabilities.md) — three exact contracts, account-conditioned title/photo deferred и 124-method open set.
- [TDLib addChatMember overclaim correction](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md) — hidden regular-user/direct-messages-group gates возвращают incomplete contract в deferred и 125-method open set.
- [TDLib supergroup subtype capability semantics](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md) — closed Boolean flags закрывают ordinary-setting contract, сохраняя оба self/cardinality-dependent invite flows deferred; open set 124.
- [TDLib chat event log capability semantics](../raw/2026-07-15-tdlib-chat-event-log-capability.md) — exact regular-user, supergroup/channel и administrator contract; open set 123.
- [TDLib unpinChatMessage overclaim correction](../raw/2026-07-15-tdlib-unpin-chat-message-overclaim-correction.md) — hidden account/subtype/message branches возвращают incomplete real-method DNF в deferred; open set 124.
- [TDLib chat invite-link counts capability](../raw/2026-07-15-tdlib-chat-invite-link-counts-capability.md) — exact regular-user, three-kind owner contract в existing invite-link module; open set 123.
- [TDLib video chat RTMP access capability](../raw/2026-07-15-tdlib-video-chat-rtmp-access-capability.md) — exact regular-user, three-kind `can_manage_video_chats` contract; dialog read access остаётся runtime boundary, open set 122.
- [TDLib video chat RTMP replacement capability](../raw/2026-07-15-tdlib-video-chat-rtmp-replacement-capability.md) — exact regular-user, three-kind owner revoke contract; shared admin precheck не ослабляет public owner boundary, open set 121.
- [TDLib video chat creation capability](../raw/2026-07-15-tdlib-video-chat-creation-capability.md) — exact regular-user, three-kind `can_manage_video_chats` contract; request values остаются server semantics, open set 120.

## Current records

- Implementation: [P0 in progress](project-state.md) — workspace, exact schema, strict parser/inventory, capability foundation/ChatKind/per-signal/MessageProperties/GroupCall/SupergroupFullInfo/runtime-option/username-owner/invite-link/chat-setting/supergroup-subtype/chat-event-log/video-chat semantics, planning-taxonomy/unpin corrections и macOS native pin закрыты через `W-20260715-033`; 120 typed dispositions, 1010-method capability corpus, risk/retry, full registry и runtime ещё не реализованы.
- Native pin: [reviewed rebuild correction](../raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md) — exact source/schema и crash-safe macOS arm64 artifact закреплены; Linux/reproducibility остаются open.
- Decision: [D-20260715-001](../decisions/archive/2026-07-15--2026-07-15-001.md) — раздельная memory model, rotation и secret boundary.
- Decision: [D-20260715-002](../decisions/archive/2026-07-15--2026-07-15-002.md) — публичный GitHub remote принят как canonical `origin`.
- Decision: [D-20260715-003](../decisions/archive/2026-07-15--2026-07-15-002.md) — initial production schema pin использует exact TDLib commit, не moving branch.
- Decision: [D-20260715-004](../decisions/decisions.md) — binary остаётся в content-addressed local cache, Git хранит exact policy/recipe/provenance.
- Decision: [D-20260715-005](../decisions/decisions.md) — inherited global lease, gated target и proof-backed recovery определяют crash ownership.
- Decision: [D-20260715-006](../decisions/decisions.md) — schema parser остаётся pure strict TDLib subset в `telegram-core`, а policy classification отделена от AST.
- Superseded decision: [D-20260715-007](../decisions/archive/2026-07-15--2026-07-15-006.md) — historical owner-classification design; superseded by `D-20260715-017`.
- Superseded decision: [D-20260715-008](../decisions/archive/2026-07-15--2026-07-15-007.md) — historical exact owner mapping; superseded by `D-20260715-017`.
- Decision: [D-20260715-009](../decisions/archive/2026-07-15--2026-07-15-008.md) — static capability requirements имеют closed bounded model; распознанные unsupported gate signals и лишнее policy-сужение fail closed, runtime truth остаётся отдельным слоем.
- Decision: [D-20260715-010](../decisions/archive/2026-07-15--2026-07-15-009.md) — capability grammar закрывается малыми reviewed source-family tasks по exact open set; full artifact требует zero-open gate.
- Decision: [D-20260715-011](../decisions/archive/2026-07-15--2026-07-15-010.md) — chat kind является closed typed evidence; channel — refinement `chatTypeSupergroup.is_channel`, не отдельный constructor.
- Decision: [D-20260715-012](../decisions/decisions.md) — method complete только при terminal disposition каждого exact signal key.
- Decision: [D-20260715-013](../decisions/decisions.md) — message-property capability требует exact source, identifier space и scalar/universal cardinality; mixed invocation semantics остаются deferred.
- Decision: [D-20260715-014](../decisions/decisions.md) — group-call capability требует explicit kind/property/cardinality; setting semantics и stale runtime evidence не считаются caller capability.
- Decision: [D-20260715-015](../decisions/archive/2026-07-15--2026-07-15-016.md) — supergroup full-info property является static typed evidence; stale/missing snapshot остаётся отдельной fail-closed runtime boundary.
- Decision: [D-20260715-016](../decisions/archive/2026-07-15--2026-07-15-018.md) — runtime boolean option является generation-bound typed evidence, а не Premium entitlement.
- Decision: [D-20260715-017](../decisions/archive/2026-07-15--2026-07-15-019.md) — numeric feature inventory остаётся только документацией; executable architecture keyed by semantic modules и exact TDLib schema identity.
- Decision: [D-20260715-018](../decisions/archive/2026-07-15--2026-07-15-020.md) — username-management prerequisite требует explicit supergroup/channel kind и current owner evidence.
- Decision: [D-20260715-019](../decisions/archive/2026-07-15--2026-07-15-021.md) — invite-link creation требует explicit chat kind и current administrator `can_invite_users` evidence.
- Decision correction: [D-20260715-020](../decisions/archive/2026-07-15--2026-07-15-023.md) — current four-method supergroup-setting boundary; historical base сохранён в shard 022.
- Decision: [D-20260715-021](../decisions/archive/2026-07-15--2026-07-15-023.md) — chat settings закрываются только complete kind/right/account contracts; account-conditioned title/photo остаются deferred.
- Decision: [D-20260715-022](../decisions/decisions.md) — membership contract остаётся deferred, если pinned handler добавляет account или supergroup-subtype gate, отсутствующий в static DNF.
- Decision: [D-20260715-023](../decisions/decisions.md) — supergroup subtype выражается closed schema-bound Boolean fact; static prerequisite требует отдельной generation-bound runtime freshness.
- Decision: [D-20260715-024](../decisions/decisions.md) — chat event log требует explicit regular-user, supergroup/channel и current administrator evidence.
- Decision: [D-20260715-025](../decisions/decisions.md) — `unpinChatMessage` остаётся deferred, пока grammar не выражает account/subtype/message branches deeper handler.
- Decision: [D-20260715-026](../decisions/decisions.md) — invite-link `RequiredAccess` связывает owner/admin semantics с account scope, DNF и exact consumed keys.
- Decision: [D-20260715-027](../decisions/decisions.md) — RTMP access pin-ит regular-user, chat-kind и `can_manage_video_chats` prerequisites без invented call-state gates.
- Decision: [D-20260715-028](../decisions/decisions.md) — RTMP revoke использует stricter owner contract поверх shared local administrator precheck.
- Decision: [D-20260715-029](../decisions/decisions.md) — video-chat creation использует existing administrator-right vocabulary; request values не расширяют capability DSL.
- Open problem: [P-20260715-001](../problems/problems.md) — database key ещё не подключён к штатному gateway.
- Open problem: [P-20260715-003](../problems/problems.md) — Linux x86_64 native artifact ещё не закреплён.
- Open problem: [P-20260715-005](../problems/problems.md) — 120 pinned runtime-signal methods ещё не имеют typed disposition.

## Operating rules

- Raw digests и archive shards immutable.
- Wiki pages являются компактным synthesis и обновляются при изменении verified state.
- Work, decisions и problems никогда не смешиваются в одном журнале.
- `.env.local` используется только через protected loader; значения не читаются и не записываются в memory.
