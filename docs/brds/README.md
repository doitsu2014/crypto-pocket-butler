# Business Requirements Documents (BRDs)

The authoritative business requirements for **Crypto Pocket Butler** — what the business needs, not how the software is built. Consolidated 2026-11-01 from the former product brief, use-case catalogue, and domain models.

## Document set, by business domain

| Domain | BRD | Covers |
|--------|-----|--------|
| Identity & Access | [BRD-002](./BRD-002-users-roles-and-permissions.md) · [BRD-003](./BRD-003-authentication-and-session-management.md) | Actors, roles, permissions; sign-in, sessions, JWT trust |
| Account Management | [BRD-004](./BRD-004-account-management.md) | Two read-only account types — exchange and on-chain — each owning its holdings |
| Asset Catalog & Market Data | [BRD-005](./BRD-005-asset-catalog-and-market-data.md) | Canonical assets, contracts, prices |
| Portfolio Management | [BRD-006](./BRD-006-portfolio-management.md) | Portfolios, links, targets, guardrails |
| Valuation & Snapshots | [BRD-007](./BRD-007-valuation-allocation-and-snapshots.md) | Value, weights, drift, immutable history |
| Orders & Trades | [BRD-008](./BRD-008-orders.md) | Buy/sell orders from the portfolio view; trades (executions) affect allocation immediately |
| Reporting | [BRD-009](./BRD-009-reporting-and-agent-integration.md) | In-app daily/weekly briefs |
| Administration | [BRD-010](./BRD-010-admin-configuration.md) | Chain and token registry management |
| Background Jobs | [BRD-011](./BRD-011-background-jobs-and-data-refresh.md) | Scheduled catalogue, price, and EOD processing |
| Cross-cutting | [BRD-001](./BRD-001-product-vision-scope-and-success.md) · [BRD-012](./BRD-012-security-compliance-and-non-functional-requirements.md) | Vision, scope, success; security, compliance, NFRs |
| Shared terms | [GLOSSARY](./GLOSSARY.md) | Terminology, grouped by the same domains |

**Reading order:** BRD-001 (why) → 002/003 (who) → 004 → 007 (core flow) → 008/009 (differentiators) → 010–012.

## Conventions

- **IDs:** `BR-` business rule · `FR-` functional requirement · `NFR-` non-functional · `AC-` acceptance criterion. Area codes: GEN, USR, AUTH, ACC, AST, PORT, VAL, REB, REP, ADM, JOB, SEC. IDs are never reused; removed ones move to a Retired table.
- **MoSCoW:** Must (release blocker) · Should (soon, workaround exists) · Could (capacity permitting) · Won't (explicitly out of scope now).
- **Status:** all Draft. v1.1 revisions: BRD-001, 005, 008, 009, 010, 011, 012 (2026-11-02); the rest v1.0.
- **Changes:** update the document, its version, and Last updated in the same change; cross-document changes apply to all affected BRDs together.

## MVP boundary

Read-only aggregation and ordering: one exchange (OKX) plus on-chain accounts on configured EVM chains and Solana → normalised holdings → consolidated USD valuation, allocation, drift → snapshots and history → **user-initiated buy/sell orders**, whose trades affect allocation immediately → in-app daily/weekly briefs. The product itself never places trades, holds custody, files taxes, or gives regulated advice (BRD-001, BRD-012).
