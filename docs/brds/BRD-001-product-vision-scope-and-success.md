# BRD-001 — Product Vision, Scope & Success

| Field | Value |
|-------|-------|
| Document ID | BRD-001 |
| Version | 1.1 |
| Status | Draft — requires product sign-off |
| Owner | Product |
| Last updated | 2026-11-02 |
| Related documents | BRD-002 … BRD-012 |

## 1. Purpose

This document defines why Crypto Pocket Butler exists, who it serves, what the MVP must achieve, what is deliberately excluded, and how success is measured. It is the anchor for all other BRDs: any requirement that does not trace to a goal or scope statement here should be challenged.

## 2. Problem statement

Crypto investors hold assets across many venues: centralized exchanges, multiple EVM chains, and Solana. Each venue reports balances differently, in different units, and at different times. As a result:

- Investors cannot see a single, trustworthy consolidated position.
- Allocation drift versus an intended strategy is invisible until it is large.
- Rebalancing decisions are manual, inconsistent, and poorly documented.
- Reporting to family members, clients, or advisors is assembled by hand.
- Sensitive exchange credentials are often over-permissioned because read-only setups are inconvenient.

Crypto Pocket Butler addresses this by being a **read-only aggregation and decision-support layer**: it never holds private keys or withdrawal rights, and it never executes trades without explicit human approval.

## 3. Vision

> A unified, trustworthy view of a crypto portfolio across wallets and exchanges, with intelligent rebalancing guidance, built-in guardrails, and effortless reporting.

## 4. Target users

| Persona | Description | Key needs |
|---------|-------------|-----------|
| **Retail investor** | Individual with 3–10 wallets across chains | Simple consolidated view, allocation tracking, understandable guidance |
| **Active trader** | Frequent trader with multiple strategies | Fresh data, drift alerts, fast refresh, multiple portfolios |
| **Family office** | Managing wealth for a family | Multi-user access, reporting, compliance, audit trails |
| **Fund manager** | Running a crypto fund with client reporting | Performance analytics, white-label reporting, client portals |

The **MVP is optimized for the retail investor and the active trader**. Family-office and fund-manager capabilities are post-MVP.

## 5. Product goals

| ID | Goal | Measure of achievement |
|----|------|------------------------|
| G-01 | Consolidate holdings across connected venues | A user can see every configured account and its holdings in one portfolio view |
| G-02 | Provide a trustworthy USD valuation | Valuation reconciles with manual spot-checking to >99% |
| G-03 | Make allocation drift visible | Every portfolio with a target allocation displays drift per asset |
| G-04 | Turn drift into an actionable, safe recommendation | A recommendation lists proposed trades, rationale, risk, and guardrail status |
| G-05 | Remove manual reporting effort | A daily/weekly brief is available in-app without manual assembly |
| G-06 | Protect user credentials | Exchange credentials are encrypted at rest, never returned by the API, and read-only |
| G-07 | Preserve history | Point-in-time snapshots are immutable and available for trend review |

## 6. Scope

### 6.1 In scope — MVP

| Capability | Summary | BRD |
|------------|---------|-----|
| Identity | Supabase Auth single sign-on (email/password, magic link, OAuth, MFA), JWT-protected APIs, per-user data isolation | BRD-002, BRD-003 |
| Account connections | OKX exchange accounts (read-only API keys) and wallet accounts (public addresses) | BRD-004 |
| Wallet networks | Configured EVM chains and Solana | BRD-004, BRD-010 |
| Asset catalogue | Asset identity, contract mappings, decimals, prices | BRD-005 |
| Portfolio management | Multiple portfolios per user, linked accounts, target allocation, guardrails | BRD-006 |
| Valuation and allocation | Aggregation, USD valuation, weights, drift versus target | BRD-007 |
| Snapshots | Manual and end-of-day (EOD) snapshots, immutable history, trend view | BRD-007 |
| Recommendations | Drift-based rebalancing suggestions requiring human accept/reject | BRD-008 |
| Reporting | Daily/weekly brief composed in-app and readable in the SPA | BRD-009 |
| Administration | Chain and token registry management | BRD-010 |
| Scheduled processing | Coin catalogue refresh, price collection, EOD snapshots | BRD-011 |

### 6.2 Post-MVP (roadmap)

| Capability | Description | Indicative horizon |
|------------|-------------|--------------------|
| Investment insights | IRR, CAGR, Sharpe/Sortino, drawdown, attribution, risk analytics, benchmark comparison | Q4 2026 |
| Tax and compliance | Cost-basis methods (FIFO/LIFO/HIFO), capital-gains reports, transaction export, sanctioned-address screening, geographic restrictions | Q4 2026 – Q1 2027 |
| Multi-user portfolios | Owner/Admin/Analyst/Viewer roles, shared portfolios, audit logs | Q3 2026 |
| Mobile applications | iOS and Android clients | Q3 2026 |
| Trade execution | DEX aggregator integration with allowlists, maximum order sizes, and full audit trail | Post-MVP |
| Institutional features | White-label, client portals, developer API, SLA | Q1 2027 |

> Roadmap horizons are planning intent, not commitments. They must be re-baselined before being used for delivery planning.

### 6.3 Out of scope

- Custody of assets, storage of private keys or seed phrases.
- Automated or one-click trade execution in the MVP.
- Fiat on/off-ramp, payments, or custody of fiat.
- Tax filing or regulated tax advice.
- Investment advice in a regulated sense; recommendations are decision support requiring human judgement.
- Futures, margin, or derivatives position management in the MVP.
- Transactions or transfers initiated by the product.

## 7. Functional requirements (product level)

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-GEN-01 | A signed-in user must be able to add multiple accounts and combine them into portfolios. | Must |
| FR-GEN-02 | The product must present a single consolidated view per portfolio in USD. | Must |
| FR-GEN-03 | The product must compare actual allocation against a user-defined target allocation. | Must |
| FR-GEN-04 | The product must generate rebalancing recommendations that the user can accept or reject; it must never execute trades automatically. | Must |
| FR-GEN-05 | The product must record immutable point-in-time snapshots and display their history. | Must |
| FR-GEN-06 | The product must generate a scheduled portfolio brief, readable in-app. | Should |
| FR-GEN-07 | The product must support more than one portfolio per user. | Must |
| FR-GEN-08 | The product must expose a documented API for the web client and future integrations. | Should |
| FR-GEN-09 | The product should notify users when allocation drifts beyond configured guardrails. | Should |
| FR-GEN-10 | The product must not require exchange withdrawal permissions at any point. | Must |

## 8. Business model and packaging (draft)

| Tier | Price | Included capabilities |
|------|-------|-----------------------|
| **Free** | $0 | 3 wallets, basic tracking, 24-hour refresh |
| **Pro** | $15/month | Unlimited wallets, real-time refresh, insights |
| **Team** | $50/month | Multi-user, audit logs, reports |
| **Enterprise** | Custom | White-label, API access, SLA |

> Packaging is a draft hypothesis and is not a validated commercial commitment. Tier enforcement is post-MVP.

## 9. Success metrics

| Metric | Target | Measurement method |
|--------|--------|--------------------|
| Portfolio accuracy | >99% | Compare product valuation with a manual calculation on a sample of portfolios |
| Price data freshness | <30 seconds for price updates | End-to-end timestamp measurement |
| Supported wallets | 5+ types | MetaMask, WalletConnect, Ledger, and other read-only address sources |
| Supported chains | 10+ | EVM chains plus Solana |
| Time to first value | <2 minutes | From account creation to first portfolio view |
| Recommendation acceptance rate | Baseline in first quarter, then improve | Accepted ÷ presented recommendations |
| Reporting reliability | 100% of scheduled briefs generated or explicitly failed | Job run log and brief generation record |

## 10. Key assumptions and constraints

- **Read-only by design.** Exchange integrations use read-only API keys; the product never asks for withdrawal or trading permissions in the MVP.
- **Human-in-the-loop.** Recommendations are advisory; execution is manual and outside the product.
- **Exchange data dependency.** Prices and market metadata depend on the OKX exchange API; outages must degrade gracefully using last-known stored values, and assets without a supported exchange pair are unpriced.
- **Public wallet addresses only.** Wallet connections require no private keys.
- **Base currency is USD.** Multi-currency reporting is post-MVP.
- **Single-tenant personal deployment initially.** The architecture must not prevent future multi-tenant scaling.

## 11. Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Market-data outage or rate limiting | High | Shared request budget, caching, last-known-price fallback, unpriced labelling, tiered refresh rates |
| Stale or incorrect balance data | High | Show last-synced time, surface sync failures, allow manual re-sync |
| Credential compromise | Critical | Encryption at rest, never return secrets, read-only permissions, least privilege |
| Regulatory change affecting crypto reporting | High | Keep compliance rules configurable; obtain legal review before launch of tax features |
| Low trust in automated guidance | Medium | Show rationale and data freshness; require explicit human approval |
| Competitive pressure | Medium | Focus on underserved segments and on trustworthy, transparent aggregation |

## 12. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which pricing tiers, if any, are committed for launch versus tested later? | Product |
| OQ-002 | Is in-app-only reporting sufficient for launch, or is an external channel (email/PDF) needed? | Product |
| OQ-003 | What is the target regulatory posture by jurisdiction? | Compliance |
| OQ-004 | Which wallet-connection methods are required at launch (address import only vs. wallet-extension connect)? | Product |
