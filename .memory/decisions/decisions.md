# Decision Journal

Active append-only decision records. Изменение решения оформляется новой entry; старое решение не переписывается.

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
