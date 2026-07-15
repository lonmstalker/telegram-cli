# Feature Logic Harness: Mini Apps и Web Apps

## Summary

- Feature ID: F013
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: полностью поддержать Telegram-side Web App/Mini App lifecycle и безопасно передать запуск браузерному harness.
- Product workflow/job served: resolve bot/button -> get/open URL -> redacted handoff -> browser assertions -> close/cleanup.
- Primary ambiguity to keep explicit: TDLib launch success не доказывает UI success.

## Product Context

- Product context source: product.md
- Product purpose: агент тестирует Mini Apps account-side и browser-side без утечки init data.
- Primary users: developer/QA agent и owner.
- Core workflows touched: Web App URL/open/close/data, attachment menu, login/OAuth/internal/external links, browser bridge.
- Domain terms used: launch artifact, init data, browser handoff, bridge.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: Mini App boundary; limits: none.
- SRC002: HARNESS.md; type: file; supports: secret/browser invariants; limits: none.
- SRC003: pinned official schema; type: supplied; supports: Web App/OAuth/link method families; limits: full generated registry absent.
- SRC004: plans.md P4/P7/P10; type: file; supports: workflow/browser gate; limits: implementation absent.

## TDLib API Coverage

- Primary owner: get/open/close Web App, Web App links/data, attachment-menu/prepared buttons, login/OAuth/deep/internal/external link handling used by Mini Apps.
- Bot trigger F012; generic file/network/browser utilities F010/F020.

## Request Graph

`resolve/preload bot -> locate button/menu/direct URL -> get/open Web App -> create mode-0600 redacted launch artifact -> browser run -> collect sanitized artifacts -> closeWebApp -> cleanup`.

## Completion Proof

Telegram-side completion and browser-side completion are separate. UI pass requires explicit DOM/bridge/network/assertion evidence; TDLib URL alone is partial.

## Cache and Update Semantics

Bot/user/chat prerequisites use shared cache; launch lifecycle records update/gap state. Browser events are separate ordered artifacts.

## Retry and Reconciliation

Resolve/get URL reads bounded-retry. Open/close timeout reconciles launch state; never regenerate/replay secret artifact blindly.

## CLI/MCP Exposure

CLI may emit local protected artifact. Remote MCP returns brokered artifact handle/one-time handoff, never raw init data in transcript.

## Permissions and Account Capabilities

Bot/Web App/attachment-menu/OAuth availability and user consent are checked; browser cannot elevate Telegram permissions.

## Live Verification Boundary

No Mini App opened. Existing tg-analytics Playwright runner is a reuse candidate, not current project evidence.

## Scope

### In scope

- Full Web App/Mini App/link protocol, secure launch artifacts, browser handoff and cleanup.

### Out of scope

- Treating URL retrieval as visual proof; persisting raw init data; public artifact URLs.

### Ambiguous

- The first remote artifact handoff mechanism remains undecided; see Q001.

## Context Map

- User surfaces: Mini App workflow and browser report.
- Backend surfaces: TDLib workflow, artifact broker, browser runner.
- Data entities: LaunchArtifact, BrowserRun, TelegramLaunchState.
- External dependencies: bot/Mini App/browser/network.
- Async flows: launch, page readiness, bridge events, close.
- Config flags: viewport/mode/deadlines/artifact retention.
- Tests/examples/docs: synthetic bridge, live disposable Mini App, redaction gates.
- Observability: safe status/duration only; URL fragment/init data excluded.

## Actors and Permissions

- QA agent: requests test and reads sanitized result.
- Owner: approves Telegram interaction where required.
- Artifact broker/browser: secrets isolated from model.

## Domain Entities

- LaunchArtifact: protected one-time data with TTL/owner.
- BrowserRun: assertions/artifacts/cleanup state.
- TelegramLaunchState: URL/open/close proof.

## State Model

- Planned -> TelegramPrepared -> BrowserRunning -> Passed/Failed/Partial -> Closed/Cleaned.

## Operations and Data Model

- Operations: resolve/get/open/handoff/run/assert/close/cleanup.
- Reads: bot/menu/link and sanitized browser state.
- Writes: protected artifact and test artifacts.
- Side effects: Web App open/close and browser network activity.
- Shapes: redacted summary plus protected handle.

## Contracts

- C001: raw init data never enters model-visible output.
- C002: Telegram and browser proofs are reported separately.
- C003: artifacts have owner/TTL/exact cleanup.

## Invariants

- I001: TDLib does not claim DOM/UI success.
- I002: remote path/handle cannot expose host filesystem.
- I003: repeated run uses fresh launch artifact.

## Dimensions

- D001 - Surface/proof
  - Description: local/remote and Telegram/browser status; Status: partial; Values: combinations; Boundary values: Telegram success + browser fail; Why it matters: claim/security; Related entities: LaunchArtifact/BrowserRun; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Artifact trust boundary
  - Description: local protected artifact, remote brokered handle or leaked raw data; Status: partial; Values: local, brokered, leaked; Boundary values: remote launch needs init data; Why it matters: secret isolation; Related entities: LaunchArtifact; Related contracts: C001-C003; Related invariants: I002-I003; Unknowns: Q001.

## Domain Overlays Used

- Browser/security: two-system proof and secret-bearing handoff.

## Scenario Cells

- SC001 - Telegram URL, browser JS error
  - Dimensions: D001, D002; Workflow/entity anchor: BrowserRun; Scenario: open succeeds, app crashes; Expected behavior: Telegram complete/browser failed/overall failed; Related contracts: C002; Related invariants: I001; Why this matters: honest test; Status: modeled.
- SC002 - Remote MCP launch
  - Dimensions: D001, D002; Workflow/entity anchor: LaunchArtifact; Scenario: remote agent requests URL; Expected behavior: opaque brokered handle, no raw init data; Related contracts: C001/C003; Related invariants: I002; Why this matters: secret isolation; Status: modeled.

## Assumptions

- A001: existing browser harness can be extracted without Telegram secrets; support_basis: repo_source.

## Open Questions

- Q001: one-time local proxy, SSH transfer или server-side browser для remote handoff; owner: maintainer; blocking for remote Mini App tests only.

## Coverage Notes

- Kernel coverage: Telegram/browser/security boundary modeled.
- Modeled: end-to-end test phases.
- Partial: remote handoff and exact schema classification.
- Unknown: deployment-specific browser topology.
- Not applicable: general chat analytics.
