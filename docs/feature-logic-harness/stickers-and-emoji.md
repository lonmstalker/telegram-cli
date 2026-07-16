# Feature Logic Harness: Стикеры, emoji и реакции

## Summary

- Feature ID: F014
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: дать агенту полное schema-driven управление стикерами, custom emoji, emoji status, реакциями и наборами без неполных request chains.
- Product workflow/job served: найти или загрузить media -> проверить права/формат -> создать либо изменить набор -> дождаться подтверждения -> перечитать итог.
- Primary ambiguity to keep explicit: exact method registration/classification строится из pinned schema, а не из ручного списка в этом файле.

## Product Context

- Product context source: product.md
- Product purpose: агент создаёт и обслуживает наборы, использует emoji/reactions и получает актуальные данные.
- Primary users: owner, channel operator, content agent и QA agent.
- Core workflows touched: stickers, custom emoji, emoji status, saved/recent/favorite stickers, reactions, saved animations.
- Domain terms used: sticker set, custom emoji, reaction type, emoji status, upload artifact.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: user jobs and safety boundary; limits: none.
- SRC002: HARNESS.md; type: file; supports: semantic scope and required dimensions; limits: planning IDs are documentation-only.
- SRC003: pinned official schema; type: supplied; supports: sticker/emoji/reaction method families; limits: source alone does not prove generated registry.
- SRC004: plans.md P3/P4/P7/P10; type: file; supports: raw parity, workflow behavior and live matrix; limits: live mutation remains P10.
- SRC005: `crates/telegram-core/src/workflows.rs`, `apps/telegramd/src/server.rs`; type: file; supports: typed plan/apply, uploaded-file prerequisite, ownership/inventory reconciliation and cleanup proof; limits: curated lifecycle is regular-user only.

## TDLib API Coverage

- Primary owner: sticker sets and stickers, custom emoji, emoji status, reaction catalog/default/availability metadata, saved/recent/favorite stickers and animations, keyword/language lookup.
- Upload/download primitives belong to F010; message-level reaction application/removal/readers belong to F009; channel permission checks belong to F011.
- Every matching pinned-schema method is assigned by generated manifest; unassigned or multiply assigned methods fail CI.

## Request Graph

`prepare media via F010 -> terminal upload -> typed plan -> external exact approval -> create/add/delete once -> reread set/name -> return completeness proof`.

## Completion Proof

Mutation is complete only when TDLib confirms it and a reread or relevant update agrees. Upload success alone does not prove sticker-set mutation.

## Cache and Update Semantics

Curated lifecycle использует fresh `getFile`, `getStickerSet`, `searchStickerSet(ignore_cache)`
и `checkStickerSetName`, поэтому не выводит absence/ownership из stale cache. Receipt хранит
`observed_at` и server-snapshot freshness; общий raw surface не обещает cached `not_found`.

## Retry and Reconciliation

Lookup reads use bounded retry. Create/add/delete/reorder operations are not blindly repeated after uncertain outcome; reconcile by set ID/name and item inventory.

## CLI/MCP Exposure

Expose typed `upload_sticker_file` + plan/apply lifecycle and raw `td call`. Binary input uses local file handles or protected remote uploads, never inline secret/base64 blobs in agent context.

## Permissions and Account Capabilities

Check account type, set ownership, Premium/custom-emoji availability, chat rights and method-specific capability before mutation. Unsupported capability is not reported as absent API.

## Live Verification Boundary

Synthetic runtime tests prove terminal upload and create -> lost-response add reconciliation
-> delete/name-availability cleanup. No live sticker/emoji mutation has been executed; P10
uses a disposable test set and requires explicit destructive-operation permission immediately
before cleanup.

## Scope

### In scope

- Full pinned-schema sticker, custom emoji, emoji status, saved animation and reaction surface.

### Out of scope

- Image/video authoring itself; browser-side rendering assertions; bypass of Telegram ownership or Premium constraints.

### Ambiguous

- The reusable media-conversion boundary between CLI and F010 remains undecided; see Q001.

## Context Map

- User surfaces: CLI/MCP workflows and compact result envelopes.
- Backend surfaces: schema registry, file pipeline, update reducer and reconciliation journal.
- Data entities: StickerSetRef, StickerAsset, CustomEmojiRef, ReactionRef, EmojiStatusRef.
- External dependencies: Telegram, filesystem/upload broker and optional converter.
- Async flows: upload, mutation response, update and reread.
- Config flags: deadlines, max input size and conversion policy.
- Tests/examples/docs: fake update sequences plus disposable live set.
- Observability: method class, duration and outcome only; media/name identifiers remain out of labels.

## Actors and Permissions

- Read agent: searches and inspects sets/statuses.
- Content agent: applies reactions and saved items within scope.
- Owner/operator: approves set and account-status mutations.

## Domain Entities

- StickerSetRef: stable set identity and ownership metadata.
- StickerAsset: prepared/uploaded media plus format dimensions.
- MutationReceipt: request, update and reconciliation evidence.

## State Model

- Unknown -> Resolved -> Prepared -> Submitted -> Confirmed/Reconciled/Failed/Uncertain.

## Operations and Data Model

- Operations: search/get/create/add/replace/reorder/delete, save/favorite, status and reaction actions.
- Reads: set contents, custom emoji details, statuses, recent/favorite lists and reaction availability.
- Writes: account/chat/message state and Telegram-hosted set contents.
- Side effects: uploads and public/shared asset changes.
- Shapes: versioned result envelope with identifiers, completeness, freshness and next action.

## Contracts

- C001: media prerequisites finish before mutation dispatch.
- C002: uncertain mutation is reconciled before retry.
- C003: generated coverage owns every pinned-schema method exactly once.

## Invariants

- I001: upload success is never presented as set creation success.
- I002: no mutation bypasses capability/policy checks.
- I003: cleanup of disposable live assets is explicit and verified.

## Dimensions

- D001 - Asset and mutation state
  - Description: local/prepared/uploaded and pending/confirmed/uncertain; Status: partial; Values: cross-product; Boundary values: upload complete + mutation timeout; Why it matters: duplicate prevention; Related entities: StickerAsset/MutationReceipt; Related contracts: C001-C002; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Capability and ownership
  - Description: owner/non-owner, Premium/regular and permitted/denied operation; Status: partial; Values: combinations; Boundary values: uploaded asset without set ownership; Why it matters: mutation eligibility; Related entities: StickerSetRef/StickerAsset; Related contracts: C001-C003; Related invariants: I001-I002; Unknowns: Q001.

## Domain Overlays Used

- Media, stateful mutation, capability and cleanup overlays.

## Scenario Cells

- SC001 - Create custom emoji set
  - Dimensions: D001, D002; Workflow/entity anchor: StickerSetRef; Scenario: valid upload and owner capability; Expected behavior: create, wait, reread and return confirmed inventory; Related contracts: C001-C003; Related invariants: I001-I002; Why this matters: remembered user case; Status: implemented synthetic.
- SC002 - Timeout after add
  - Dimensions: D001, D002; Workflow/entity anchor: MutationReceipt; Scenario: response lost after server commit; Expected behavior: inspect set before any retry; Related contracts: C002; Related invariants: I001; Why this matters: duplicate avoidance; Status: implemented synthetic.

## Assumptions

- A001: media conversion can remain an explicit adapter around F010; support_basis: inference.

## Open Questions

- Q001: bundled converter or caller-supplied prepared assets; owner: maintainer; blocking for curated creation workflow only.

## Coverage Notes

- Kernel coverage: typed upload prerequisite, exact-plan create/add/delete, ownership checks,
  inventory reconciliation and cleanup proof implemented.
- Modeled: remaining read/status/reaction families stay generated raw/default-deny until a consumer requires review.
- Partial: live rights/Premium matrix; regular-user curated lifecycle only.
- Unknown: deployment converter choice.
- Not applicable: browser UI proof.
