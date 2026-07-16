# Feature Logic Harness: Платежи и цифровые активы

## Summary

- Feature ID: F018
- Context sufficiency: sufficient
- Mode: draft
- Source priority: user request -> product_context_source -> repo sources -> assumptions
- draft_reason: source_gap
- Product context: loaded
- Product context source type: file
- product_context_source: product.md
- Feature purpose: полностью экспонировать TDLib payment/digital-asset surface при default-deny для денежных, gift, subscription и identity side effects.
- Product workflow/job served: inspect invoice/asset/status -> capability and risk check -> preview exact plan -> external approval -> submit once -> reconcile ledger/status.
- Primary ambiguity to keep explicit: raw schema availability не означает, что агенту разрешено выполнять финансовую операцию.

## Product Context

- Product context source: product.md
- Product purpose: агент может собирать данные и тестировать платежные потоки, а writes выполняются только под строгой политикой.
- Primary users: owner, finance operator, bot/Mini App QA и read-only analytics agent.
- Core workflows touched: invoices/payments/orders, bank-card info, Premium/Stars, gifts/subscriptions/giveaways/affiliate/boost purchases and Telegram Passport.
- Domain terms used: financial plan, payment form, Stars balance, gift, identity document, reconciliation ledger.
- Open product questions: Q001.

## Source Ledger

- SRC001: product.md; type: file; supports: full API/default-deny risk; limits: none.
- SRC002: HARNESS.md; type: file; supports: financial/destructive dimensions and secret boundary; limits: none.
- SRC003: pinned official schema; type: supplied; supports: payment/Stars/gift/Passport method families; limits: source alone does not prove generated registry.
- SRC004: plans.md P5/P7; type: file; supports: plan hash/idempotency/reconciliation; limits: implementation absent.
- SRC005: `crates/telegram-core/src/workflows.rs`, `apps/telegramd/src/server.rs`; type: file; supports: redacted Stars balance and exact approved Stars-invoice payment with ledger reconciliation; limits: provider/card/Passport consumers intentionally absent.

## TDLib API Coverage

- Primary owner: invoices, payment forms/results/receipts/orders, bank-card information, Premium purchases, Stars and paid features, gifts/upgrades/transfers, subscriptions, giveaways, affiliate programs and Passport/identity flows.
- Revenue/statistics reads belong to F019; channel boost administration not involving purchase belongs to F011.
- Every method remains reachable through raw CLI only after policy classification; financial writes default-deny.

## Request Graph

`resolve invoice/asset -> refresh balance/status/terms -> validate capability/currency/amount/recipient -> create immutable preview + plan hash -> obtain external approval -> dispatch once -> record pending -> query receipt/balance/status -> finalize or mark uncertain`.

## Completion Proof

A client response is insufficient for value transfer. Completion requires authoritative receipt/status or a reconciled ledger delta tied to the approved plan.

## Cache and Update Semantics

Curated Stars balance/form/ledger всегда читаются с сервера. Apply повторно строит plan по
fresh form и ещё раз читает balance перед dispatch; долгоживущий payment cache отсутствует.

## Retry and Reconciliation

Financial writes never automatically retry after dispatch uncertainty. Persist idempotency fingerprint and reconcile receipt/order/balance before any human-approved follow-up.

## CLI/MCP Exposure

Balance/preview/apply доступны через typed workflow. Curated apply поддерживает только Stars
invoice с `credentials=null`; card/order/Passport/provider secrets не имеют ordinary JSON route.

## Permissions and Account Capabilities

Check account type, Premium/Stars/region/provider availability, terms acceptance, balance, recipient and Telegram-provided eligibility.

## Live Verification Boundary

Synthetic backend покрывает balance, exact plan, lost response, ledger confirmation и
verification URL redaction. Live financial/gift/subscription/Passport operation не выполнялась.

## Scope

### In scope

- Complete pinned TDLib payment, digital-asset, subscription, giveaway, affiliate and Passport API surface.

### Out of scope

- Custody of card/identity secrets, autonomous spending, or treating a balance cache as settlement proof.

### Ambiguous

- An approved test provider/account and external approval mechanism are still required; see Q001.

## Context Map

- User surfaces: inspect, preview, approve-reference, submit and reconcile.
- Backend surfaces: policy engine, secure provider, idempotency journal and audit ledger.
- Data entities: FinancialPlan, ApprovalReceipt, PaymentRef, AssetBalance, IdentityHandle.
- External dependencies: Telegram/payment provider/identity provider.
- Async flows: verification, payment result, gift/subscription state and disputes.
- Config flags: deny/allow policy, amount caps, currency/recipient allowlists.
- Tests/examples/docs: fake provider, timeout/reconciliation and redaction tests.
- Observability: class/duration/status only; amount, identity and payment identifiers excluded by default.

## Actors and Permissions

- Read agent: inspects forms, balances and receipts within scope.
- Finance operator/owner: supplies external approval.
- Secure provider: holds payment and identity material outside model context.

## Domain Entities

- FinancialPlan: immutable operation/amount/currency/recipient/terms hash.
- ApprovalReceipt: scoped, expiring authorization from outside the agent.
- ReconciliationRecord: pending/confirmed/failed/uncertain evidence.

## State Model

- Observed -> Previewed -> Approved/Denied -> Submitted -> Pending -> Confirmed/Failed/Uncertain -> Reconciled.

## Operations and Data Model

- Operations: inspect forms/receipts/assets, validate orders, submit approved payments/transfers/actions, manage subscriptions and Passport flows.
- Reads: balances, terms, eligibility, receipts, gifts and subscription status.
- Writes: monetary/digital assets, subscriptions, identity authorization and giveaway/affiliate state.
- Side effects: irreversible value transfer and sensitive identity disclosure.
- Shapes: heavily redacted financial envelope with plan/approval/reconciliation IDs.

## Contracts

- C001: amount/currency/recipient/operation exactly match approved plan hash.
- C002: sensitive payment/identity data never enters model-visible transport.
- C003: uncertain dispatch blocks automatic retry and remains auditable.

## Invariants

- I001: financial writes default-deny.
- I002: stale balance/form cannot authorize a write.
- I003: one approval cannot be replayed for a different or repeated plan.

## Dimensions

- D001 - Value/risk/outcome
  - Description: read/preview/write crossed with no-value/value/identity and confirmed/uncertain; Status: partial; Values: combinations; Boundary values: provider timeout after submit; Why it matters: financial safety; Related entities: FinancialPlan/ApprovalReceipt/ReconciliationRecord; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.
- D002 - Approval and snapshot validity
  - Description: missing/valid/expired approval crossed with fresh/stale form or balance; Status: partial; Values: combinations; Boundary values: price or approval expires before submit; Why it matters: exact authorization; Related entities: FinancialPlan/ApprovalReceipt/AssetBalance; Related contracts: C001-C003; Related invariants: I001-I003; Unknowns: Q001.

## Domain Overlays Used

- Financial, identity-secret, approval, idempotency and reconciliation overlays.

## Scenario Cells

- SC001 - Inspect Stars balance
  - Dimensions: D001, D002; Workflow/entity anchor: AssetBalance; Scenario: read-only request; Expected behavior: fresh timestamped result, no approval; Related contracts: C002; Related invariants: I002; Why this matters: analytics; Status: implemented synthetic.
- SC002 - Payment provider timeout
  - Dimensions: D001, D002; Workflow/entity anchor: ReconciliationRecord; Scenario: approved Stars submit loses response; Expected behavior: query ledger, confirm only exact new transaction, never resubmit automatically; Related contracts: C001-C003; Related invariants: I001-I003; Why this matters: prevent double charge; Status: implemented synthetic.

## Assumptions

- A001: financial writes remain disabled in default builds/configuration; support_basis: repo_source.

## Open Questions

- Q001: which sandbox/test accounts and approval broker are authorized; owner: maintainer; blocking for write acceptance.

## Coverage Notes

- Kernel coverage: fresh Stars balance and Stars-only exact-plan payment/reconciliation implemented.
- Modeled: card/provider/gift/Premium/Passport/affiliate families stay generated raw/default-deny.
- Partial: protected providers and approved live fixtures.
- Unknown: approved financial test topology.
- Not applicable: media codec behavior.
