# TDLib unpinChatMessage capability overclaim correction

Date: 2026-07-15

## Scope

Sanitized correction evidence for the previously accepted static `unpinChatMessage` capability contract. The old description-only DNF was incomplete and is removed from pinned capability coverage. This digest does not implement the missing account/subtype/input grammar or runtime evaluator.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Reviewed source archive SHA-256: `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- `vendor/tdlib/td_api.tl` exact signature: `unpinChatMessage chat_id:int53 message_id:int53 = Ok;`.
- The public description mentions `can_pin_messages` for basic groups/supergroups and `can_edit_messages` for channels, but is not a complete handler contract.
- [`Requests.cpp` lines 6038-6042](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L6038-L6042): no `CHECK_IS_USER()`; regular users and bots reach `pin_dialog_message`.
- [`MessagesManager.cpp` lines 30348-30370](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/MessagesManager.cpp#L30348-L30370): non-business path loads the concrete message and delegates to `can_pin_message` before the update query.
- [`MessagesManager.cpp` lines 23211-23225](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/MessagesManager.cpp#L23211-L23225): `can_pin_message` delegates chat access to `can_pin_messages` and additionally rejects missing, scheduled, non-server and service messages.
- [`DialogManager.cpp` lines 2901-2935](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogManager.cpp#L2901-L2935): private chat is allowed; secret chat is rejected; bot in a basic group must also be an appointed administrator; monoforum bypasses the ordinary channel-permission branch; ordinary supergroup uses `can_pin_messages`, broadcast channel uses `can_edit_messages`; write peer access is required.

## Correction

- The previous five-branch DNF was false-positive for secret chats.
- The basic-group branch omitted the account-conditioned appointed-administrator guard for bots.
- The supergroup branch omitted the direct-messages/monoforum exception, where the ordinary right check is bypassed.
- Concrete message-state and write-peer prerequisites are also outside that description-only DNF.
- Current descriptor grammar cannot express the account-conditioned basic-group implication and complete input/state partition without speculative widening or excessive narrowing. The real method therefore returns to deferred `SchemaDrift`.
- The conditional-right generator fixture was renamed to `requireSyntheticConditionalPinRight`. It remains a synthetic test method absent from pinned `td_api.tl`; real-method oracles do not include it.

## TDD and exact oracles

- Red: a pinned regression expected deferred `SchemaDrift` and instead received the old five-branch `RequirementAlternatives`, including a secret-chat branch.
- Green/refactor: the real reviewed row and pinned positive assertion were removed; all generic DTO/policy tests now use the explicit synthetic fixture. The pinned recognizer regression requires `SchemaDrift` for `unpinChatMessage`.
- Supported typed set: 66, SHA-256 `38b1a2788216e889db327c13e26995db8f28f155862b6b993042689af536bb24`.
- Terminal set: 69, SHA-256 `75ecdb99e460dd373f11f4eafc7eb85b0d8d874d5e182c17a9f4f9dbaa3d6554`.
- Open set: 124, SHA-256 `ffd5fe2eed81664bc9e2d07d80582faf5a19531c553c36e92fd5096cfe759fb1`.
- Semantic disposition rows: SHA-256 `15f4aba28980b7e0b14b0a0e90636aeab7d5bdc4115a7c732b5403da07253fa2`.
- Verification before documentation: 60 generator + 24 core = 84 workspace tests; Clippy `-D warnings`, rustfmt, planning/workspace/schema/native/skeleton/process/diff gates green with `jobs=2`.
- Independent code review: `APPROVED`; actionable findings absent. Reviewer repeated 60 generator tests and `git diff --check`.

## Boundary

Deferred means “not proven by the current static model”, not “unsupported by Telegram”. Future completion needs an exact account-conditioned runtime grammar, closed direct-messages subtype evidence, message/input prerequisites and current write-access evidence. Missing or stale facts must fail closed.
