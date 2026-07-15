# TDLib video chat creation capability contract

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `createVideoChat` capability contract in pinned TDLib. It proves caller and chat prerequisites, not current dialog/right freshness, value validity, network acceptance or successful group-call creation.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `vendor/tdlib/td_api.tl` exact signature: `createVideoChat chat_id:int53 title:string start_date:int32 is_rtmp_stream:Bool = GroupCallId;`.
- Exact normalized description restricts the method to basic groups, supergroups and channels and requires `can_manage_video_chats` administrator right.
- [`Requests.cpp` lines 5074-5088](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L5074-L5088): dispatcher executes `CHECK_IS_USER()`, cleans the title and delegates to video-chat creation.
- [`GroupCallManager.cpp` lines 2658-2674](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/GroupCallManager.cpp#L2658-L2674): handler requires dialog read access and `can_manage_video_chats` before sending the query.

## Model

- Domain module `capability/video_chats.rs` owns the three reviewed create/get/replace video-chat contracts; executable code contains no planning/task IDs.
- `createVideoChat` is regular-user-only.
- Static DNF has exactly three alternatives for `chat_id`: `basic_group|supergroup|channel AND ChatAdministratorRight(can_manage_video_chats)`.
- Private and secret chats have no alternative. `title`, `start_date` and `is_rtmp_stream` are request values with server/RPC validation, not capability atoms.
- Dialog read access is a runtime availability/freshness boundary, not another static role atom.
- Full signature/source drift, bot-enabled policy and an extra argument-level signal fail closed.

## TDD and exact oracles

- Red: the pinned creation-contract test first returned `SchemaDrift` because its runtime signal had no typed disposition.
- Green/refactor: the exact contract made the focused and corpus tests green; the broadened domain renamed `video_chat_streaming.rs` to `video_chats.rs` without changing getter/replacement semantics.
- Supported typed set: 70, SHA-256 `96d4b3382adf63541c60b2ca84b55d7995617213fb6b928bf64f6dd666b65fd5`.
- Terminal set: 73, SHA-256 `317e5524313cf2740c9732e94d4d9c9a5fe04f22c066057f0c16fced1b421aaa`.
- Open set: 120, SHA-256 `c525212cc279557aae39bd821e2c74d8912c01f0595e9f35f19f1259e7e4922d`.
- Semantic disposition rows: SHA-256 `d050be73e8e9211e624719be10050ca2829891befdadd02ad7f6e975442d370e`.
- Verification before documentation: 64 generator + 24 core = 88 workspace tests; Clippy `-D warnings`, rustfmt and project gates green with `jobs=2`.
- Independent source/Rust review: `APPROVED`; actionable findings absent.

## Boundary

`ChatAdministratorRight` and read access are prerequisites, not current proof. The future singleton daemon must provide current account/target/session-bound chat kind, right and dialog-access evidence and invalidate it after role, membership, kind, session generation or update-gap changes. A static pass does not guarantee that Telegram accepts title/start date/RTMP mode or creates the video chat.
