# Feature Logic Harness: Статистика и ресурсы

## Summary

- Feature ID: F019
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: получать полную и актуальную Telegram statistics/revenue/resource telemetry с корректной загрузкой async graphs и явной свежестью.
- Product workflow/job served: resolve entity -> check stats capability -> request overview -> recursively load graph tokens/pages -> mark coverage/freshness -> return digest and raw data.
- Primary ambiguity to keep explicit: Telegram statistics are server snapshots and may lag; completion не означает real-time truth.

## Product Context

- Product context source: product.md
- Product purpose: основной агентный use case — сбор статистики по чатам/каналам без ложного `not_found` и неполных графиков.
- Primary users: analyst agent, channel owner/operator and platform operator.
- Core workflows touched: chat/message/story/revenue statistics, async statistical graphs, storage/database/network/download statistics and resource optimization.
- Domain terms used: stats capability, async graph token, coverage window, freshness, resource snapshot.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: primary analytics job and honest freshness; limits: none.
- SRC002: HARNESS.md; type: file; supports: request-chain/completion dimensions; limits: none.
- SRC003: pinned official schema; type: supplied; supports: statistics/resource method families; limits: source alone does not prove generated registry.
- SRC004: plans.md P4/P7/P10; type: file; supports: graph tokens, terminal rules and live acceptance; limits: implementation absent.

## TDLib API Coverage

- Primary owner: chat/message/story/revenue statistics, statistical graphs and async graph loading; storage/database/network/download statistics and related resource-management operations.
- Entity resolution/history belongs to F008/F009; channel rights to F011; file transfers to F010.
- Generated registry classifies every matching pinned-schema function, object and update by exact schema identity.

## Request Graph

`resolve chat/channel/message/story -> load full info/capability -> request statistics for explicit range -> traverse all async graph tokens and pagination -> detect no-progress/terminal condition -> attach observed_at/source/window -> optionally optimize resources -> reread resource state`.

## Completion Proof

Statistics are complete only when every required graph is data/error (not async), requested pages/ranges terminate by method rule, and unresolved tokens are reported. Freshness and coverage window are mandatory.

## Cache and Update Semantics

Statistics cache keys include entity, date range, filters and schema version. Domain updates may invalidate current windows; server observation time is distinct from event time.

## Retry and Reconciliation

Overview/graph reads use bounded retry respecting FLOOD_WAIT. Repeated async token with no progress terminates partial, not infinite loops. Resource optimization is reconciled by new statistics.

## CLI/MCP Exposure

Human output is a short digest; JSON/JSONL retains graph/data provenance and completeness. Large raw series stream or write to an explicit artifact handle rather than filling agent context.

## Permissions and Account Capabilities

Check `can_get_statistics`/method-specific availability, channel role, time range and revenue entitlement. A capability denial is distinct from no data.

## Live Verification Boundary

Account access is proven, but no channel/statistics query has yet been executed in this project. P10 needs owned/test channels with known ranges.

## Scope

### In scope

- Full pinned-schema statistics, revenue and local resource-statistics/optimization surface.

### Out of scope

- Inventing missing metrics, warehouse/BI implementation and claiming real-time freshness without evidence.

### Ambiguous

- Initial export formats and the maximum inline series size remain undecided; see Q001.

## Context Map

- User surfaces: stats digest, structured export, graph watch and resource report.
- Backend surfaces: resolver, graph walker, cache, exporter and scheduler.
- Data entities: StatisticsQuery, GraphNode, CoverageProof, ResourceSnapshot.
- External dependencies: Telegram statistics backend and local TDLib database/files.
- Async flows: graph token loading, scheduled refresh and resource optimization.
- Config flags: range limits, cache TTL, output threshold and scheduled jobs.
- Tests/examples/docs: synthetic token graphs, permission/no-data cases and owned-channel live fixture.
- Observability: workflow/graph counts and latency; chat/channel IDs excluded from metric labels.

## Actors and Permissions

- Analyst agent: executes read-only collection/export.
- Channel owner/operator: supplies access and approves scheduled scope.
- Platform operator: runs local resource optimization.

## Domain Entities

- StatisticsQuery: entity/range/filter/capability context.
- GraphNode: data/async/error graph state with token lineage.
- CoverageProof: requested vs resolved windows/graphs/pages and freshness.
- ResourceSnapshot: storage/network/database/download statistics.

## State Model

- Planned -> Resolving -> LoadingOverview -> LoadingGraphs/Pages -> Complete/Partial/Denied/Failed.
- Resource change: Observed -> Optimizing -> Reconciled/Uncertain/Failed.

## Operations and Data Model

- Operations: request statistics/revenue, load graphs, export series, inspect/optimize resource usage.
- Reads: aggregates, graph series, revenue and local resource counters.
- Writes: optional cache/storage optimization only.
- Side effects: Telegram load, disk cleanup and potentially expensive scans.
- Shapes: digest plus typed datasets and CoverageProof.

## Contracts

- C001: unresolved async graphs keep result partial.
- C002: every result states range, source, freshness and capability outcome.
- C003: empty data is distinguished from denied, stale, incomplete and not found.

## Invariants

- I001: a short/empty first response is not automatically terminal.
- I002: graph token traversal has deadline and no-progress guard.
- I003: resource optimization never runs implicitly as part of a read.

## Dimensions

- D001 - Capability/coverage/freshness
  - Description: allowed/denied crossed with complete/partial and fresh/stale; Status: partial; Values: combinations; Boundary values: overview with unresolved async token; Why it matters: honest analytics; Related entities: StatisticsQuery/GraphNode/CoverageProof; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Graph terminal state
  - Description: data, async token, error, repeated token or deadline; Status: filled; Values: data, async, error, no-progress, timeout; Boundary values: repeated unresolved token; Why it matters: traversal termination; Related entities: GraphNode/CoverageProof; Related contracts: C001-C003; Related invariants: I001-I002; Unknowns: none.

## Domain Overlays Used

- Analytics freshness, async graph, pagination, large-output and resource overlays.

## Scenario Cells

- SC001 - Channel graph is async
  - Dimensions: D001, D002; Workflow/entity anchor: GraphNode; Scenario: overview returns token; Expected behavior: load token until data/error/deadline and expose lineage; Related contracts: C001-C002; Related invariants: I001-I002; Why this matters: core user complaint; Status: modeled.
- SC002 - Statistics unavailable
  - Dimensions: D001, D002; Workflow/entity anchor: StatisticsQuery; Scenario: channel resolves but account lacks rights; Expected behavior: capability_denied, never not_found; Related contracts: C002-C003; Related invariants: I001; Why this matters: diagnosability; Status: modeled.

## Assumptions

- A001: large datasets can be materialized as protected local artifacts with compact summaries; support_basis: repo_source.

## Open Questions

- Q001: JSONL/CSV/Arrow priority and inline threshold; owner: maintainer; blocking for polished exports, not raw API.

## Coverage Notes

- Kernel coverage: request chains, graphs, freshness and resource effects modeled.
- Modeled: principal statistics/resource families.
- Partial: exact schema mapping and live fixtures.
- Unknown: final export UX.
- Not applicable: browser rendering.
