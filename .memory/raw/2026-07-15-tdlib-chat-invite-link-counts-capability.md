# TDLib chat invite link counts capability contract

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `getChatInviteLinkCounts` capability contract in pinned TDLib. It proves request prerequisites, not current ownership, runtime freshness or live request success.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Reviewed source archive SHA-256: `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- `vendor/tdlib/td_api.tl` exact signature: `getChatInviteLinkCounts chat_id:int53 = ChatInviteLinkCounts;`.
- Exact normalized description requires owner privileges in the chat.
- [`Requests.cpp` lines 6226-6230](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L6226-L6230): dispatcher executes `CHECK_IS_USER()` and calls `get_dialog_invite_link_counts`.
- [`DialogInviteLinkManager.cpp` lines 1041-1046](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogInviteLinkManager.cpp#L1041-L1046): handler calls `can_manage_dialog_invite_links(dialog_id, true)` before the query.
- [`DialogInviteLinkManager.cpp` lines 918-952](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogInviteLinkManager.cpp#L918-L952): write access is required; private and secret chats are rejected; basic group must be active; `creator_only=true` requires creator/owner status for basic groups and channel-dialog targets, covering Telegram supergroups and channels.

## Model

- Existing `capability/chat_invite_links.rs` is extended instead of adding another module.
- Closed `RequiredAccess` distinguishes `AdministratorRight(right)` from `Owner`. It is the single source for DNF construction, method account scope and exact consumed signal keys; no duplicate boolean or hardcoded named right remains.
- `getChatInviteLinkCounts`: regular user; three DNF alternatives, each `basic_group|supergroup|channel AND ChatOwner` for `chat_id`.
- Existing create/replace contracts retain regular-user+bot scope and `can_invite_users` administrator right.
- Owner-signal partition is exhaustive: four username-owner contracts, two complete contracts in other families, twelve deferred mixed methods.
- Signature/source drift, bot-enabled method policy and extra argument signal fail closed. Capability format remains `8`; executable code contains no planning/task IDs.

## TDD and exact oracles

- Red: pinned count test first returned `SchemaDrift` because the owner signal was deferred.
- Green/refactor: `RequiredAccess::Owner`, regular-user validation, three-kind owner DNF and per-contract signal consumption made the focused and whole-workspace tests green.
- Supported typed set: 67, SHA-256 `dde02998c0f1cd47b9dc30383c11b8d7e815e128a39e9ab53f0c0f772574a417`.
- Terminal set: 70, SHA-256 `d859cc0a687bca878d715900e5750cc208a335e7ada8621c44e77c423b23dedf`.
- Open set: 123, SHA-256 `38dd369d689f9924166f54934b1e4207ddfd9fec692e3f4219b76dac4ee19fbb`.
- Semantic disposition rows: SHA-256 `841d9b9e16822b6e264b814401f4583e8559a06f786181bb28636c783c11f14f`.
- Verification before documentation: 61 generator + 24 core = 85 workspace tests; Clippy `-D warnings`, rustfmt, planning and diff gates green with `jobs=2`.
- Independent source/code review: `APPROVED`; actionable findings absent.

## Boundary

`ChatOwner` is a prerequisite, not current proof. The future singleton daemon must provide current account/target/session-bound membership status and write-access evidence, including active basic-group state, and invalidate it after role, membership, chat-state, session-generation or update-gap changes. Missing/stale evidence fails closed.
