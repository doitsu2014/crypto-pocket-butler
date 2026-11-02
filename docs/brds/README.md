# Business Requirements Documents (BRDs)

This folder is the authoritative home for the business requirements of **Crypto Pocket Butler** — a crypto portfolio management product for individuals and small professional teams.

The requirements were consolidated from the project's former product brief, use-case catalogue, domain models, and architecture documentation, which were removed from the repository on **2026-11-01** as part of moving to a business-requirements-only repository. They describe **what the business needs**, not how the software is built.

## Document set

| ID | Document | Business area | Primary audience |
|----|----------|---------------|------------------|
| [BRD-001](./BRD-001-product-vision-scope-and-success.md) | Product Vision, Scope & Success | Strategy, scope, success measures | Product, leadership |
| [BRD-002](./BRD-002-users-roles-and-permissions.md) | Users, Roles & Permissions | Actors and access control | Product, security |
| [BRD-003](./BRD-003-authentication-and-session-management.md) | Authentication & Session Management | Identity and sessions | Product, security |
| [BRD-004](./BRD-004-account-management.md) | Account Management | Exchange and wallet connections | Product, operations |
| [BRD-005](./BRD-005-asset-catalog-and-market-data.md) | Asset Catalog & Market Data | Assets, contracts, prices | Product, data |
| [BRD-006](./BRD-006-portfolio-management.md) | Portfolio Management | Portfolios, targets, guardrails | Product |
| [BRD-007](./BRD-007-valuation-allocation-and-snapshots.md) | Valuation, Allocation & Snapshots | Valuation, drift, history | Product, finance |
| [BRD-008](./BRD-008-rebalancing-recommendations.md) | Rebalancing Recommendations | Drift response and approvals | Product, compliance |
| [BRD-009](./BRD-009-reporting-and-agent-integration.md) | Reporting | In-app daily/weekly briefs | Product, operations |
| [BRD-010](./BRD-010-admin-configuration.md) | Admin Configuration | Chains, tokens, registries | Product, operations |
| [BRD-011](./BRD-011-background-jobs-and-data-refresh.md) | Background Jobs & Data Refresh | Scheduled processing | Product, operations |
| [BRD-012](./BRD-012-security-compliance-and-non-functional-requirements.md) | Security, Compliance & NFRs | Security, compliance, quality | Security, compliance, leadership |
| [GLOSSARY](./GLOSSARY.md) | Business Glossary | Shared terminology | Everyone |

## How to read these documents

1. Start with **BRD-001** for the problem, scope, and success measures.
2. Read **BRD-002** and **BRD-003** to understand who uses the product and how they sign in.
3. The core product flows are **BRD-004 → BRD-007**.
4. **BRD-008** and **BRD-009** cover the differentiating intelligence and reporting capabilities.
5. **BRD-010 → BRD-012** cover administration, scheduled processing, and cross-cutting obligations.

## Requirement conventions

### Identifiers

| Prefix | Meaning | Example |
|--------|---------|---------|
| `BR-xxx-nn` | Business rule | `BR-ACC-03` |
| `FR-xxx-nn` | Functional requirement | `FR-PORT-04` |
| `NFR-xxx-nn` | Non-functional requirement | `NFR-SEC-02` |
| `DR-xxx-nn` | Data requirement | `DR-SNAP-01` |
| `AC-xxx-nn` | Acceptance criterion | `AC-SYNC-02` |

`xxx` is the three-letter area code: `GEN`, `USR`, `AUTH`, `ACC`, `AST`, `PORT`, `VAL`, `REB`, `REP`, `ADM`, `JOB`, `SEC`.

### Priorities (MoSCoW)

| Priority | Meaning |
|----------|---------|
| **Must** | Required for the MVP / a release to be viable |
| **Should** | Important, expected soon after MVP; a workaround may exist |
| **Could** | Desirable; included only when capacity allows |
| **Won't** | Explicitly out of scope for the current horizon |

### Status

All documents are **Draft**. Documents revised on 2026-11-02 (BRD-001, 005, 008, 009, 010, 011, 012) are v1.1; the remainder are v1.0. They require validation by product ownership and, where noted, legal/compliance review before being treated as signed-off requirements.

## MVP boundary

The MVP consists of read-only aggregation and guidance:

- Connect **one exchange** (OKX) and **wallet accounts** for configured EVM chains and Solana.
- Normalize holdings into a single asset model.
- Show a consolidated portfolio with valuation, allocation, and drift versus target.
- Capture point-in-time snapshots and show history.
- Generate **human-approved** rebalancing recommendations.
- Produce a daily/weekly portfolio brief, readable in-app.

Anything involving trade execution, custody, tax filing, or regulated advice is explicitly out of scope for the MVP (see BRD-001 and BRD-012).

## Change control

- Every change to a requirement must update the relevant document, its version, and the `Last updated` date.
- New requirements receive the next free ID within their area; IDs are never reused.
- Removed requirements are moved to a **Retired requirements** table in the affected document rather than deleted, so history remains traceable.
- Cross-document changes (for example, a new role affecting several flows) must be applied consistently across all affected BRDs in the same change.
