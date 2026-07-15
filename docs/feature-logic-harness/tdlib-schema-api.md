# Feature Logic Harness: полная TDLib-схема и generic API

## Summary

- Feature ID: F003
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: missing_contracts
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: сделать всю закреплённую TDLib-схему discoverable, валидируемой и вызываемой без ручного wrapper на каждый method.
- Product workflow/job served: version -> search/describe -> policy -> call -> typed/raw result.
- Primary ambiguity to keep explicit: schema pin и macOS arm64 artifact приняты, но Linux x86_64 artifact и generated registry ещё отсутствуют.

## Product Context

- Product context source: product.md
- Product purpose: named examples не ограничивают весь TDLib functionality.
- Primary users: агент и разработчик интеграции.
- Core workflows touched: capability discovery и universal call.
- Domain terms used: pinned schema, raw registry, feature owner, coverage level.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: full-scope rule; limits: none.
- SRC002: HARNESS.md; type: file; supports: owner/classification gates; limits: none.
- SRC003: docs/tdlib-api-coverage.md; type: file; supports: baseline/counts/formula; limits: generator absent.
- SRC004: `vendor/tdlib/manifest.json`, exact official commit `07d3a097...` и raw digest `.memory/raw/2026-07-15-tdlib-1.8.66-schema-pin.md`; type: verified repo source; supports: 1010 functions/2168 definitions/184 updates/13 auth states; limits: generated registry absent.
- SRC005: `vendor/tdlib/native-build-policy.json`, `vendor/tdlib/native-builds/aarch64-apple-darwin.json` и `.memory/raw/2026-07-15-tdlib-1.8.66-native-macos-arm64-reviewed-rebuild.md`; type: verified repo source; supports: exact crash-safe macOS arm64 artifact identity, runtime version/commit and bounded recovery contract; limits: resource thresholds sampled, Linux artifact и bit-for-bit reproducibility не доказаны.

## TDLib API Coverage

- This feature owns schema ingestion, not every method; generated manifest assigns each method exactly once across F001–F022.
- Levels: L0 codec, L1 surface, L2 semantics/policy, L3 workflow, L4 live proof.
- Product may claim full pinned API only after L0–L2 for every method/update/type; L3/L4 are reported separately.

## Request Graph

`read pinned tl -> parse -> normalize signatures -> classify owner/risk/retry/surfaces -> generate registry/docs/tests -> runtime hash handshake -> call`.

## Completion Proof

Coverage complete when unowned, duplicate-owner, unclassified and schema-drift counts are zero and registry counts equal pinned schema counts.

## Cache and Update Semantics

Unknown constructors/fields survive raw round-trip. Unknown update is routed as raw and may make dependent state incomplete until classified/reduced.

## Retry and Reconciliation

Registry classifies retry but never performs it itself. Schema refresh is explicit and reviewable; automatic master following is forbidden.

## CLI/MCP Exposure

CLI: `version`, `schema search/describe`, `td call`. MCP: small schema/call tools over same registry, not tool-per-method.

## Permissions and Account Capabilities

Registry records user/bot/business/premium/admin/official-only constraints separately from policy risk.

## Live Verification Boundary

Current counts/hash проверены against upstream и vendored offline; macOS arm64 artifact проверен по hash/Mach-O/dependencies/exports/runtime version+commit. Linux artifact, generated registry и live per-method matrix отсутствуют.

## Scope

### In scope

- Pinning, parsing, code generation, raw validation, discovery, ownership, compatibility and coverage reporting.

### Out of scope

- Pretending raw serialization equals correct stateful workflow or live permission proof.

### Ambiguous

- Linux x86_64 artifact identity и target-specific build provenance остаются незакрытой частью P0.

## Context Map

- User surfaces: CLI/MCP discovery and generic call.
- Backend surfaces: schema parser/generator/runtime registry.
- Data entities: SchemaManifest, MethodDescriptor, TypeDescriptor, CoverageRecord.
- External dependencies: official TDLib repository/native library.
- Async flows: none at generation; runtime call delegated to core.
- Config flags: pinned commit/hash and compatibility policy.
- Tests/examples/docs: generated round-trip/coverage reports.
- Observability: version mismatch and unsupported capability counters.

## Actors and Permissions

- Maintainer: pins/triages schema diff.
- Агент: discovers/calls only permitted descriptors.
- CI: rejects drift/unclassified symbols.

## Domain Entities

- SchemaManifest: commit/version/hash/counts.
- CoverageRecord: method owner, risk, retry, prerequisites, surfaces, tests.
- CapabilityConstraint: account/rights/platform requirements.

## State Model

- Candidate -> Reviewed -> Pinned -> Generated -> RuntimeMatched.
- Drifted/Unclassified are blocking states.

## Operations and Data Model

- Operations: pin, diff, generate, validate, describe, call.
- Reads: schema and manifest.
- Writes: generated registry/report/tests.
- Side effects: none until call dispatch.
- Input and output shapes: JSON schema descriptors and protocol envelopes.

## Contracts

- C001: every pinned symbol is losslessly represented.
- C002: every function has one owner and complete security/retry classification.
- C003: runtime/native mismatch blocks normal calls.

## Invariants

- I001: no manually maintained 1000-method Markdown list is authoritative.
- I002: method absence due to rights is a capability result, not missing API.
- I003: raw call always passes policy.

## Dimensions

- D001 - Coverage level
  - Description: L0–L4; Status: filled; Values: codec/surface/semantics/workflow/live; Boundary values: L2 vs L3; Why it matters: claim honesty; Related entities: CoverageRecord; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: none.
- D002 - Schema state
  - Description: matched/drifted/unclassified; Status: filled; Values: three; Boundary values: changed signature; Why it matters: startup/CI gate; Related entities: SchemaManifest; Related contracts: C002/C003; Related invariants: I001; Unknowns: Q001.

## Domain Overlays Used

- Compatibility/security: generated coverage and explicit triage.

## Scenario Cells

- SC001 - Новый upstream method
  - Dimensions: D001, D002; Workflow/entity anchor: schema diff; Scenario: added method without owner; Expected behavior: CI fails until classification; Related contracts: C002; Related invariants: I001-I003; Why this matters: prevents silent gap; Status: modeled.
- SC002 - Rights-limited method
  - Dimensions: D001, D002; Workflow/entity anchor: call; Scenario: regular account invokes bot-only method; Expected behavior: structured requires_bot/capability error; Related contracts: C001; Related invariants: I002; Why this matters: full API without false success; Status: modeled.

## Assumptions

- A001: exact upstream commit remains retrievable for bounded repeat builds; bit-for-bit reproducibility после одной сборки не заявляется; support_basis: repo_source.

## Open Questions

- Q001: resolved by `D-20260715-003`; initial production schema pin — exact commit `07d3a0973f5113b0827a04d54a93aaaa9e288348`, никогда moving `master`.

## Coverage Notes

- Kernel coverage: complete contract, no implementation.
- Modeled: pin/diff/classify/discover/call claims.
- Partial: macOS arm64 native proof complete; Linux artifact и generated registry отсутствуют.
- Unknown: none for schema source identity.
- Not applicable: domain-specific request chains.
