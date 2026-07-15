# Problem Journal

Active append-only problem lifecycle. Status changes добавляются новой entry с тем же `P-*` ID.

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

## [2026-07-15] open | P-20260715-009 | addChatMember contract терял account и direct-messages-group gates

- Evidence: [addChatMember correction digest](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md); pinned `Requests.cpp` выполняет `CHECK_IS_USER`, а channel participant path отклоняет `is_monoforum`, отражённый как `supergroup.is_direct_messages_group`.
- Impact: bot account или direct-messages supergroup могли получить ложный supported verdict по broad kind/member-right DNF.
- Status: open at discovery; current model не содержит required subtype condition.
- Next check: удалить incomplete contract до commit либо добавить exact closed subtype evidence и account constraint.
- Related decisions: [D-20260715-022](../decisions/decisions.md).

## [2026-07-15] resolved | P-20260715-009 | addChatMember возвращён в deferred

- Evidence: `MemberRightInKinds` и единственный contract удалены; exact regression требует `SchemaDrift`, signal dispositions deferred, independent reviewer дал `APPROVED`.
- Resolution: false-positive path закрыт без speculative subtype abstraction.
- Status: resolved for current implementation; future complete contract требует regular-user и direct-messages-group evidence.
- Related decisions: [D-20260715-022](../decisions/decisions.md).

## [2026-07-15] open correction | P-20260715-005 | addChatMember correction увеличила open set до 125 methods

- Corrects: preceding 124-method current state, где incomplete `addChatMember` считался complete.
- Evidence: [addChatMember correction digest](../raw/2026-07-15-tdlib-add-chat-member-overclaim-correction.md); supported 65, terminal 68, open-set SHA-256 `ff2f1639bd2947b460ebac2d7a733e71556619db8804ebe49f7410e73cd13af6`.
- Status: open; zero-open gate не достигнут, 125 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: закрывать exact source families только после dispatcher/deeper-handler evidence; subtype/account-sensitive methods fail closed.
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/decisions.md), [D-20260715-022](../decisions/decisions.md).

## [2026-07-15] resolved update | P-20260715-009 | Reviewer сохраняет singular membership contract deferred

- Evidence: [supergroup subtype capability digest](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md); independent reviewer подтвердил, что self-join обходит `can_invite_users`, а non-self path требует right.
- Resolution: false-positive correction остаётся в силе; ни singular, ни size-one plural path не возвращены в supported set без typed self/non-self/cardinality invocation partition.
- Status: resolved как устранённый overclaim; missing singular capability остаётся частью [P-20260715-005](../problems/problems.md), а не новым ложным contract.
- Related decision: [D-20260715-023](../decisions/decisions.md).

## [2026-07-15] open update | P-20260715-005 | Ordinary-supergroup subtype contract уменьшил open set до 124 methods

- Evidence: [supergroup subtype capability digest](../raw/2026-07-15-tdlib-supergroup-flag-capabilities.md); exact supported set 66, terminal set 69, open-set SHA-256 `437c17ed2ccb09f23aa7eba6b04223e0b05a97ae55493d280fa18f28fe7ce796`.
- Transition: один method получает exact account/kind/subtype/right contract; оба self/cardinality-dependent invite methods остаются deferred; capability format становится `8`.
- Status: open; zero-open gate не достигнут, 124 methods дают `SchemaDrift` и не считаются capability coverage.
- Next check: продолжать reviewed source-family tasks; runtime subtype evidence обязано быть current-session, target/account/DC-bound и fail closed на gap/staleness.
- Archive link map после ротации: [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md).
- Related decisions: [D-20260715-010](../decisions/decisions.md), [D-20260715-012](../decisions/archive/2026-07-15--2026-07-15-011.md), [D-20260715-023](../decisions/decisions.md).
