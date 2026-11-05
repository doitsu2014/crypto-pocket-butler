# Scope Definition Questions

## Q1. What is the minimum viable slice that delivers value?

A. The Option B retail slice: cross-venue aggregation, base-currency valuation, allocation and drift, transaction ledger with cost basis and P&L, user-initiated order intents, snapshots and briefs, billing, and the 10 launch services. Notification, Analytics and the Support Console stay deferred per Q8. [Recommended]
B. Core view only: aggregation, valuation, allocation, drift and snapshots; defer P&L, order intents, briefs and billing.
C. Core plus P&L: aggregation, valuation, drift, transactions and P&L; defer order intents, briefs and billing.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. The Option B retail slice: cross-venue aggregation, base-currency valuation, allocation and drift, transaction ledger with cost basis and P&L, user-initiated order intents, snapshots and briefs, billing, and the 10 launch services. Notification, Analytics and the Support Console stay deferred per Q8.

## Q2. What sequencing preference should order the backlog?

A. Risk-first with a walking skeleton: prove market-data licensing, the Vault pattern, jurisdiction gating and the 99.9% topology before scaling features. [Recommended]
B. Value-first: ship the first valuation to a real user as early as legally possible.
C. Dependency-first: strictly follow the asset registry → market data → portfolio → ledger DAG.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Risk-first with a walking skeleton: prove market-data licensing, the Vault pattern, jurisdiction gating and the 99.9% topology before scaling features.

## Q3. Is there a hard deadline tied to a capability?

A. No external hard date; the 9-month target is a planning commitment, not a contractual date. [Recommended]
B. Yes — a fixed launch date exists and must be recorded (describe it).
C. Yes — tied to a funding, regulatory or partner window (describe it).
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. No external hard date; the 9-month target is a planning commitment, not a contractual date.

## Derived assumptions (recorded, not separately asked)

The stage's remaining question topics were not asked as separate questions because the confirmed answers already determine them. They are recorded here as explicit assumptions so they are not inferred silently:

- **Must-have vs nice-to-have.** Derived from the BRD MoSCoW convention and Q1: BRD **Must** requirements are the MVP committed scope; BRD **Should** requirements are fast-follow and excluded from the committed effort total; **Could/Won't** and the BRD-001 post-MVP list are not built. [assumption]
- **Dependencies between capabilities.** Derived from the proto-Unit dependency columns and Q2's risk-first walking skeleton: the DAG in `intent-backlog.md` is the dependency source of truth; a dependency that cannot be resolved is a cut-list candidate, not a parallel workaround. [assumption]

**Summary confirmation:** the active `classic` scope sets `summary_confirmation: off`, so there is no separate consolidated-summary checkpoint; the approval gate is the single decision.
