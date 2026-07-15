# Feature Logic Harness: Платформенные utilities и специальные API

## Summary

- Feature ID: F020
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: не оставить «прочие» TDLib methods за пределами продукта: localization, options/config, links, proxies/network, logs, themes/backgrounds и custom/test utilities.
- Product workflow/job served: discover schema/capability -> classify platform method -> validate environment/risk -> execute raw or curated utility -> verify explicit postcondition.
- Primary ambiguity to keep explicit: test/custom/development methods доступны по schema, но production policy может их запрещать.

## Product Context

- Product context source: product.md
- Product purpose: full API parity без дыр для редких, платформенных и будущих методов.
- Primary users: platform operator, developer/QA agent and any agent using schema discovery.
- Core workflows touched: language packs/localization, countries/phone info, options/config, themes/backgrounds, links, proxies/network, logging and special/custom/test methods.
- Domain terms used: platform capability, runtime option, proxy profile, special-method policy.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: complete API and self-discovery; limits: none.
- SRC002: HARNESS.md; type: file; supports: semantic scope/default-deny; limits: planning IDs are documentation-only.
- SRC003: pinned official schema; type: supplied; supports: platform method/object/update families; limits: source alone does not prove generated registry.
- SRC004: docs/tdlib-api-coverage.md; type: file; supports: no-unclassified-method gate; limits: full registry absent.

## TDLib API Coverage

- Semantic scope: localization/language packs, countries and phone-number information, application options/config, themes/backgrounds, general link parsing/generation, proxies/network state, logging, custom requests and TDLib test/development methods.
- Planning category не является runtime fallback: каждый pinned method регистрируется по exact schema identity и получает явную risk/prerequisite/retry/capability classification.
- Unknown future methods default-deny до reviewed classification.

## Request Graph

`schema describe -> resolve platform capability/environment -> validate arguments and policy -> execute -> consume relevant update or reread option/network/proxy state -> return typed/redacted proof`.

## Completion Proof

Reads require a typed TDLib response. Setters require reread/update convergence. Logging/custom/test requests never count as successful merely because dispatch returned.

## Cache and Update Semantics

Language/config/options/background/theme caches carry version and source. Network/proxy state is update-led. Runtime/schema mismatch invalidates platform caches.

## Retry and Reconciliation

Pure reads retry boundedly. Proxy/network/option setters reconcile current state; custom/test methods follow explicit manifest policy and default to no retry.

## CLI/MCP Exposure

Everything is reachable by CLI raw call after policy validation; common utilities get curated commands. MCP exposes generic schema/call tools, not one tool per utility.

## Permissions and Account Capabilities

Classify whether method needs authorization, account capability, filesystem/network access, developer mode or official-client eligibility. Unsupported environment is explicit.

## Live Verification Boundary

No platform setting/proxy/logging mutation was performed. Only native TDJSON loading, authorization and close were exercised by the access probe.

## Scope

### In scope

- Matching pinned TDLib functions/objects/updates с explicit policy classification.

### Out of scope

- Silent fallback for unclassified future methods, arbitrary shell execution or unsafe exposure of log/custom payloads.

### Ambiguous

- The production allowlist for custom/test/development methods remains undecided; see Q001.

## Context Map

- User surfaces: schema discovery, platform inspect/set/test and raw call.
- Backend surfaces: generated registry, policy, runtime configuration and redactor.
- Data entities: PlatformCapability, RuntimeOption, ProxyProfile, SpecialMethodClass.
- External dependencies: OS/network, TDLib build flags and Telegram configuration.
- Async flows: network/proxy/options updates and language-pack synchronization.
- Config flags: production/development profile and special-method allowlist.
- Tests/examples/docs: every method classified, negative policy cases and fake network transitions.
- Observability: method class/outcome only; proxy endpoints/log payloads excluded.

## Actors and Permissions

- Read agent: discovers schema and reads safe platform data.
- Platform operator: changes proxy/network/options under scope.
- Developer/QA agent: invokes test methods only in explicit development profile.

## Domain Entities

- PlatformCapability: environment/auth/policy availability with reason.
- ProxyProfile: protected endpoint configuration and current state.
- SpecialMethodClass: risk, retry and exposure classification.

## State Model

- Unclassified -> ClassifiedDenied/ClassifiedAllowed -> Dispatched -> Converged/Failed/Uncertain.

## Operations and Data Model

- Operations: localization/config/options/theme/background/link/proxy/network/log/custom/test reads and writes.
- Reads: catalogs, settings, link information, runtime and network state.
- Writes: runtime options, proxy/network/log settings and development/test state.
- Side effects: routing changes, sensitive logs and environment-specific behavior.
- Shapes: typed result plus capability, policy and postcondition evidence.

## Contracts

- C001: no schema method remains unclassified.
- C002: future/unknown and special methods default-deny.
- C003: proxy endpoints, logs and custom payload secrets are redacted.

## Invariants

- I001: documentation category cannot hide coverage gaps behind a runtime catch-all.
- I002: runtime/schema mismatch fails before working calls.
- I003: production profile cannot invoke development/test methods without explicit allowlist.

## Dimensions

- D001 - Method class/environment/policy
  - Description: standard/special/future crossed with supported/unsupported and allow/deny; Status: partial; Values: combinations; Boundary values: newly added upstream method; Why it matters: full API with safe drift; Related entities: PlatformCapability/SpecialMethodClass; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Runtime profile
  - Description: local/server and production/development execution profile; Status: partial; Values: local-prod, local-dev, server-prod, server-dev; Boundary values: test method in production; Why it matters: exposure and network safety; Related entities: PlatformCapability/ProxyProfile; Related contracts: C002-C003; Related invariants: I002-I003; Unknowns: Q001.

## Domain Overlays Used

- Schema drift, environment, networking, secret-redaction and development-mode overlays.

## Scenario Cells

- SC001 - Upstream adds a method
  - Dimensions: D001, D002; Workflow/entity anchor: SpecialMethodClass; Scenario: schema hash changes and method has no classification; Expected behavior: generation/CI fails before release; Related contracts: C001-C002; Related invariants: I001-I002; Why this matters: honest full coverage; Status: modeled.
- SC002 - Set proxy then connection fails
  - Dimensions: D001, D002; Workflow/entity anchor: ProxyProfile; Scenario: setter responds but state/connectivity diverges; Expected behavior: reread/reconcile, return partial/failed and preserve rollback data; Related contracts: C003; Related invariants: I003; Why this matters: remote availability; Status: modeled.

## Assumptions

- A001: development-only calls can be compiled in but denied by runtime policy; support_basis: inference.

## Open Questions

- Q001: exact production allowlist and whether custom requests are compile-time gated; owner: maintainer; blocking for special-method release.

## Coverage Notes

- Kernel coverage: platform classification and safe schema drift modeled.
- Modeled: platform families and special method policy.
- Partial: full generated registry and allowlist.
- Unknown: deployment-specific network/proxy support.
- Not applicable: domain-specific financial approval.
