# TDLib channel gift-notification capability

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `toggleChatGiftNotifications` capability contract in pinned TDLib. It proves the accepted account, chat-kind and administrator-right prerequisites, not current read access, channel/status freshness or Telegram server success.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- [`td_api.tl` lines 15151-15154](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/generate/scheme/td_api.tl#L15151-L15154) defines `toggleChatGiftNotifications chat_id:int53 are_enabled:Bool = Ok;` for a channel chat and requires `can_post_messages`.
- [`Requests.cpp` lines 8435-8439](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L8435-L8439) applies `CHECK_IS_USER()` before delegating.
- [`StarGiftManager.cpp` lines 2919-2927](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/StarGiftManager.cpp#L2919-L2927) requires read access, a broadcast channel and `can_post_messages`.
- [`DialogManager.cpp` lines 1973-1978](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogManager.cpp#L1973-L1978) and [`ChatManager.cpp` lines 8585-8587](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/ChatManager.cpp#L8585-L8587) map the internal broadcast-channel branch to public `ChatType::Channel`.
- [`StarGiftManager.cpp` lines 598-627](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/StarGiftManager.cpp#L598-L627) only resolves the input peer and sends `are_enabled`; it adds no value-dependent capability gate.

## Model

- Existing `capability/chat_settings.rs` gains one exact row; no parallel module or new abstraction is introduced.
- Account scope is `RegularUser`. A bot-enabled descriptor is rejected by the pinned dispatcher contract.
- Static DNF has exactly one alternative for `chat_id`: `channel AND ChatAdministratorRight(can_post_messages)`.
- `are_enabled` is a request value, not a capability atom. `required_supergroup_flags` stays empty and `target_source_text` stays absent.
- Source/signature drift and an additional argument-level signal fail closed. Executable code contains no planning/task IDs.

## TDD and exact oracles

- Red: the pinned test initially returned deferred `SchemaDrift` because the method had no typed runtime disposition.
- Green/refactor: one row in the existing chat-settings domain model makes the focused and corpus tests green without adding infrastructure.
- Supported typed set: 72, SHA-256 `6075cb099bb5bd91e3e58108b85c855c423c8b1a725bf99a5579b4ae28f3bd7e`.
- Terminal set: 75, SHA-256 `b0313021036610a5f6d9412e2ea99361681f59b983d8a6a14d6a6e9598c5d69a`.
- Open set: 118, SHA-256 `090cf24de23ace4b7bc1a9b9115181afacdca75b23eff0d8506fc0efc5a6c29a`.
- Semantic disposition rows: SHA-256 `f6d0258163531e781ef5911161fe9d5f9cf2671010bb4167c71aa6c0e1742b7f`.
- Verification: 67 generator + 24 core = 91 workspace tests green with `jobs=2`.
- Independent source/Rust review: `APPROVED`; actionable findings absent.

## Boundary

Read access, current public chat type, administrator rights and Telegram server acceptance are runtime facts, not proved by the static descriptor. The future singleton daemon must bind evidence to the current account/target/session generation and fail closed on missing, stale or gap-affected state. A static pass does not promise successful mutation by Telegram.
