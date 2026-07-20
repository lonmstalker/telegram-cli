# Feature Logic Harness: чаты, списки, папки и темы

## Summary

- Feature ID: F008
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: согласованно находить, создавать и организовывать chats/lists/folders/topics, включая Saved Messages и secret chats.
- Product workflow/job served: load/resolve chat -> open/use -> organize/topic lifecycle -> close.
- Primary ambiguity to keep explicit: joining/membership belongs F011 и никогда не скрыт внутри read resolve.

## Product Context

- Product context source: product.md
- Product purpose: актуальный chat state без false not-found.
- Primary users: агент и владелец аккаунта.
- Core workflows touched: loadChats, chat lists/folders/archive, topics, private/secret chat creation.
- Domain terms used: chat list position, folder, topic, open lease.
- Open product questions: none.

## Source Ledger

- SRC001: product.md; type: file; supports: complete-result rule; limits: none.
- SRC002: HARNESS.md; type: file; supports: update/pagination invariants; limits: none.
- SRC003: pinned official schema/getting-started; type: supplied; supports: chat list/update semantics; limits: source alone does not prove generated registry.
- SRC004: plans.md P4/P7 и `telegram_core::workflows`; type: file/code; supports: list/open/topic chain acceptance; limits: live topic/admin fixture absent.

## TDLib API Coverage

- Primary owner: chat resolution/info/list/position/folder/archive/open/close, private/secret-chat creation and Saved Messages/direct/forum topic CRUD/list/read families.
- Membership/moderation F011; message contents F009; notifications F016.

## Request Graph

`Ready -> repeat loadChats -> reduce positions -> resolve/create -> optional openChat lease -> folder/topic operation -> wait update -> closeChat`.

## Completion Proof

Chat list reaches terminal only by documented load condition, not `len < limit`. Topic lists require cursor/no-progress proof; newly created chat requires returned object/update.

## Cache and Update Semantics

Chat and positions are reducer-owned; `getChats` is diagnostic only. Gap forces list/topic resync. Open lease keeps channel/supergroup updates active.

## Retry and Reconciliation

Read/load retries are bounded. Create/move/delete operations use idempotency and state probe after timeout.

## CLI/MCP Exposure

`chat resolve/list/create/open/close`, `folder *`, `topic *`; same protocol for CLI/MCP.

## Permissions and Account Capabilities

Secret chats/user accounts, forum/admin operations and folder limits depend on account/chat capability.

## Live Verification Boundary

P4/P7 реализуют отдельные read-only `resolve`/explicit `ensure_membership`, terminal-correct
main/archive/folder loader, paired chat inspection/open lease, cursor-safe forum topic list
и desired-state close/reopen с post-timeout state probe. Live read-only evidence 2026-07-19
закрывает returning auth, terminal main/archive lists и compact channel inventory без message/file
payload. Public-link resolve подтверждён на трёх точных channel fixtures без membership/open;
invite preview закрыт без membership/open. CHAT-006 2026-07-21 подтвердил scoped
`openChat`/`closeChat` на public supergroup под `read,presence`, включая deterministic
timeout cleanup; folder/forum fixtures остаются pending.

## Scope

### In scope

- Chat discovery/info/lists/folders/archive, open/close, Saved Messages/direct/forum topics and private/secret-chat creation.

### Out of scope

- Implicit joining, member moderation, message content workflows.

### Ambiguous

- None.

## Context Map

- User surfaces: chat/folder/topic workflows.
- Backend surfaces: chat reducer/list index/open lease manager.
- Data entities: Chat, ChatPosition, ChatFolder, Topic, OpenLease.
- External dependencies: TDLib updates.
- Async flows: loading, position changes, topic lifecycle.
- Config flags: page/load budget and open lease TTL.
- Tests/examples/docs: list short-batch/gap/topic pagination.
- Observability: loaded count, terminal reason, open leases.

## Actors and Permissions

- Агент: reads/organizes within scopes.
- Owner/admin: creation/settings requiring rights.
- Core: resolves without joining.

## Domain Entities

- ChatIndex: ordered positions by list.
- OpenLease: reason/client/TTL.
- TopicCursor: pagination and terminal proof.

## State Model

- Unknown -> Resolving/Loading -> Known/Open/Closed; list: Partial -> Complete/Gapped; topic: Active/Closed/Deleted.

## Operations and Data Model

- Operations: load/list/resolve/create/open/close/archive/folder/topic.
- Reads: reducer cache and TDLib pages.
- Writes: Telegram organization/settings state.
- Side effects: presence/open and chat creation are policy-classified.
- Shapes: chat view + source/freshness/completeness/cursor.

## Contracts

- C001: lists use `loadChats` + ordered updates.
- C002: resolve never joins implicitly.
- C003: openChat lifetime is paired with closeChat.

## Invariants

- I001: short batch is not terminal proof.
- I002: gap prevents complete list.
- I003: one workflow cannot close another client’s open lease.

## Dimensions

- D001 - Chat/load state
  - Description: unknown/loaded/open/gapped/terminal; Status: filled; Values: states; Boundary values: load returns fewer, gap; Why it matters: completion/presence; Related entities: ChatIndex/OpenLease; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Side-effect mode
  - Description: read-only resolve, presence/open and explicit create/organize mutation; Status: filled; Values: resolve, open, create, organize; Boundary values: resolver would join or create; Why it matters: policy and lifecycle; Related entities: ChatIndex/OpenLease; Related contracts: C002-C003; Related invariants: I001-I003; Unknowns: none.

## Domain Overlays Used

- Ordered collection/lifecycle: chat positions, pagination and open leases.

## Scenario Cells

- SC001 - Запрошено 100, загружено 20
  - Dimensions: D001, D002; Workflow/entity anchor: chat list; Scenario: TDLib returns smaller batch; Expected behavior: repeat load, not complete; Related contracts: C001; Related invariants: I001/I002; Why this matters: official semantics; Status: modeled.
- SC002 - Resolve public chat for read
  - Dimensions: D001, D002; Workflow/entity anchor: resolve; Scenario: chat not cached; Expected behavior: resolve/hydrate without join; Related contracts: C002; Related invariants: I002; Why this matters: avoids side effect; Status: modeled.

## Assumptions

- A001: per-client open leases can be reference-counted in broker; support_basis: inference.

## Open Questions

- None.

## Coverage Notes

- Kernel coverage: resolve/membership, main/archive/folder loading, inspection/open и forum-topic read/close lifecycle implemented.
- Modeled: chat/private/secret creation, Saved/direct topics и folder CRUD остаются universal raw/default-deny paths.
- Live: returning auth, terminal main/archive lists, compact channel inventory, public-link
  resolve, invite preview без membership и CHAT-006 scoped open/close pairing.
- Partial: live rights matrix и disposable forum/folder fixtures.
- Unknown: none blocking.
- Not applicable: message payload semantics.
