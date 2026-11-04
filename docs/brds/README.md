# Business Requirements Documents (BRDs)

The authoritative business requirements for **Crypto Pocket Butler** — what the business needs, not how the software is built. Consolidated 2026-11-01; extended 2026-11-04 for **global mass-market retail**.

## Document set, by business domain

| Domain | BRD | Covers |
|--------|-----|--------|
| Identity & Access | [BRD-002](./BRD-002-users-roles-and-permissions.md) · [BRD-003](./BRD-003-authentication-and-session-management.md) | Actors, roles, permissions, support access; sign-in, sessions, MFA, devices, JWT trust |
| Account Management | [BRD-004](./BRD-004-account-management.md) | Exchange, on-chain, WalletConnect, xpub, and CSV sources; coverage matrix; key verification; quotas |
| Asset Catalog & Market Data | [BRD-005](./BRD-005-asset-catalog-and-market-data.md) | Canonical assets, contracts, multi-source prices, FX, de-peg, licensing |
| Portfolio Management | [BRD-006](./BRD-006-portfolio-management.md) | Portfolios, shared-account membership, targets, guardrails, templates |
| Valuation & Snapshots | [BRD-007](./BRD-007-valuation-allocation-and-snapshots.md) | Base-currency value, weights, drift, immutable history, attribution |
| Orders & Trades | [BRD-008](./BRD-008-orders.md) | Order intents, trades, immediate allocation effect, reconciliation |
| Reporting | [BRD-009](./BRD-009-reporting-and-agent-integration.md) | In-app, email, and PDF briefs, localized and timezone-aware |
| Transactions & P&L | [BRD-013](./BRD-013-transactions-cost-basis-and-pnl.md) | Ledger, cost basis, net invested, realized/unrealized P&L, tax-ready export |
| Onboarding | [BRD-014](./BRD-014-onboarding-education-and-first-run.md) | First-run paths, demo, education, activation |
| Notifications | [BRD-015](./BRD-015-notifications-and-alert-preferences.md) | In-app, email, push; alert types, preferences, delivery |
| Billing | [BRD-016](./BRD-016-billing-entitlements-and-quotas.md) | Plans, entitlements, quotas, global subscriptions, tax |
| Privacy & Data Rights | [BRD-017](./BRD-017-privacy-data-rights-and-export.md) | Data map, retention, consent, export, deletion, subprocessors |
| Localization & Accessibility | [BRD-018](./BRD-018-localization-multi-currency-and-accessibility.md) | Locale, base currency, FX display, timezone, WCAG 2.2 AA |
| Support & Status | [BRD-019](./BRD-019-support-service-status-and-incident-communication.md) | Help centre, tickets, SLAs, status page, incidents |
| Performance & Benchmarks | [BRD-020](./BRD-020-performance-benchmarks-and-attribution.md) | TWR, MWR/IRR, benchmarks, contribution, attribution |
| Analytics & Experimentation | [BRD-021](./BRD-021-product-analytics-experimentation-and-measurement.md) | Metric tree, event taxonomy, consent, experiments |
| Administration | [BRD-010](./BRD-010-admin-configuration.md) | Chain, token, source, user, entitlement, and flag administration |
| Background Jobs | [BRD-011](./BRD-011-background-jobs-and-data-refresh.md) | Catalog, price, FX, sync, snapshots, notifications, billing, retention |
| Cross-cutting | [BRD-001](./BRD-001-product-vision-scope-and-success.md) · [BRD-012](./BRD-012-security-compliance-and-non-functional-requirements.md) | Vision, scope, success; security, compliance, NFRs |
| Shared terms | [GLOSSARY](./GLOSSARY.md) | Terminology, grouped by the same domains |
| Decisions | [DECISIONS.md](./DECISIONS.md) | Resolved cross-BRD decisions and rationale |
| Traceability | [TRACEABILITY.md](./TRACEABILITY.md) | Goals → BRDs → requirements |
| Readiness | [GLOBAL-READINESS.md](./GLOBAL-READINESS.md) | Global retail scorecard and remaining gaps |

**Reading order:** BRD-001 (why) → 002/003 (who) → 004/005 (sources and data) → 006/007 (core model) → 013 (P&L) → 008 (action) → 009/015 (reporting and alerts) → 020 (performance) → 014/016/017/018/019/021 (global operations) → 010–012.

## Conventions

- **IDs:** `BR-` business rule · `FR-` functional requirement · `NFR-` non-functional · `AC-` acceptance criterion. Area codes: GEN, USR, AUTH, ACC, AST, PORT, VAL, ORD, REP, ADM, JOB, SEC, PRV, LEG, TXN, ONB, NOT, BIL, L10N, SUP, PERF, ANL. IDs are never reused; removed ones move to a Retired table.
- **MoSCoW:** Must (release blocker) · Should (soon, workaround exists) · Could (capacity permitting) · Won't (explicitly out of scope now).
- **Status:** all Draft. Revised 2026-11-04: BRD-001–012 (see each document's version); new BRDs 013–021 v1.0.
- **Changes:** update the document, its version, and Last updated in the same change; cross-document changes apply to all affected BRDs together.
- **Decisions:** resolved open questions move to [DECISIONS.md](./DECISIONS.md); the OQ table keeps only what is still open.

## MVP boundary (global retail)

Read-only aggregation and ordering across **at least five exchanges and ten chains** by region → normalised holdings → base-currency valuation, allocation, drift → transaction ledger, cost basis, and P&L → snapshots and history → **user-initiated buy/sell order intents**, whose trades affect allocation immediately → in-app, email, and PDF briefs → notifications → performance and benchmarks → onboarding, billing, privacy, localization, support, and analytics.

The product itself never places trades in the MVP, holds custody, files taxes, or gives regulated advice (BRD-001, BRD-012). Post-MVP: direct execution, DeFi/staking position accounting, NFTs, all-asset aggregation, native apps, multi-user/advisor, tax filing, KYC/AML.

## Global readiness

The 2026-11-04 review measured the set at roughly 4/10 for global mass-market product coverage and 7.5/10 for engineering specification quality. This revision adds the missing domains and targets **8/10 specification coverage**. Details and remaining implementation dependencies are in [GLOBAL-READINESS.md](./GLOBAL-READINESS.md).
