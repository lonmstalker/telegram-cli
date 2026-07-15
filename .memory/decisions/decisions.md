# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

## [2026-07-15] accepted | D-20260715-010 | Real-schema capability grammar закрывается по exact open set

- Context: foundation принимал пять real runtime contracts, но pinned documentation содержит 193 methods с capability-like signals. Попытка сразу заполнить 1010-row policy либо остановилась бы на 188 `SchemaDrift`, либо потребовала бы ослабить fail-closed recognizer.
- Decision: закрепить exact 193-method signal set и exact 188-method open set до расширения grammar. Каждый следующий source-family task обязан уменьшать open set через closed typed model и independent review. Deferred row не считается capability coverage; canonical 1010-method artifact разрешён только после zero-open gate.
- Evidence: [capability evidence baseline](../raw/2026-07-15-tdlib-capability-evidence-baseline.md), corpus hashes в `tools/tdlib-registry-gen/src/capability/tests.rs`, red-green authorization test для `setCustomLanguagePack.@info`.
- Alternatives: немедленный handwritten 1010-row artifact отклонён из-за unsupported contracts; free-form expression strings отклонены как непроверяемая semantic surface; один monolithic grammar/corpus commit отклонён как слишком широкий review unit.
- Consequences: capability grammar развивается малыми reviewed commits по source family. Следующий oracle хранит disposition каждого exact signal, а method выходит из open set только после consumption всех своих signals. Runtime capability, input prerequisite, retry и lexical false positive получают раздельные lanes; до этого [P-20260715-005](../problems/problems.md) остаётся open.
- Supersedes: none; extends [D-20260715-009](decisions.md).

## [2026-07-15] accepted | D-20260715-011 | Chat kind является typed runtime evidence, а не free-form predicate

- Context: conditional TDLib rights зависят от resolved chat kind; один `chat_id` может обозначать private, basic group, supergroup, channel или secret chat, а `supergroup_id` — supergroup либо channel. Одних right atoms недостаточно для exact `unpinChatMessage` и других reviewed contracts.
- Decision: добавить closed `ResolvedChatKind` и schema-bound `ChatKindCondition` как самостоятельный DNF atom. `Channel` моделируется как semantic refinement `chatTypeSupergroup.is_channel=true`, не как выдуманный constructor. Несовместимый identifier space и две разные kinds одного target в AND-clause fail closed.
- Evidence: [ChatKind capability digest](../raw/2026-07-15-tdlib-chat-kind-capability.md), exact `ChatType` signature gate, six-method pinned DNF oracle и corpus open-set hashes в `tools/tdlib-registry-gen/src/capability/tests.rs`.
- Alternatives: свободные expression strings и implication DSL отклонены как лишняя непроверяемая поверхность; broad `is_group` отклонён, потому что он теряет channel-specific rights; считать private/secret unpin unconditional без kind evidence отклонено как ложное расширение branch.
- Consequences: capability policy format становится `2`; vendor/owner formats не меняются. Exact supported set растёт с 5 до 6, open set уменьшается с 188 до 187. Остальные source families добавляются отдельными reviewed tasks и не наследуют generic fallback.
- Supersedes: none; extends [D-20260715-009](decisions.md) и [D-20260715-010](decisions.md).

## [2026-07-15] accepted | D-20260715-012 | Capability completion требует terminal disposition каждого signal key

- Context: method-level recognizer не сохранял source/family multiplicity и мог скрыть дополнительный deferred signal после реализации одного reviewed contract.
- Decision: canonical scan строит exact `(method, source, family)` keys; каждый key получает terminal consumed/non-gate либо deferred lane. Reviewed contract явно объявляет consumed keys, и method покидает open set только при exact equality и отсутствии deferred keys. Неизвестный description остаётся `UnclassifiedDescription`, а lexical non-gate требует exact normalized wording.
- Evidence: [per-signal disposition digest](../raw/2026-07-15-tdlib-runtime-signal-dispositions.md), 208-source/398-key и semantic hashes, partial-consumption/semantic-drift negative controls, два independent `Approved` reviews.
- Alternatives: auto-consume всех description families отклонено как fail-open; family-wide lexical exceptions отклонены из-за semantic drift; сразу вводить отдельный policy artifact отклонено до canonical 1010-method policy task.
- Consequences: in-code oracle не меняет capability format `2`; два exact `ChatBoost` lexical non-gates terminally dispositioned, open set уменьшается 187 → 185. Следующие typed families изменяют только свои explicit keys и exact open-set oracle.
- Supersedes: none; extends [D-20260715-010](decisions.md) и [D-20260715-011](decisions.md).

## [2026-07-15] archive pointer | D-20260715-002 | Публичный GitHub remote

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-002.md). Pointer only; решение не изменено.

## [2026-07-15] archive pointer | D-20260715-003 | Exact initial production schema pin

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-002.md). Pointer only; решение не изменено.

## [2026-07-15] accepted | D-20260715-013 | Message property является exact quantified evidence

- Context: coarse `MessagePropertiesFact`/`CanFieldReference` keys не различают конкретное поле, cardinality и invocation-dependent semantics; method-name-only consumption мог скрыть подмену field или исчезновение source.
- Decision: pin ordered 39-field `messageProperties` constructor и closed 36-field action vocabulary. Runtime atom хранит exact `One(chat_id,message_id)` либо `Each(supergroup_id,message_ids)` subject. Reviewed method lookup сначала фиксирует имя, затем требует единственный exact normalized source и schema shape; любой drift даёт `SchemaDrift`. `addOffer` — OR двух target-state properties; `reportSupergroupSpam` — conjunction supergroup/admin/all messages. `deleteMessages`, `forwardMessages`, `getMessageLink` и `reportChat` остаются deferred.
- Evidence: [MessageProperties capability digest](../raw/2026-07-15-tdlib-message-properties-capabilities.md), exhaustive 33-method partition, 30-binding/59-consumed/11-deferred hashes, pinned C++ branch evidence, public generator E2E и три independent `Approved` reviews.
- Alternatives: free-form property strings, method-name-only auto-consumption, global `all(message_ids)` и permissive fallback отклонены как fail-open; mixed methods не получили ложный common predicate.
- Consequences: capability format становится `3`; supported typed set растёт 6 → 35, terminal complete 8 → 37, exact open set уменьшается 185 → 156. Static requirement не является runtime truth или policy permission; zero-open gate и canonical 1010-method artifact остаются обязательными.
- Supersedes: none; extends [D-20260715-009](decisions.md), [D-20260715-010](decisions.md), [D-20260715-011](decisions.md) и [D-20260715-012](decisions.md).

## [2026-07-15] archive pointer | D-20260715-004 | Native artifact provenance без binary в Git

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-003.md). Pointer only; решение не изменено.

## [2026-07-15] archive pointer | D-20260715-005 | Crash-safe ownership native scratch и child processes

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-004.md). Pointer only; решение не изменено.

## [2026-07-15] archive pointer | D-20260715-001 | Memory model, secret boundary и план P0–P10

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-001.md). Pointer only; решение не изменено.

## [2026-07-15] accepted | D-20260715-014 | Group-call capability требует kind, exact property и freshness-aware evidence

- Context: coarse `GroupCallFact`/`CanFieldReference` keys не различают video chat, live story и unbound call, конкретное Bool-поле, universal message cardinality и текст, который описывает значение настройки, а не право вызывающего.
- Decision: pin ordered `groupCall`/`groupCallMessage` shapes и их getter/update ingress; хранить closed `ResolvedGroupCallKind`, exact `GroupCallProperty`, semantic `group_call_id:int32` и `Each(message_ids:vector<int32>)`. Двенадцать complete methods получают exact name-first source contracts и typed DNF; `getVideoChatInviteLink` и `toggleGroupCallParticipantIsHandRaised` остаются deferred. Фраза `only by administrators` для mute-new-participants — exact non-gate semantics настройки, не consumed prerequisite. Все complete contracts regular-user-only по pinned `CHECK_IS_USER()` evidence.
- Freshness rule: future evaluator не вправе считать static atom runtime truth. `is_active`/`is_joined`/`need_rejoin` и invocation inputs принадлежат prerequisite/runtime слоям. Cached `GroupCallMessage.can_be_deleted` после релевантного `updateGroupCall`, eviction или неполного ingress должен считаться stale/unknown и fail closed; отсутствие arbitrary getter допускает безопасный false-negative.
- Evidence: [GroupCall capability digest](../raw/2026-07-15-tdlib-group-call-capabilities.md), exact 14-method partition, 38 consumed/6 deferred signal hashes, pinned source hashes, public generator tests и три independent `Approved` reviews.
- Alternatives: free-form kind/property strings, `otherwise` без explicit kind branches, implicit all-message cardinality, auto-consumption всех exact-description families и трактовка `only by administrators` как caller right отклонены как fail-open или семантически неверные.
- Consequences: capability format становится `4`; supported typed set растёт 35 → 47, terminal complete 38 → 50, exact open set уменьшается 155 → 143. Runtime evaluator, freshness store, prerequisite/risk/retry и zero-open 1010-method artifact остаются отдельными обязательными слоями.
- Supersedes: none; extends [D-20260715-009](decisions.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md) и [D-20260715-013](decisions.md).

## [2026-07-15] archive pointer | D-20260715-006 | Strict TDLib schema parser отделён от policy classification

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-005.md). Pointer only; решение не изменено.

## [2026-07-15] accepted correction | D-20260715-014 | Message evidence invalidates on group-call, chat and account changes

- Corrects: emphasis on `updateGroupCall` in the original freshness rule was too narrow for future runtime implementation.
- Exact rule: pinned `can_delete_group_call_message` also depends on sender=self, current channel-administrator status and `created_public_broadcasts`, besides group-call active/live/delete facts. Cached `GroupCallMessage.can_be_deleted` must become unknown after any capability-affecting group-call, chat or account change, eviction or incomplete ingress; if dependency-complete invalidation is unavailable, use conservative age-out. Unknown/stale always fails closed.
- Evidence: `GroupCallManager.cpp:3503-3528` from the pinned source archive and the corrected [GroupCall capability digest](../raw/2026-07-15-tdlib-group-call-capabilities.md).
- Status: `D-20260715-014` remains accepted with this broader future-runtime boundary; W018 static DNF and corpus counts are unchanged.

## [2026-07-15] archive pointer | D-20260715-007 | Feature ownership генерируется отдельным fail-closed tool

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-006.md). Pointer only; решение не изменено.

## [2026-07-15] accepted | D-20260715-015 | Supergroup full-info property является static typed evidence с отдельной freshness boundary

- Context: coarse `SupergroupFullInfoFact`/`CanFieldReference` keys не различают конкретное Bool-поле, semantic target, conjunction с administrator right и cross-token lexical false positives. При этом full-info snapshot имеет собственный lifecycle и не может считаться runtime truth только из static policy.
- Decision: pin ordered 42-field `supergroupFullInfo`, exact getter/update ingress и closed eight-property vocabulary. Пять complete methods получают name-first exact source contracts и typed DNF; семь mixed owner/filter/value/password methods остаются deferred. `setChatPaidMessageStarCount` требует conjunction `can_restrict_members AND can_enable_paid_messages`. Два cross-token `OnlyIfAdministrator` keys являются exact-source non-gates; реальная filter-dependent administrator оговорка у `getSupergroupMembers` остаётся deferred. Все complete contracts regular-user-only по pinned `CHECK_IS_USER()` evidence, но property atom account-neutral.
- Freshness rule: W019 не реализует runtime evaluator. Будущий full-info store обязан разрешать `chat_id` в соответствующий supergroup/channel, различать fresh/stale/missing snapshot, учитывать 60-second TDLib expiry и capability-affecting chat/account changes. Missing, evicted или stale evidence всегда false; при неполной dependency invalidation нужен conservative age-out.
- Evidence: [SupergroupFullInfo capability digest](../raw/2026-07-15-tdlib-supergroup-full-info-capabilities.md), exact 12-method partition, 12 consumed/18 deferred signal hashes, pinned source hashes, public generator tests и три independent approved reviews.
- Alternatives: generic extraction любого `supergroupFullInfo.can_*`, common predicate для всех 12 methods, consumption реальной filter-dependent administrator оговорки, bot narrowing на уровне atom и static claim о freshness отклонены как семантически неверные или fail-open.
- Consequences: capability format становится `5`; supported typed set растёт 47 → 52, terminal complete 50 → 55, exact open set уменьшается 143 → 138. Runtime evaluator, freshness store, prerequisite/risk/retry и zero-open 1010-method artifact остаются отдельными обязательными слоями.
- Supersedes: none; extends [D-20260715-009](decisions.md), [D-20260715-010](decisions.md), [D-20260715-012](decisions.md) и [D-20260715-014](decisions.md).

## [2026-07-15] archive pointer | D-20260715-008 | Exact owner corpus закрепляет domain ownership, а не runtime parity

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-007.md). Pointer only; решение не изменено.

## [2026-07-15] archive pointer | D-20260715-009 | Static capability requirements отделены от runtime truth и policy permission

- Canonical entry: [immutable decision shard](archive/2026-07-15--2026-07-15-008.md). Pointer only; решение не изменено.
