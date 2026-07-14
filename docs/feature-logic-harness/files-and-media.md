# Feature Logic Harness: файлы и медиа

## Summary

- Feature ID: F010
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: безопасно загружать, скачивать, генерировать и передавать media/files с прогрессом и ясной local/server семантикой.
- Product workflow/job served: source -> upload/generate -> wait updateFile -> use -> download/export/cleanup.
- Primary ambiguity to keep explicit: remote MCP не интерпретирует клиентский путь как серверный.

## Product Context

- Product context source: product.md
- Product purpose: media, sticker и Mini App workflows работают без потерянных prerequisite uploads.
- Primary users: агент, developer/operator.
- Core workflows touched: upload/download/cancel/delete/generated file/import/autosave/download manager.
- Domain terms used: file ID, remote/local file, upload token, generated file.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: local/server boundary; limits: none.
- SRC002: HARNESS.md; type: file; supports: secret/path dimensions; limits: none.
- SRC003: pinned official schema; type: supplied; supports: file/media method/update families; limits: generated mapping absent.
- SRC004: plans.md P7/P9; type: file; supports: packaging/path gates; limits: implementation absent.

## TDLib API Coverage

- Primary owner: file lookup/download/upload/cancel/delete/generation/part/preliminary upload, downloaded-file management, auto-download/autosave/import media helpers.
- Message content construction F009; sticker files F014 orchestrate through this owner.

## Request Graph

`validate source/ownership -> upload or generation handshake -> wait updateFile terminal/progress -> pass file ID -> optional download/export -> verify checksum/size`.

## Completion Proof

Upload/download completes only from terminal file state plus expected size/checksum where available; method `ok` or file ID allocation alone is not completion.

## Cache and Update Semantics

File reducer tracks local/remote/progress states from ordered `updateFile`; gap requires `getFile`/workflow resync.

## Retry and Reconciliation

Chunk/download retries are bounded/resumable. Upload timeout probes file state; generation protocol is correlated and cancellation-aware.

## CLI/MCP Exposure

CLI may use local paths. Remote MCP requires explicit upload handle/server-side artifact reference; arbitrary path passthrough is rejected.

## Permissions and Account Capabilities

File access follows Telegram/chat rights, protected content and local filesystem scopes.

## Live Verification Boundary

No transfer executed in this task; only schema/plan behavior is modeled.

## Scope

### In scope

- Full TDLib file lifecycle, progress/events, generated files, media import/export and local/server path contract.

### Out of scope

- Bypassing protected content, unrestricted server filesystem access.

### Ambiguous

- The remote artifact provider for the first server release remains undecided; see Q001.

## Context Map

- User surfaces: file/media commands/workflows.
- Backend surfaces: file reducer, transfer/generation manager, artifact store adapter.
- Data entities: FileState, Transfer, GeneratedFile, ArtifactHandle.
- External dependencies: TDLib/filesystem/storage.
- Async flows: progress/generation/download/upload/cancel.
- Config flags: roots, quotas, concurrency, cleanup.
- Tests/examples/docs: resume/gap/cancel/path traversal.
- Observability: bytes/duration/state, no filenames/content labels.

## Actors and Permissions

- Agent: scoped artifacts/paths.
- Operator: server roots/quotas.
- Core: validates ownership and protected-content constraints.

## Domain Entities

- ArtifactHandle: opaque owner-scoped reference.
- FileState: TDLib local/remote/progress state.
- TransferReceipt: fingerprint/state/result.

## State Model

- Pending -> Uploading/Downloading/Generating -> Complete/Failed/Cancelled/Uncertain.

## Operations and Data Model

- Operations: upload/download/generate/cancel/delete/import/export.
- Reads: file cache and filesystem metadata.
- Writes: files/artifacts and transfer journal.
- Side effects: network/disk usage and Telegram upload.
- Shapes: file metadata/progress/handle, never secret raw path remotely.

## Contracts

- C001: terminal file update proves completion.
- C002: remote client paths are never treated as server paths.
- C003: transfer resumes/cancels without duplicating final artifact.

## Invariants

- I001: path access is confined to scoped roots/handles.
- I002: protected content policy remains enforced.
- I003: update gap cannot leave false complete progress.

## Dimensions

- D001 - Location/state
  - Description: local/server/remote handle and transfer states; Status: partial; Values: combinations; Boundary values: reconnect/cancel/path mismatch; Why it matters: security/completion; Related entities: ArtifactHandle/FileState; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Update continuity
  - Description: complete, delayed or gapped `updateFile` stream; Status: filled; Values: complete, delayed, gapped; Boundary values: gap after upload dispatch; Why it matters: terminal proof; Related entities: FileState/TransferReceipt; Related contracts: C001-C003; Related invariants: I002-I003; Unknowns: none.

## Domain Overlays Used

- Filesystem/async transfer: path trust, progress and resumability.

## Scenario Cells

- SC001 - Server MCP receives local path
  - Dimensions: D001, D002; Workflow/entity anchor: upload; Scenario: client sends `/tmp/a.png`; Expected behavior: reject or require upload handle; Related contracts: C002; Related invariants: I001; Why this matters: trust boundary; Status: modeled.
- SC002 - updateFile lost
  - Dimensions: D001, D002; Workflow/entity anchor: transfer; Scenario: gap during upload; Expected behavior: partial + getFile/resync; Related contracts: C001; Related invariants: I003; Why this matters: completion honesty; Status: modeled.

## Assumptions

- A001: remote release includes an owner-scoped artifact adapter; support_basis: inference.

## Open Questions

- Q001: выбрать server artifact implementation; owner: maintainer; blocking for remote file workflows only.

## Coverage Notes

- Kernel coverage: state/path/resume modeled.
- Modeled: file lifecycle and cross-surface semantics.
- Partial: artifact backend and exact schema mapping.
- Unknown: server quota defaults.
- Not applicable: Telegram account authorization.
