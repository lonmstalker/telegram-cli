# TDLib recent-reaction sender moderation capability

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `deleteAllRecentMessageReactionsFromSender` capability contract in pinned TDLib. It proves accepted account/kind/right prerequisites, not current write access, sender availability, rights freshness or server success.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `vendor/tdlib/td_api.tl` exact signature: `deleteAllRecentMessageReactionsFromSender chat_id:int53 sender_id:MessageSender = Ok;`.
- Exact description supports only basic groups and supergroups and requires `can_delete_messages` administrator right.
- [`Requests.cpp` lines 3959-3963](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L3959-L3963): dispatcher resolves the sender and delegates without an account-kind guard.
- [`MessageQueryManager.cpp` lines 3951-3961](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/MessageQueryManager.cpp#L3951-L3961): deeper path checks target write access and sender input-peer availability before the RPC; it adds no account, subtype or role gate.
- The pinned query path adds no further local account/subtype prerequisite. Telegram server acceptance remains outside the static descriptor.

## Model

- Existing `capability/message_moderation.rs` gains a second exact row; no new abstraction or parallel module is introduced.
- Account scope is `[RegularUser, Bot]`. A regular-user-only policy is hidden narrowing and is rejected.
- Static DNF has exactly two alternatives for `chat_id`: `basic_group AND ChatAdministratorRight(can_delete_messages)` or `supergroup AND ChatAdministratorRight(can_delete_messages)`.
- `required_supergroup_flags` is empty. No direct-messages, broadcast-group, gigagroup or other subtype atom is invented without pinned handler evidence.
- Source/signature drift, additional argument-level signal and incomplete account policy fail closed. Executable code contains no planning/task IDs.

## TDD and exact oracles

- Red: the pinned test first returned deferred `SchemaDrift` because the method had no typed runtime disposition.
- Green/refactor: one exact domain row reuses the message-moderation contract and makes focused/corpus tests green.
- Supported typed set: 71, SHA-256 `f592b6e3b87a9fa978247d9c44a1088a777030ba40c65bd8468409eb1da45f85`.
- Terminal set: 74, SHA-256 `88e23c78cc5f54ceae5e5ec920b93055737454bed41b2d5bed79c7beee38242b`.
- Open set: 119, SHA-256 `27dd1e3d3014e2d30880e69b8adf865969c4ec9a536fbf46c167835c5b1c6ca2`.
- Semantic disposition rows: SHA-256 `44c17d11ba3079bfed67b3b8d91e73dc728368a5479354eeff508b6ff2ccd4b1`.
- Verification: 66 generator + 24 core = 90 workspace tests green with `jobs=2`.
- Independent source/Rust review: `APPROVED`; actionable findings absent.

## Boundary

Write access, sender input-peer availability and `can_delete_messages` are current runtime facts, not proved by the static descriptor. The future singleton daemon must bind evidence to the current account/target/session generation and fail closed on missing, stale or gap-affected state. A static pass does not promise successful deletion by Telegram.
