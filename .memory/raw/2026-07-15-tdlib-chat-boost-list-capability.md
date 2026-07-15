# TDLib chat-boost list capability

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static `getChatBoosts` capability contract in pinned TDLib. It proves the accepted account and administrator prerequisite without inventing a chat-kind restriction; it does not prove current read access, request-value validity or Telegram server success.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`; cached source archive SHA-256 `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- [`td_api.tl` lines 13729-13734](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/generate/scheme/td_api.tl#L13729-L13734) defines `getChatBoosts chat_id:int53 only_gift_codes:Bool offset:string limit:int32 = FoundChatBoosts;` and requires administrator rights in a generic chat.
- [`Requests.cpp` lines 5959-5964](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L5959-L5964) applies `CHECK_IS_USER()`, cleans `offset` and forwards all request values.
- [`BoostManager.cpp` lines 481-490](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/BoostManager.cpp#L481-L490) requires generic dialog read access and a positive `limit`; it adds no dialog-type or local administrator-status branch.
- [`BoostManager.cpp` lines 264-308](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/BoostManager.cpp#L264-L308) obtains a generic read `InputPeer` and sends the list query without an account-, kind- or value-dependent capability gate.
- [`telegram_api.tl` line 3075](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/generate/scheme/telegram_api.tl#L3075) maps `only_gift_codes` to the RPC `gifts` flag; it remains an invocation value.

## Model

- New semantic `capability/chat_boosts.rs` contains one exact source/signature-bound contract. It reuses the existing `ChatAdministrator` runtime atom and shared administrator-signal disposition.
- Account scope is `RegularUser`. A bot-enabled descriptor is rejected by the pinned dispatcher contract.
- Static DNF has one atom: `ChatAdministrator(chat_id)`. There is deliberately no `ChatKind`: neither the exact method source nor the handler/query chain proves a positive supergroup/channel restriction.
- `only_gift_codes`, `offset` and `limit` are request/RPC values, not capability atoms. Positive-limit validation is invocation validity.
- Source/signature drift and an additional argument-level signal fail closed. Engine identity includes the new semantic module; executable code contains no planning/task IDs.

## TDD and exact oracles

- Red: the pinned test returned deferred `SchemaDrift` because the administrator signal had no typed disposition.
- Green/refactor: the minimal chat-boost module and a shared administrator-signal helper produce the exact descriptor without changing event-log semantics.
- Supported typed set: 73, SHA-256 `5f071d0f663b25ea1819b13afa107be02385c1076a1ae410d2c9bee9798439cb`.
- Terminal set: 76, SHA-256 `61cd3f0ae0ea7b4a4ac55908217d0b1b7f2b6e060d42ca14036605e7471166f1`.
- Open set: 117, SHA-256 `e39bd801f0cba2b684c0b9025e0a048a7e8a08e49541a63fbd66ab8a85078e98`.
- Semantic disposition rows: SHA-256 `6cf71ae778b3e7884164ba23b3005b7e1390916133d8666c22612d9d0d9a534b`.
- Verification: 68 generator + 24 core = 92 workspace tests green with `jobs=2`.
- Independent source/Rust review: `APPROVED`; actionable findings absent.

## Boundary

Read access, current administrator evidence, offset validity, limit bounds and Telegram server acceptance are runtime or invocation facts, not proved by the static descriptor. The future singleton daemon must bind role/access evidence to the current account/target/session generation and fail closed on missing, stale or gap-affected state. A static pass does not promise that Telegram will return boosts for a given chat or request page.
