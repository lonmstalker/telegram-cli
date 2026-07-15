# Feature Logic Harness: пользователи, контакты и профиль

## Summary

- Feature ID: F007
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: получать и управлять user/profile/contact данными с корректным update-cache и capability rules.
- Product workflow/job served: resolve user -> hydrate profile/full info -> read or controlled update.
- Primary ambiguity to keep explicit: доступность телефонов/полных данных задаётся Telegram и privacy, не платформой.

## Product Context

- Product context source: product.md
- Product purpose: полный TDLib доступ без ложного absent/permission результата.
- Primary users: агент и владелец аккаунта.
- Core workflows touched: getMe, users/full info, contacts/import/search, usernames/profile updates.
- Domain terms used: user identity, contact, public profile, private field.
- Open product questions: none.

## Source Ledger

- SRC001: product.md; type: file; supports: purpose/trust; limits: none.
- SRC002: HARNESS.md; type: file; supports: full API/update invariants; limits: none.
- SRC003: pinned official `td_api.tl`; type: supplied; supports: method/type/update families; limits: source alone does not prove generated registry.
- SRC004: plans.md P7; type: file; supports: delivery gate; limits: implementation absent.

## TDLib API Coverage

- Primary owner: user/profile/contact/import/search/status/username families and their updates.
- Privacy/password/session methods remain F016; chat-member administration remains F011.
- Exact method registrations are generated from schema identity; cross-domain dependencies remain explicit.

## Request Graph

`ensure Ready -> resolve ID/username/contact -> wait updateUser/updateUserStatus -> optional full info -> capability/privacy check -> operation -> verify update`.

## Completion Proof

User is known after authoritative cache/update or successful exact resolver. Missing private fields mean unavailable/hidden, not incomplete user existence.

## Cache and Update Semantics

Reduce user/full-info/status updates in order; unknown ID triggers resolver/hydration. Gap marks related profiles partial until resync.

## Retry and Reconciliation

Reads may bounded-retry after hydration. Contact/profile writes read current state and verify resulting update; non-idempotent imports use operation receipts.

## CLI/MCP Exposure

Raw and curated `user resolve/show`, `contacts list/search/import`, `profile update`; sensitive fields redacted by default on both surfaces.

## Permissions and Account Capabilities

Regular/bot accounts and privacy rules differ. Private phone/identity fields require explicit sensitive-read scope and may remain unavailable.

## Live Verification Boundary

Current live proof only confirms regular-user `getMe`; contact/profile workflows are not verified.

## Scope

### In scope

- User identity/full info/status, contacts, imports, public usernames, profile fields and block-list handoff.

### Out of scope

- Scraping hidden personal data; chat membership/moderation; account password/session administration.

### Ambiguous

- None; runtime rights/privacy are structured capability outcomes.

## Context Map

- User surfaces: user/contact/profile CLI/MCP workflows.
- Backend surfaces: resolver, user cache, policy/redaction.
- Data entities: User, UserFullInfo, ContactImport, Usernames.
- External dependencies: TDLib/privacy settings.
- Async flows: status/profile updates and contact synchronization.
- Config flags: sensitive-read scope and output redaction.
- Tests/examples/docs: codec/reducer/privacy/permission scenarios.
- Observability: counts/status only, no phone/name labels.

## Actors and Permissions

- Агент: ordinary profile/contact reads; scoped writes.
- Владелец: sensitive-read/profile-change approval.
- Core: redaction and capability enforcement.

## Domain Entities

- UserIdentity: stable ID/type/public fields.
- ContactImport: input fingerprint and per-entry result.
- ProfileView: source/freshness/redaction metadata.

## State Model

- Unknown -> Resolving -> Known/Unavailable/Forbidden; writes: Planned -> Applied -> Verified/Uncertain.

## Operations and Data Model

- Operations: resolve/show/list/search/import/remove/update.
- Reads: user/contact caches and TDLib full info.
- Writes: Telegram contacts/profile; no local secret copy.
- Side effects: contact/profile mutation with audit.
- Shapes: redacted profile plus completeness/capability fields.

## Contracts

- C001: unknown user ID triggers supported resolver before `not_found`.
- C002: sensitive fields are redacted unless explicitly scoped.
- C003: profile/contact mutation is verified by state/update.

## Invariants

- I001: hidden data is never fabricated.
- I002: user cache follows ordered updates.
- I003: telemetry contains no personal-field labels.

## Dimensions

- D001 - Account/privacy capability
  - Description: regular/bot and visible/hidden fields; Status: filled; Values: capability combinations; Boundary values: sensitive field hidden; Why it matters: result/policy; Related entities: ProfileView; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Resolution/cache state
  - Description: cached, stale, missing or gapped identity/contact state; Status: filled; Values: cached, stale, missing, gapped; Boundary values: public username absent from cache; Why it matters: false-not-found prevention; Related entities: ProfileView/ContactMutation; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.

## Domain Overlays Used

- Privacy/data lifecycle: PII redaction and update-driven identity.

## Scenario Cells

- SC001 - Unknown username
  - Dimensions: D001, D002; Workflow/entity anchor: user resolve; Scenario: cache miss with public username; Expected behavior: resolver then known/not-found proof; Related contracts: C001; Related invariants: I001/I002; Why this matters: avoids false absence; Status: modeled.
- SC002 - Hidden phone
  - Dimensions: D001, D002; Workflow/entity anchor: profile show; Scenario: TDLib omits field; Expected behavior: unavailable/hidden, no guess; Related contracts: C002; Related invariants: I001/I003; Why this matters: privacy; Status: modeled.

## Assumptions

- A001: generated schema classification resolves contact/privacy boundary explicitly; support_basis: inference.

## Open Questions

- None.

## Coverage Notes

- Kernel coverage: identity/privacy/cache modeled.
- Modeled: primary read/write flows.
- Partial: exact schema classification and live rights matrix.
- Unknown: none blocking.
- Not applicable: chat content and payments.
