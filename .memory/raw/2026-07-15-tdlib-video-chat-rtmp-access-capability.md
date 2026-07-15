# TDLib video chat RTMP access capability contract

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `getVideoChatRtmpUrl` capability contract in pinned TDLib. It proves request prerequisites, not current rights freshness, dialog availability, server success or the existence/state of an active video chat.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `vendor/tdlib/td_api.tl` exact signature: `getVideoChatRtmpUrl chat_id:int53 = RtmpUrl;`.
- Exact normalized description requires `can_manage_video_chats` administrator right.
- [`Requests.cpp` lines 5095-5100](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L5095-L5100): dispatcher executes `CHECK_IS_USER()` and calls `get_video_chat_rtmp_stream_url(dialog_id, false, false)`.
- [`GroupCallManager.cpp` lines 2781-2794](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/GroupCallManager.cpp#L2781-L2794): handler first requires read access and, for this non-story request, delegates to `can_manage_video_chats`.
- [`GroupCallManager.cpp` lines 2356-2379](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/GroupCallManager.cpp#L2356-L2379): basic groups use chat permissions, channel-dialog targets use channel permissions, and user/secret dialogs are rejected. Channel-dialog targets cover both TDLib supergroups and channels.

## Model

- Semantic module `capability/video_chat_streaming.rs` owns the source/signature/right/kind contract; executable code contains no planning/task IDs.
- Method scope is regular user only.
- Static DNF has exactly three alternatives for `chat_id`: `basic_group|supergroup|channel AND ChatAdministratorRight(can_manage_video_chats)`.
- Private and secret chats have no alternative. No owner-only, active-call, RTMP-state or `group_call_id` prerequisite is invented.
- Read access is a runtime availability/freshness boundary, not an additional static role atom.
- Full signature/source drift, bot-enabled policy and an extra argument-level signal fail closed.

## TDD and exact oracles

- Red: pinned contract test first returned `SchemaDrift` because the method signal was deferred.
- Green/refactor: exact semantic contract, method account scope, three-kind typed DNF and exact consumed-signal set made the focused and corpus tests green.
- Supported typed set: 68, SHA-256 `21eec49724a797737a89712bad46e53e0a9ef6e9e1462b3cbaaec4d3e0199834`.
- Terminal set: 71, SHA-256 `20b00f041a5809d51dc539a8141dcfdabceb7d59fef5038c835f1482e8433704`.
- Open set: 122, SHA-256 `df35fcbf3d7ed48c81bba37beaeea8d407d8066ba4b90f1ff8c8bc9ce59e35da`.
- Semantic disposition rows: SHA-256 `1c607a624b2e3a610996afcf6aa7bf0b4278badc683e3e1b436297f32b6b5268`.
- Verification before documentation: 62 generator + 24 core = 86 workspace tests; Clippy `-D warnings`, rustfmt and project gates green with `jobs=2`; `target` remained 151 MiB and no project process remained.
- Independent source/Rust review: `APPROVED`; actionable findings absent.

## Boundary

`ChatAdministratorRight` and read access are prerequisites, not current proof. The future singleton daemon must derive current account/target/session-bound chat kind, membership/right and dialog-access evidence and invalidate it after membership, rights, chat kind, session generation or update-gap changes. Missing or stale evidence fails closed; a successful static check does not promise that Telegram will return an RTMP URL.
