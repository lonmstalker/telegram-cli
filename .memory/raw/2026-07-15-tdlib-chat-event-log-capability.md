# TDLib chat event log capability contract

Date: 2026-07-15

## Scope

Sanitized evidence for the exact static prerequisite contract of `getChatEventLog` in pinned TDLib. This digest does not prove current administrator status, runtime freshness, daemon readiness or a successful live request.

## Pinned source

- TDLib version/commit: `1.8.66`, `07d3a0973f5113b0827a04d54a93aaaa9e288348`.
- Reviewed source archive SHA-256: `1a8c9429af7b2eae2e36f24db9a9b8f523dbd5bd2ae448f8bb9322bb1c80dbfb`.
- `vendor/tdlib/td_api.tl` exact signature: `getChatEventLog chat_id:int53 query:string from_event_id:int64 limit:int32 filters:chatEventLogFilters user_ids:vector<int53> = ChatEvents;`.
- Exact normalized method description states that the method is available only for supergroups and channels and requires administrator rights.
- [`Requests.cpp` lines 6329-6336](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/Requests.cpp#L6329-L6336): request dispatcher executes `CHECK_IS_USER()` before `get_dialog_event_log`; bot accounts are not accepted.
- [`DialogEventLog.cpp` lines 658-681](https://github.com/tdlib/td/blob/07d3a0973f5113b0827a04d54a93aaaa9e288348/td/telegram/DialogEventLog.cpp#L658-L681): target must use TDLib `DialogType::Channel`, which covers Telegram supergroups and channels, and current channel status must be administrator.

## Model

- `capability/chat_event_logs.rs` pins one method, its complete canonical signature, exact normalized description and closed supported-kind set.
- Static DNF has exactly two alternatives: `supergroup AND ChatAdministrator` or `channel AND ChatAdministrator` for `chat_id`.
- Method account scope is regular-user-only. This is a method-axis constraint, not a runtime atom.
- The exact method-description `RequiresAdministrator` signal is consumed by the typed DNF. Any extra argument-level signal remains unconsumed and fails closed.
- Source text drift, signature drift, bot-enabled policy and additional runtime-signal drift are negative controls returning `SchemaDrift` or `InvalidPolicy` as appropriate.
- The semantic module is included in the capability engine source hash. Capability format remains `8`; no planning/task identifiers appear in executable code or machine-readable contracts.

## TDD and exact oracles

- Red: the pinned method test first returned `SchemaDrift` because the administrator signal had no typed terminal disposition.
- Green/refactor: the exact semantic module, account validation, consumed-signal set and two-branch DNF made the focused test and whole workspace green without adding a new domain atom.
- Supported typed set: 67, SHA-256 `354bd086523c04cb6ac2f8f35f92b770cdc67c16a36bc7f44be16445979572db`.
- Terminal set: 70, SHA-256 `9de489e0204801f32ba0d644def028f8519de0946e560b1875bf4742182829bf`.
- Open set: 123, SHA-256 `a142adc309d4c392ae78f34437eb0568b23b4e69d0a576db335bab659b572b10`.
- Semantic disposition rows: SHA-256 `1059126baece82a1200222e92f3ad4166191f629b5f9979b39c907a9f35f1414`.
- Verification before documentation: 59 generator + 24 core = 83 workspace tests; Clippy `-D warnings`, rustfmt, planning/workspace/schema/native/skeleton/diff gates green with `jobs=2`.
- Independent code review: `APPROVED`; actionable findings absent. Reviewer repeated 59 generator tests and `git diff --check`.

## Boundary

`ChatAdministrator` is only a static request prerequisite. The future singleton daemon must derive it from current account/target-bound Telegram state and invalidate it after membership, role, chat-kind, session-generation or update-gap changes. Missing or stale evidence must fail closed. The 48-hour event window and reverse chronological ordering describe result semantics, not caller capability.
