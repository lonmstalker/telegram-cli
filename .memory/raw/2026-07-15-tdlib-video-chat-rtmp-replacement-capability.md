# TDLib video chat RTMP replacement capability contract

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `replaceVideoChatRtmpUrl` capability contract in pinned TDLib. It proves prerequisites for revoking/replacing streaming credentials, not current owner evidence, dialog freshness or server success.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `vendor/tdlib/td_api.tl` exact signature: `replaceVideoChatRtmpUrl chat_id:int53 = RtmpUrl;`; exact description requires owner privileges in the chat.
- [`Requests.cpp` lines 5102-5106](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L5102-L5106): dispatcher executes `CHECK_IS_USER()` and calls the shared handler with `is_story=false`, `revoke=true`.
- [`GroupCallManager.cpp` lines 2781-2794](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/GroupCallManager.cpp#L2781-L2794): shared local precheck requires dialog read access and `can_manage_video_chats` before the revoke RPC.
- [`GroupCallManager.cpp` lines 415-447](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/GroupCallManager.cpp#L415-L447): query sends `phone.getGroupCallStreamRtmpUrl` with `revoke=true`; no local active-call, RTMP-state or `group_call_id` gate exists.

## Model

- Existing `capability/video_chat_streaming.rs` now has closed `RequiredAccess::{AdministratorRight, Owner}`; the same access value determines DNF and consumed signals.
- `replaceVideoChatRtmpUrl`: regular user; three alternatives, each `basic_group|supergroup|channel AND ChatOwner` for `chat_id`.
- Public owner requirement is stricter than the shared local administrator precheck. `ChatOwner` implies video-chat management, so no redundant administrator-right atom is added.
- Private/secret alternatives and owner-unrelated call/RTMP state predicates are not invented.
- Source/signature drift, bot-enabled method policy and an additional argument signal fail closed. Executable code contains no planning/task IDs.

## TDD and exact oracles

- Red: pinned replacement test first returned `SchemaDrift` because the owner signal was deferred.
- Green/refactor: the streaming contract became a two-row table with closed required access; the method moved from deferred to the existing complete-elsewhere owner partition.
- Supported typed set: 69, SHA-256 `1e5cb8b56a2295d98918f81ae5006f7452fe0deff5d332cef459080ce3a0e92c`.
- Terminal set: 72, SHA-256 `2ee3321d89f3463b8ce90c123b9beaff6e22e9282a3ff90dbca59abd29f1b5fe`.
- Open set: 121, SHA-256 `f12c4e511942b14979dc26a17bc4797ff05bbcaceda7f45625829960222faf0c`.
- Semantic disposition rows: SHA-256 `4cf97a1d10c9c5bb3845aaf17ca2509016cad03eb3188dda679f5cd1116c40d3`.
- Owner complete-elsewhere partition: SHA-256 `d12d2d98a15c1a7788957c32e92792988ac7822febb6f9464fcc2d3f43231d06`; deferred owner partition: `1fc8b2af9778253531a385d50fe619316fdf51a0c615f7296f6a31d2badec933`.
- Verification before documentation: 63 generator + 24 core = 87 workspace tests; Clippy `-D warnings`, rustfmt, planning and diff gates green with `jobs=2`.
- Independent source/Rust review: `APPROVED`; actionable findings absent.

## Boundary

`ChatOwner` and read access are prerequisites, not current proof. The future singleton daemon must provide current account/target/session-bound kind, owner and dialog-access evidence, invalidated on role, membership, kind, generation or update-gap changes. A static pass does not guarantee successful revoke or new credentials from Telegram.
