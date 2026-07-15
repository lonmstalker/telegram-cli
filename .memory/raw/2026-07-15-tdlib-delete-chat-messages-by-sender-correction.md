# TDLib delete-chat-messages-by-sender capability correction

Date: 2026-07-15

## Scope

Sanitized evidence correcting the existing static `deleteChatMessagesBySender` contract. The previous contract captured public kind/right wording but omitted pinned account and direct-messages-group guards, so it could produce false-positive capability verdicts.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- `vendor/tdlib/td_api.tl` exact signature: `deleteChatMessagesBySender chat_id:int53 sender_id:MessageSender = Ok;`; description restricts the method to supergroups and requires `can_delete_messages`.
- [`Requests.cpp` lines 4127-4132](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L4127-L4132): dispatcher executes `CHECK_IS_USER()` before delegating.
- [`MessageQueryManager.cpp` lines 3412-3425](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/MessageQueryManager.cpp#L3412-L3425): deeper path repeats the non-bot invariant, requires write access, checks sender availability and delegates only a channel-dialog target.
- [`DialogManager.cpp` lines 2875-2898](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogManager.cpp#L2875-L2898): target must be a megagroup, must not be monoforum and must have `can_delete_messages`.
- [`ChatManager.cpp` lines 9850-9862](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/ChatManager.cpp#L9850-L9862): internal `is_monoforum` is exported in the ordered `supergroup` object position of public `is_direct_messages_group`.

## Corrected model

- Exact static prerequisites are `RegularUser + Supergroup + SupergroupFlag(is_direct_messages_group=false) + ChatAdministratorRight(can_delete_messages)`.
- Broadcast channels are already excluded by `ResolvedChatKind::Supergroup`; the additional flag excludes direct-messages/monoforum supergroups. No unsupported gigagroup restriction is invented.
- Write access and sender availability are runtime freshness/availability boundaries, not new static role atoms.
- The row moved from the legacy generic table to `capability/message_moderation.rs`, which pins exact signature/source/account/kind/subtype/right semantics. Executable code contains no planning/task IDs.

## TDD and verification

- First red: a bot-enabled descriptor returned `Ok(())` despite pinned `CHECK_IS_USER()`.
- Deeper-source review found the monoforum guard. A negative control with the subtype flag removed reproduced the old broad DNF and failed exact equality; the corrected flag restored green.
- Regular-user exact descriptor is accepted; bot-enabled and broad-supergroup descriptors are rejected. Source/signature/additional-argument-signal drift fail closed.
- Supported/terminal/open oracles remain 70/73/120 because this is a correction of an already dispositioned method, not new coverage. Semantic disposition SHA-256 remains `d050be73e8e9211e624719be10050ca2829891befdadd02ad7f6e975442d370e`.
- Verification: 65 generator + 24 core = 89 workspace tests green; rustfmt, Clippy `-D warnings`, project gates and diff checks green with `jobs=2`.
- Independent source/Rust review: `APPROVED` after the subtype correction; actionable findings absent.

## Boundary

The static descriptor does not prove current write access, administrator status, target freshness or sender availability and does not promise server success. The future singleton daemon must bind all facts to the current account/target/session generation and fail closed on missing, stale or gap-affected evidence.
