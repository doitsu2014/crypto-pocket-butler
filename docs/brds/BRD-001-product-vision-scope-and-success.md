# BRD-001 — Product Vision, Scope & Success

Draft v1.2 · Product · 2026-11-04. The anchor BRD: every requirement elsewhere traces to a goal or scope statement here. v1.2 repositions the product for **global mass-market retail** and closes the gaps identified in the 2026-11-04 readiness review.

## Why

Retail crypto investors hold assets across exchanges, wallets, and chains. They cannot answer three basic questions with confidence: **what do I own, what is it worth, and am I up or down?** Drift is invisible until it is large, rebalancing is manual, reporting is assembled by hand, and exchange credentials are over-permissioned.

Crypto Pocket Butler is a **read-only aggregation and decision-support layer with user-initiated order intents**: no private keys, no withdrawal rights, and the product never places trades in the MVP.

**Vision:** one trustworthy view of your crypto portfolio across venues and currencies, with clear P&L, guardrails, and reporting you do not have to assemble by hand.

## Users

| Segment | Priority | Profile | Jobs to be done |
|---------|----------|---------|-----------------|
| **Mainstream retail** | Primary | 1–10 accounts; mobile-first; low crypto literacy; wants simplicity and trust | "Show me what I own, whether I am up or down, and what to look at next." |
| **Crypto-native active investor** | Secondary | 3–15 wallets; multi-chain; comfortable with API keys | "Consolidate venues, compare allocation to targets, and keep a trade journal." |
| **Family office / advisor** | Post-MVP | Multi-user, client reporting, permissions | "See client portfolios, performance, and reports under access control." |

The MVP is built for the first two. Family-office features must remain designable without a migration blocker.

## Goals

| ID | Goal | Achieved when |
|----|------|---------------|
| G-01 | Consolidate holdings across venues | Every configured account is visible in one portfolio view |
| G-02 | Trustworthy valuation | Reconciles with manual spot-checks to >99%; freshness and exclusions are visible |
| G-03 | Make drift visible | Every portfolio with a target shows drift per asset |
| G-04 | Answer "am I up or down?" | Net invested, realized/unrealized P&L, and total return are available per asset and portfolio |
| G-05 | Turn the view into action | Buy/sell order intents and recorded trades; a trade updates allocation immediately |
| G-06 | Remove manual reporting | Daily/weekly brief is available in-app, by email, and as PDF |
| G-07 | Protect credentials and privacy | Read-only credentials encrypted; no secret returned or logged; user rights honoured |
| G-08 | Preserve history | Snapshots and transactions are immutable and available for trends and exports |
| G-09 | Work globally | Base currency, locale, timezone, and jurisdiction are first-class |
| G-10 | Retain users | Notifications reach users where they are; activation and D30 retention meet targets |
| G-11 | Be trustworthy and lawful | Security, privacy, support, status, and regulatory posture are launch-ready |
| G-12 | Sustain the business | Plans, entitlements, quotas, and billing work globally |

## Scope

**MVP (global retail launch):** Supabase Auth sign-in (BRD-002/003) · exchange and on-chain accounts, WalletConnect, xpub, CSV import (BRD-004) · multi-source prices, FX, EVM + Solana + Bitcoin + Tron (BRD-005) · portfolios, targets, guardrails (BRD-006) · base-currency valuation, drift, snapshots (BRD-007/018) · transaction ledger, cost basis, P&L (BRD-013) · user-initiated buy/sell order intents and trades (BRD-008) · in-app/email/PDF briefs and notifications (BRD-009/015) · performance and benchmarks (BRD-020) · chain/token registry admin (BRD-010) · scheduled refresh, transaction sync, FX, and notification jobs (BRD-011) · onboarding and education (BRD-014) · billing, entitlements, and quotas (BRD-016) · privacy, export, and deletion (BRD-017) · localization and accessibility (BRD-018) · support and status (BRD-019) · product analytics (BRD-021).

**Post-MVP (intent, not commitment):** direct trade execution through exchange APIs and on-chain signing · DeFi/staking/LP position accounting · NFT valuation · all-asset aggregation (bank/brokerage) · native mobile apps · multi-user and advisor portals · tax filing · KYC/AML · white-label · AI assistant.

**Never:** custody or private keys · automatic trade execution in the MVP · fiat on/off-ramp · tax filing or regulated advice · futures/margin · transfers initiated by the product.

## Client platforms

| Platform | MVP | Notes |
|----------|-----|-------|
| Responsive web SPA | Yes | Desktop and mobile browser |
| Installable PWA + web push | Yes | Primary mobile path before native |
| Native iOS/Android | Post-MVP | App-store billing and push rules apply |
| Email/PDF reports | Yes | Brief delivery and export |

## Product requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-GEN-01 | Add multiple accounts and combine them into portfolios. | Must |
| FR-GEN-02 | One consolidated view per portfolio in the user's base currency. | Must |
| FR-GEN-03 | Compare actual allocation against the user's target. | Must |
| FR-GEN-04 | Let users create buy/sell order intents from the portfolio view; a recorded trade affects allocation immediately. The product never places orders itself. | Must |
| FR-GEN-05 | Record immutable snapshots and show their history. | Must |
| FR-GEN-06 | Scheduled portfolio brief, readable in-app, by email, and as PDF. | Should |
| FR-GEN-07 | More than one portfolio per user. | Must |
| FR-GEN-08 | Documented API for the web client and future integrations. | Should |
| FR-GEN-09 | Notify when drift exceeds guardrails, through configured channels. | Should |
| FR-GEN-10 | Never require exchange withdrawal permissions. | Must |
| FR-GEN-11 | Record transactions and compute cost basis, net invested, and realized/unrealized P&L. | Must |
| FR-GEN-12 | Support a user-selected base currency, locale, and timezone. | Must |
| FR-GEN-13 | Deliver in-app, email, and web-push notifications with preferences. | Must |
| FR-GEN-14 | Provide an installable PWA with web push; native apps are post-MVP. | Should |
| FR-GEN-15 | Localize the product for launch locales and meet WCAG 2.2 AA on core flows. | Must |
| FR-GEN-16 | Enforce plans, entitlements, and quotas server-side. | Must |
| FR-GEN-17 | Provide user data export and in-app account deletion. | Must |
| FR-GEN-18 | Provide a help centre, support channel, and public status page. | Must |
| FR-GEN-19 | Gate access by jurisdiction and present required legal disclosures. | Must |
| FR-GEN-20 | Measure activation, engagement, retention, conversion, and quality with consent-aware analytics. | Must |

## Packaging (draft hypothesis, not committed)

Free: 3 accounts, 15-minute refresh, 90-day history · Pro: 50 accounts, 1-minute refresh for top assets, full history, notifications, P&L, exports · Team (post-MVP): multi-user, reports, routing · Enterprise (post-MVP): custom. Global pricing uses local currencies, purchasing-power tiers, and a merchant of record. Entitlements are enforced server-side (BRD-016).

## Success metrics

| Metric | Target |
|--------|--------|
| Valuation accuracy | >99% vs. manual calculation |
| Price freshness | ≤5 minutes for top-100 assets; ≤15 minutes for the tail; timestamp always shown |
| Account coverage | ≥5 exchanges and ≥10 chains at launch; ≥3 import methods |
| Time to first value | <2 minutes by address; <10 minutes by exchange key |
| Activation | ≥40% of sign-ups reach a first valuation within 24 hours |
| P&L availability | ≥90% of portfolios with imported history show P&L |
| D30 retention | ≥25% (proposed; validate in beta) |
| Free → paid conversion | ≥3% (proposed; validate in beta) |
| Brief reliability | 100% of scheduled briefs generated or explicitly failed |
| Support | 95% of tickets within first-response target |
| Availability | ≥99.9% monthly for paid plans |
| Data coverage | ≥95% of portfolio value priced and fresh |

## Assumptions

- Read-only by design; a human approves every trade.
- Multiple price sources with priority and fallback; an asset without a sourceable price is unpriced, never guessed.
- Public addresses only; no private keys.
- Base currency is user-selected; USD is the default and the accounting fallback.
- Launch is phased by jurisdiction; legal and payment readiness gate each market.
- A merchant of record handles global tax on subscriptions.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| Market-data outage or licensing gap | High | Multi-source priority, stale-price fallback, coverage labels, data-licensing review |
| Stale balances | High | Last-synced time, manual re-sync, sync-failure alerts |
| Credential compromise | Critical | Vault encryption, read-only key verification, rotation, never returned or logged |
| Cross-user leakage | Critical | Ownership checks, tests, audit, support access controls |
| Incomplete P&L history | High | Coverage labels, CSV/manual import, never fabricate cost basis |
| Regulatory breach | High | Jurisdiction gating, legal review, factual (not advisory) language |
| Order mistakes | Medium | Projected-allocation preview, guardrails, cancellable pending orders, reconciliation |
| Notification deliverability | Medium | Provider fallback, in-app record, delivery logs, dedupe |
| Billing/tax error | High | Merchant of record, tax review, audited plan changes |
| Support/incident failure | High | Status page, severity model, on-call runbooks |

## Open questions

Resolved decisions are recorded in [DECISIONS.md](./DECISIONS.md). Remaining:

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which launch markets and languages are committed for the first public release? | Product/Leadership |
| OQ-002 | Which pricing tiers and local-currency prices are committed? | Product/Finance |
| OQ-003 | Which exchanges and chains are in the launch coverage matrix by region? | Product/Data |
| OQ-004 | Which benchmark and market-data licences are secured? | Data/Legal |
| OQ-005 | Is native mobile a first-year commitment? | Product |
