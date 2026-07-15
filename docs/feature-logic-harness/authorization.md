# Feature Logic Harness: авторизация и секреты

## Summary

- Feature ID: F002
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: безопасно входить и повторно открывать зашифрованную TDLib-сессию во всех состояниях авторизации.
- Product workflow/job served: first login, returning login, re-auth и key rotation.
- Primary ambiguity to keep explicit: конкретный local keychain backend выбирается при packaging.

## Product Context

- Product context source: product.md
- Product purpose: агент использует аккаунт, не получая auth/encryption secrets.
- Primary users: владелец аккаунта и оператор; агент видит только status/challenge.
- Core workflows touched: setup profile, unlock DB, authentication challenges, Ready/getMe.
- Domain terms used: database key, challenge, secret channel, Ready.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: trust boundary; limits: none.
- SRC002: HARNESS.md; type: file; supports: secret and lifecycle invariants; limits: none.
- SRC003: plans.md P1 и `telegram-core::authorization`; type: file/code; supports: exhaustive state/challenge machine; limits: database-key provider и runtime driver отсутствуют.
- SRC004: pinned official `td_api.tl`; type: supplied; supports: 13 auth states and auth methods; limits: human UI not specified.
- SRC005: live probe 2026-07-15; type: supplied; supports: encrypted returning Ready/getMe/Closed; limits: first-login branches not tested.

## TDLib API Coverage

- Primary owner: authorization-state methods: `setTdlibParameters`, database-key checks, phone/QR/code, authentication password/email challenges, registration and device confirmation.
- Exact functions/objects/updates are generated from schema and must cover all 13 authorization states.
- Ready-state account password/recovery settings belong to F016; logout/destroy risk classification cross-references F001/F016.

## Request Graph

`profile -> parameters -> database key -> auth state loop -> human challenge via secure channel -> Ready -> getMe identity proof`.

## Completion Proof

Login is complete only after `authorizationStateReady` and successful `getMe` matching the expected profile identity. Challenge acceptance or DB unlock alone is insufficient.

## Cache and Update Semantics

Latest authorization state is authoritative only from ordered updates/current state. Restart resumes the state machine without assuming Ready.

## Retry and Reconciliation

Wrong key fails closed. OTP/2FA attempts are bounded and never replayed automatically. Network failure resumes from fresh authorization state.

## CLI/MCP Exposure

CLI provides secure TTY/file-descriptor/keychain login. MCP exposes `auth.begin`, `auth.status` and `auth.wait`; the owner submits OTP, 2FA or database key through a separate protected CLI/TTY or server operator channel bound to the challenge ID.

## Permissions and Account Capabilities

Only account owner/operator may submit auth secrets. Agent may wait/poll status but cannot initiate logout/destroy without destructive approval.

## Live Verification Boundary

Server key was copied over SSH to ignored local storage with mode `0600`; digest matched generation 25. Returning regular-user session reached Ready/getMe and clean Closed. Pure `telegram-core::authorization` machine обрабатывает все pinned states и exact QR/phone/code/2FA/email/device/registration requests; database-key/runtime integration ещё отсутствует. Secret value was not emitted.

## Scope

### In scope

- First/returning auth, encrypted DB, all schema auth states, secure secret input, identity proof, re-auth.

### Out of scope

- Secret recovery by the model, bypassing Telegram retries, storing OTP in config.

### Ambiguous

- The macOS default between Keychain and a file secret remains undecided; see Q001.

## Context Map

- User surfaces: CLI secure prompt, operator admin channel, MCP auth status.
- Backend surfaces: core auth loop and secret provider interface.
- Data entities: AuthState, Challenge, SecretReference, ExpectedIdentity.
- External dependencies: TDLib and OS secret storage.
- Async flows: state updates/challenge expiration.
- Config flags: DC, DB/files, key reference, expected user ID.
- Tests/examples/docs: plans.md P1/P10.
- Observability: redacted state transitions and attempt counters.

## Actors and Permissions

- Владелец: вводит phone/code/2FA/email/device confirmation.
- Оператор: настраивает file/keychain secret references.
- Агент: читает non-secret state only.

## Domain Entities

- Challenge: kind, expiry, attempts, one-time status; no secret value.
- SecretReference: opaque file descriptor/path/keychain ID.
- ExpectedIdentity: stable account proof checked after Ready.

## State Model

- TDLib authorization states are mirrored losslessly; Ready/Closed are terminal for one auth run.
- Wrong key -> Failed without phone fallback.
- Reauth-required -> fresh challenge, not silent relogin.

## Operations and Data Model

- Operations: configure profile, unlock, begin/status/submit challenge, verify identity.
- Reads: auth status and safe challenge metadata.
- Writes: encrypted DB and redacted audit state.
- Side effects: Telegram login only after explicit owner input.
- Input and output shapes: secrets via protected provider; protocol returns codes/status only.

## Contracts

- C001: every authorization state has explicit handling.
- C002: missing/wrong DB key cannot trigger a new phone login.
- C003: Ready requires getMe identity verification.

## Invariants

- I001: secret values never cross model-visible protocol.
- I002: one profile cannot silently bind another Telegram identity.
- I003: normal daemon restart reuses the existing authorization.

## Dimensions

- D001 - Auth context
  - Description: first/returning/re-auth/partial; Status: filled; Values: four; Boundary values: wrong key, expired code; Why it matters: state path; Related entities: Challenge; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Secret provider
  - Description: TTY/file/keychain/operator channel; Status: partial; Values: provider types; Boundary values: remote MCP; Why it matters: exposure boundary; Related entities: SecretReference; Related contracts: C002; Related invariants: I001; Unknowns: Q001.

## Domain Overlays Used

- Security/account lifecycle: all auth branches and secret ownership.

## Scenario Cells

- SC001 - Returning encrypted session
  - Dimensions: D001, D002; Workflow/entity anchor: unlock; Scenario: correct key, existing auth; Expected behavior: Ready/getMe without prompt; Related contracts: C002/C003; Related invariants: I001-I003; Why this matters: current primary path; Status: modeled.
- SC002 - Wrong key
  - Dimensions: D001, D002; Workflow/entity anchor: unlock; Scenario: key mismatch; Expected behavior: fail closed, no phone prompt; Related contracts: C002; Related invariants: I001; Why this matters: prevents accidental new login; Status: modeled.

## Assumptions

- A001: remote installation has an operator-controlled secret channel; support_basis: explicit_user_decision.

## Open Questions

- Q001: какой secret backend является default для macOS; owner: operator; non-blocking.

## Coverage Notes

- Kernel coverage: all auth states identified.
- Modeled: returning/wrong-key/security semantics.
- Partial: first-login state/challenge core готов; database-key provider, runtime driver, UI и key rotation отсутствуют.
- Unknown: default local secret backend.
- Not applicable: chat/message domain behavior.
