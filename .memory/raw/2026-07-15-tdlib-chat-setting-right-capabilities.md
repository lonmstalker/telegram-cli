# TDLib chat setting right capabilities

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static prerequisite subset of chat-setting methods in pinned TDLib `1.8.66`, commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`.

## Complete contracts

| Method | Supported chat kinds | Required right | Ready accounts |
|---|---|---|---|
| `setChatPermissions` | basic group, supergroup | administrator `can_restrict_members` | regular user, bot |
| `setChatDescription` | basic group, supergroup, channel | member `can_change_info` | regular user, bot |
| `setChatSlowModeDelay` | supergroup | administrator `can_restrict_members` | regular user |

Each row is bound to the exact canonical signature and normalized `@description` from `vendor/tdlib/td_api.tl`. `Requests.cpp` has `CHECK_IS_USER()` for `setChatSlowModeDelay`, but not for permissions or description. `DialogManager.cpp`/`ChatManager.cpp` independently enforce the listed kind and right checks.

## Deferred boundary

The exhaustive `setChat*` administrator/member-right family contains 3 new complete methods, prior-complete `setChatPaidMessageStarCount`, and these 12 deferred methods:

- `setChatAccentColor`
- `setChatAvailableReactions`
- `setChatBackground`
- `setChatDiscussionGroup`
- `setChatEmojiStatus`
- `setChatMemberStatus`
- `setChatMemberTag`
- `setChatMessageAutoDeleteTime`
- `setChatPinnedStories`
- `setChatProfileAccentColor`
- `setChatPhoto`
- `setChatTitle`

The first ten retain boost, input, value, target, or mixed-kind semantics not represented by the current closed grammar. `setChatTitle` and `setChatPhoto` are additionally account-conditioned: for a bot in a basic group, pinned `DialogManager.cpp` requires `is_appointed_chat_administrator()` even when effective permissions include `can_change_info`. They remain deferred instead of using a fail-open member-only DNF.

## TDD and review evidence

- Red: public `setChatTitle` fixture initially failed on untyped runtime signals.
- Initial implementation closed five methods; independent review found the bot/basic-group administrator guard for title/photo.
- Correction: title/photo were removed from complete contracts, added to the exhaustive deferred set, and the public fixture was moved to `setChatDescription`.
- Post-fix independent review: `APPROVED`, no remaining findings.
- Source/signature drift, wrong kind/right, missing branch, and extra argument signal fail closed.

## Corpus transition

- Supported typed methods: `63 -> 66`, SHA-256 `5fb3e7ba71f07968df7ca1cfdfca57cd4f1a9de2bf0db92b0491f009cd5a35a5`.
- Terminal dispositions: `66 -> 69`, SHA-256 `7d4e40331eb9eee73e899613e280a70f29dfa4dd3b418b9809bb5c836f6a1161`.
- Open methods: `127 -> 124`, SHA-256 `9286c8f2797606f47f5d136bdfdc0c80d7eb09ab650acaa6676520340880d04c`.
- Semantic disposition rows SHA-256: `3df0178c4e3c15b7d19a6189a456f874fdeed802aa118e563e619f8354f5e3e1`.
- Signal inventory remains 193 methods, 208 sources, and 398 keys.

Static requirements do not prove current runtime rights. A future evaluator must use current account/target-bound evidence and fail closed for missing or stale state.
