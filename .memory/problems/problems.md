# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

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

## [2026-07-15] open update | P-20260715-005 | Invite-link creation уменьшила open set до 131 methods

- Evidence: [chat invite-link creation digest](../raw/2026-07-15-tdlib-chat-invite-link-creation-capabilities.md); exact triple-signal family разделена на 2 complete create/replace methods и 9 mixed own/other-link methods.
- Transition: два methods получают three-kind `ChatAdministratorRight(CanInviteUsers)` DNF. Supported typed set теперь 59, terminal complete set 62, open-set SHA-256 `49480a48f3c072d8b3621c5d8e64ada2f1eacb13c697feed31279490e8886fbf`.
- Status: open; zero-open gate не достигнут, 131 method по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать следующие exact semantic families; runtime evaluator обязан fail closed на stale/unknown administrator-right evidence.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-019](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Supergroup setting rights уменьшили open set до 126 methods

- Evidence: [supergroup setting-right digest](../raw/2026-07-15-tdlib-supergroup-setting-right-capabilities.md); exact family разделена на 5 new complete, 1 prior complete и 3 boost/guard-input deferred methods.
- Transition: пять methods получают exact chat-kind and role-right requirements с account boundary. Supported typed set теперь 64, terminal complete set 67, open-set SHA-256 `71a75f389b248af4aeeb0e387e7be299d56d964f4969652f32bb3cfdcb47be9d`.
- Status: open; zero-open gate не достигнут, 126 methods по-прежнему дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать следующие exact semantic families; runtime evaluator обязан fail closed на stale/unknown right evidence.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-020](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-007 | Broad supergroup kind ошибочно закрывал ordinary-only setting

- Evidence: independent P2 review и [ordinary-supergroup correction digest](../raw/2026-07-15-tdlib-supergroup-setting-ordinary-kind-correction.md); pinned C++ отклоняет gigagroup/monoforum, которые broad kind не различает.
- Impact: `toggleSupergroupJoinToSendMessages` мог получить ложный supported verdict на неподдерживаемом target.
- Status: open at review discovery; exact refinement отсутствует.
- Next check: remove broad contract or add closed ordinary-supergroup evidence before commit.
- Related decisions: [D-20260715-020](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-007 | Ordinary-only method возвращён в deferred

- Evidence: contract row удалён, exhaustive test включает method в deferred, full generator oracle даёт supported 63/terminal 66/open 127.
- Resolution: broad claim удалён; runtime false positive невозможен через W024 contract.
- Status: resolved for current implementation. Отсутствующий ordinary-supergroup predicate остаётся честной причиной deferred state в [P-20260715-005](../problems/problems.md).
- Related decisions: corrected [D-20260715-020](../decisions/decisions.md).

## [2026-07-15] open correction | P-20260715-005 | Supergroup setting rights уменьшают open set до 127 methods

- Corrects: preceding 126-method transition, основанный на broad ordinary-supergroup claim.
- Evidence: [ordinary-supergroup correction digest](../raw/2026-07-15-tdlib-supergroup-setting-ordinary-kind-correction.md); current family 4 new complete, 1 prior complete, 4 deferred.
- Transition: supported typed set 63, terminal complete 66, open-set SHA-256 `b872e1f38e72845cd22f4a14460655508775545f5301882b8edbc6189265aa8d`.
- Status: open; zero-open gate не достигнут, 127 methods дают `SchemaDrift` и не считаются capability coverage.
- Related decisions: corrected [D-20260715-020](../decisions/decisions.md).

## [2026-07-15] open | P-20260715-008 | Member-only DNF теряла bot/basic-group administrator guard

- Evidence: independent review и pinned `DialogManager.cpp`: `setChatTitle`/`setChatPhoto` требуют `is_appointed_chat_administrator()` для bot в basic group сверх effective `can_change_info`.
- Impact: bot-member с разрешённым default `can_change_info` мог получить ложный supported verdict, хотя TDLib вернул бы `Not enough rights`.
- Status: open at review discovery; current grammar не выражает account-conditioned basic-group implication.
- Next check: удалить оба complete contracts или добавить closed account-sensitive requirement с runtime evidence до commit.
- Related decisions: [D-20260715-021](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-008 | Account-conditioned title/photo methods возвращены в deferred

- Evidence: [chat setting-right digest](../raw/2026-07-15-tdlib-chat-setting-right-capabilities.md), оба rows отсутствуют в `chat_settings::CONTRACTS`, exhaustive test включает их в deferred; post-fix reviewer verdict `APPROVED`.
- Resolution: member-only false positive удалён; safe set сокращён до permissions/description/slow mode, oracles дают supported 66, terminal 69, open 124.
- Status: resolved for current implementation. Будущий complete contract требует account/kind-conditioned prerequisite и current appointed-admin evidence.
- Related decisions: [D-20260715-021](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Chat setting rights уменьшили open set до 124 methods

- Evidence: [chat setting-right digest](../raw/2026-07-15-tdlib-chat-setting-right-capabilities.md); exact family разделена на 3 new complete, 1 prior complete и 12 deferred methods.
- Transition: permissions, description и slow mode получают exact kind/right/account contracts. Supported typed set 66, terminal complete 69, open-set SHA-256 `9286c8f2797606f47f5d136bdfdc0c80d7eb09ab650acaa6676520340880d04c`.
- Status: open; zero-open gate не достигнут, 124 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: отдельными reviewed tasks закрывать следующие exact semantic families; runtime evaluator обязан fail closed на stale/unknown/account-mismatched right evidence.
- Archive link map после ротации: [P-20260715-004 resolved и correction](archive/2026-07-15--2026-07-15-004.md).
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-021](../decisions/decisions.md).
