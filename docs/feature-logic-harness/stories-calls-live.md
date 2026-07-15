# Feature Logic Harness: Stories, звонки и live

## Summary

- Feature ID: F015
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: покрыть полный TDLib surface stories, private/group calls, video chats и live streams с честным asynchronous lifecycle.
- Product workflow/job served: resolve peer/chat -> load state/capabilities -> start/join/control/leave or publish/edit story -> wait terminal evidence -> cleanup.
- Primary ambiguity to keep explicit: media transport and real-time audio/video engine remain внешними зависимостями TDLib workflow.

## Product Context

- Product context source: product.md
- Product purpose: агент управляет story/call/live объектами и тестирует их account-side состояние.
- Primary users: owner, channel operator, moderator и QA agent.
- Core workflows touched: stories, active stories, call protocols, group calls/video chats, live streams and participants.
- Domain terms used: story, call ID, group call, join payload, participant, live stream.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: full TDLib and routine operations; limits: none.
- SRC002: HARNESS.md; type: file; supports: async proof and policy dimensions; limits: none.
- SRC003: pinned official schema; type: supplied; supports: story/call/group-call families; limits: source alone does not prove generated registry.
- SRC004: plans.md P4/P7/P10; type: file; supports: terminal updates and live gate; limits: implementation absent.

## TDLib API Coverage

- Primary owner: story CRUD/views/privacy/interactions, calls and call protocol, group calls/video chats, participants, recording/screen sharing/live-stream control.
- Story statistics and revenue belong to F019; media files to F010; channel moderation rights to F011.
- Exact method registration/classification is generated from the pinned schema and checked fail-closed.

## Request Graph

`resolve peer/chat -> fetch current state -> validate capability/privacy/rights -> prepare media/protocol -> submit start/join/publish/control -> consume ordered updates -> prove terminal state -> leave/close resources`.

## Completion Proof

Request acknowledgement is partial. Completion requires method-specific state: published/updated/deleted story on reread, or expected call/group-call terminal update and resource cleanup.

## Cache and Update Semantics

Call and participant state is update-led and ordered. Story lists are paginated snapshots with freshness metadata; gaps force resync before terminal claims.

## Retry and Reconciliation

Reads may retry within deadline. Publish/start/join/control operations reconcile IDs and current state after timeout; no blind duplicate story or call creation.

## CLI/MCP Exposure

CLI/MCP expose lifecycle operations and event streams. Real-time payloads, encryption material and media sockets remain outside model-visible JSON.

## Permissions and Account Capabilities

Check user/chat type, story privacy, posting/admin rights, call availability, participant permissions and server-provided limits.

## Live Verification Boundary

No live story/call mutation has been performed. Live tests require consenting test peers/channels and deterministic teardown.

## Scope

### In scope

- Every pinned TDLib story, call, group-call, video-chat and live-stream method/object/update.

### Out of scope

- Implementing codecs/WebRTC transport, recording media content or contacting real users without explicit approval.

### Ambiguous

- The first supported media/call adapter for end-to-end tests remains undecided; see Q001.

## Context Map

- User surfaces: lifecycle commands, watches and sanitized state summaries.
- Backend surfaces: reducer, workflow engine, media adapter and lease manager.
- Data entities: StoryRef, CallRef, GroupCallRef, ParticipantState, LiveResourceLease.
- External dependencies: Telegram real-time services and media transport.
- Async flows: upload/publish, call states, participant updates, leave/close.
- Config flags: deadlines, event buffer and test-peer allowlist.
- Tests/examples/docs: fake state machines plus approved live fixtures.
- Observability: state transition/duration only; peer identifiers excluded from labels.

## Actors and Permissions

- Read agent: observes permitted stories and call metadata.
- Operator/moderator: controls owned channel/group live sessions.
- QA peer: consenting counterpart for test calls.

## Domain Entities

- StoryRef: story identity, owner, privacy and freshness.
- CallRef/GroupCallRef: server ID and current ordered state.
- LiveResourceLease: media/join resources requiring cleanup.

## State Model

- Story: Draft/Prepared -> Submitted -> Published/Failed/Uncertain -> Edited/Deleted.
- Call: Idle -> Starting/Joining -> Active -> Leaving/Discarded/Failed -> Cleaned.

## Operations and Data Model

- Operations: list/get/post/edit/delete/view stories; create/accept/join/control/leave/discard calls and live sessions.
- Reads: state, participants, streams, limits and privacy settings.
- Writes: story/call/group-call state.
- Side effects: notifications, presence, network/media and public publishing.
- Shapes: transition timeline plus terminal proof and cleanup state.

## Contracts

- C001: async acknowledgement never equals active/published state.
- C002: live resource lease is released in finally paths.
- C003: uncertain create/publish reconciles before retry.

## Invariants

- I001: no unapproved real-user call or public story.
- I002: ordered updates are the authority for call transitions.
- I003: media secrets never enter logs or agent output.

## Dimensions

- D001 - Lifecycle/evidence
  - Description: requested/acknowledged/update-confirmed/cleaned; Status: partial; Values: story and call transitions; Boundary values: joined response without active update; Why it matters: truthful completion; Related entities: StoryRef/CallRef/LiveResourceLease; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Target and media context
  - Description: owned/consenting/unapproved target crossed with prepared/unavailable media adapter; Status: partial; Values: combinations; Boundary values: live request to unapproved peer; Why it matters: privacy and teardown; Related entities: StoryRef/CallRef/LiveResourceLease; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.

## Domain Overlays Used

- Async lifecycle, privacy, external media and cleanup overlays.

## Scenario Cells

- SC001 - Group call join stalls
  - Dimensions: D001, D002; Workflow/entity anchor: GroupCallRef; Scenario: join request returns but active update misses deadline; Expected behavior: partial/uncertain, reconcile then leave resources; Related contracts: C001-C002; Related invariants: I002-I003; Why this matters: resource safety; Status: modeled.
- SC002 - Story publish response lost
  - Dimensions: D001, D002; Workflow/entity anchor: StoryRef; Scenario: timeout after upload; Expected behavior: reread owned stories before retry; Related contracts: C003; Related invariants: I001; Why this matters: duplicate prevention; Status: modeled.

## Assumptions

- A001: core owns signaling state but media adapters are replaceable; support_basis: inference.

## Open Questions

- Q001: which media adapter and disposable peer/channel fixtures enter P10; owner: maintainer; blocking for live media acceptance only.

## Coverage Notes

- Kernel coverage: states, evidence and cleanup modeled.
- Modeled: full method families at domain level.
- Partial: exact mapping and live media adapter.
- Unknown: test fixture topology.
- Not applicable: payment flows.
