# BRD-001 — Product Vision, Scope & Success

Draft v1.1 · Product · 2026-11-02. The anchor BRD: every requirement elsewhere traces to a goal or scope statement here.

## Why

Crypto investors hold assets across exchanges, EVM chains, and Solana. No single trustworthy view, drift is invisible until large, rebalancing is manual, reporting is assembled by hand, and exchange credentials are over-permissioned. Crypto Pocket Butler is a **read-only aggregation layer with direct buy/sell ordering**: no private keys, no withdrawal rights, and the product itself never places trades.

**Vision:** one trustworthy view of your crypto portfolio across wallets and exchanges, with direct buy/sell orders, guardrails, and effortless reporting.

## Users

- **Retail investor** (MVP): 3–10 wallets; wants one consolidated view and simple buy/sell actions.
- **Active trader** (MVP): fresh data, drift alerts, multiple portfolios.
- Family office and fund manager: post-MVP (multi-user, analytics, client portals).

## Goals

| ID | Goal | Achieved when |
|----|------|---------------|
| G-01 | Consolidate holdings across venues | Every configured account visible in one portfolio view |
| G-02 | Trustworthy USD valuation | Reconciles with manual spot-checks to >99% |
| G-03 | Make drift visible | Every portfolio with a target shows drift per asset |
| G-04 | Turn the portfolio view into direct action | Buy/sell orders and trades; a trade updates allocation immediately |
| G-05 | Remove manual reporting | Daily/weekly brief available in-app |
| G-06 | Protect credentials | Encrypted at rest, never returned by the API, read-only |
| G-07 | Preserve history | Snapshots immutable, trend view available |

## Scope

**MVP:** Supabase Auth sign-in (BRD-002/003) · OKX exchange and on-chain accounts (BRD-004) · EVM + Solana (BRD-010) · asset catalogue and prices (BRD-005) · portfolios, targets, guardrails (BRD-006) · USD valuation, drift, snapshots (BRD-007) · user-initiated buy/sell orders with immediate allocation effect (BRD-008) · in-app briefs (BRD-009) · chain/token registry admin (BRD-010) · scheduled refresh (BRD-011).

**Post-MVP (intent, not commitment):** investment insights (IRR, Sharpe, attribution) · tax and compliance · multi-user portfolios · mobile apps · trade execution · white-label.

**Never:** custody or private keys · automatic trade execution · fiat on/off-ramp · tax filing or regulated advice · futures/margin · transfers initiated by the product.

## Product requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-GEN-01 | Add multiple accounts and combine them into portfolios. | Must |
| FR-GEN-02 | One consolidated USD view per portfolio. | Must |
| FR-GEN-03 | Compare actual allocation against the user's target. | Must |
| FR-GEN-04 | Let users create buy/sell orders from the portfolio view; a filled order's trade affects allocation immediately. The product never places orders itself. | Must |
| FR-GEN-05 | Record immutable snapshots and show their history. | Must |
| FR-GEN-06 | Scheduled portfolio brief, readable in-app. | Should |
| FR-GEN-07 | More than one portfolio per user. | Must |
| FR-GEN-08 | Documented API for the web client and future integrations. | Should |
| FR-GEN-09 | Notify when drift exceeds guardrails. | Should |
| FR-GEN-10 | Never require exchange withdrawal permissions. | Must |

## Packaging (draft hypothesis, not committed)

Free: 3 wallets, 24h refresh · Pro $15/mo: unlimited wallets, realtime · Team $50/mo: multi-user, reports · Enterprise: custom. Tier enforcement is post-MVP.

## Success metrics

| Metric | Target |
|--------|--------|
| Valuation accuracy | >99% vs. manual calculation |
| Price freshness | <30s end-to-end |
| Wallet / chain coverage | 5+ wallet types, 10+ chains |
| Time to first value | <2 min from account creation to first portfolio view |
| Order flow completion | Orders created → executed → allocation updated without re-sync friction |
| Brief reliability | 100% of scheduled briefs generated or explicitly failed |

## Assumptions

- Read-only by design; a human approves every trade.
- OKX is the only price source: outages degrade to last-known price; no OKX pair → unpriced.
- Public addresses only; USD only; single-tenant now, no multi-tenant blockers by design.

## Risks

Market-data outage (high) → request budget, cache, fallback, unpriced labelling · stale balances (high) → show last-synced time, manual re-sync · credential compromise (critical) → encryption, read-only keys, never returned · regulation (high) → configurable rules, legal review before tax features · order mistakes (medium) → projected-allocation preview, guardrail limits, cancellable pending orders.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which pricing tiers are committed for launch? | Product |
| OQ-002 | Is in-app-only reporting enough, or is email/PDF needed? | Product |
| OQ-003 | Target regulatory posture by jurisdiction? | Compliance |
| OQ-004 | Address import only, or wallet-extension connect at launch? | Product |
