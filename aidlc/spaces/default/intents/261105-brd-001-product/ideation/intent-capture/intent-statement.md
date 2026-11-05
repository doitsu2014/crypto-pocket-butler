# Intent Statement — Crypto Pocket Butler

## Problem Statement

Retail crypto investors hold assets across exchanges, wallets, and chains and cannot confidently answer three basic questions: what they own, what it is worth, and whether they are up or down. [Q1]

The initiative is a read-only aggregation and decision-support layer with user-initiated order intents. It never custodies assets or private keys and initiates no transfers. [Q4]

## Target Customer

Mainstream retail investors are the primary customer; crypto-native active investors are the secondary customer. [Q1]

Both segments feel the same pain: they cannot confidently answer what they own, what it is worth, and whether they are up or down. [Q1]

Family offices and advisors are post-MVP. [Q1] Multi-user and advisor portals are also post-MVP. [Q4]

## Success Metrics

| Metric | Target | Source |
|--------|--------|--------|
| Valuation accuracy | >99% versus manual calculation | [Q2] |
| Price freshness | ≤5 minutes for top-100 assets; ≤15 minutes for the tail | [Q2] |
| Activation | ≥40% of sign-ups reach a first valuation within 24 hours | [Q2] |
| D30 retention | ≥25% | [Q2] |
| Free-to-paid conversion | ≥3% | [Q2] |
| Availability | ≥99.9% monthly for paid plans | [Q2] |

Every requirement downstream of this intent must carry a testable acceptance criterion. [memory:M1]

## Initiative Trigger

The trigger is a global mass-market retail launch, following a 2026-11-04 readiness review that closed scope and compliance gaps. [Q3]

## Initial Scope Signal

- Workflow-selected scope: `classic`. [scope]
- User-confirmed product boundary for this first version: never custody assets or private keys; never place trades automatically (a human approves every user-initiated order intent); no fiat on/off-ramp; no tax filing or regulated advice; no futures or margin; the product initiates no transfers. Direct trade execution, DeFi/staking/LP accounting, NFT valuation, native mobile apps, multi-user/advisor portals, and KYC/AML are post-MVP. [Q4]
- Greenfield work starts with the thinnest end-to-end slice. [memory:M2]

## Assumptions & Open Questions

- Customer communication channels and cadence are not yet defined. This assumption was accepted by the user in the Assumption Confirmation. [assumption]
