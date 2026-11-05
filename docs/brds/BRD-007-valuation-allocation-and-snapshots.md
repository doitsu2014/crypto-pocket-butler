# BRD-007 — Valuation, Allocation & Snapshots

Draft v1.1 · Product/Finance · 2026-11-04. How value, weight, and drift are calculated, and how snapshots preserve history. **Allocation is a derived side object of a portfolio**: a point-in-time view of its priced value by asset, always totalling 100%. v1.1 adds base-currency conversion, historical FX, user-timezone snapshots, change attribution, and data-coverage stamps.

**In scope:** aggregation across member accounts · latest-price valuation in the user's base currency · per-asset value and weight · drift vs target · unpriced/unmapped handling · manual and EOD snapshots · history, trends, and change attribution · data-coverage labels · retention.
**Out of scope:** cost basis/P&L (BRD-013) · IRR/Sharpe/attribution methodology detail (BRD-020) · tax lots · editing or back-dating snapshots.

## Calculation

1. Verify ownership; load all **active** member accounts and their holdings.
2. Merge the same canonical asset across accounts and chains into one line.
3. Resolve the latest price per asset and the FX rate for the user's base currency.
4. Value = quantity × price × FX; portfolio value = sum of priced values.
5. Weight = value ÷ total × 100.
6. Drift = actual weight − target weight (only where a target exists).
7. Attach data coverage: priced share of value, price timestamps, FX timestamp, last sync time.

**Precision:** decimal arithmetic everywhere (never binary float); weights to ≥4 decimals internally, display may round; rounded weights summing to ≠100% is not an error. Currency conversion uses the stored rate at calculation time.

**Exclusions:** unpriced and unmapped assets are listed separately, clearly labelled, excluded from the total — and the total says so. Stale data is labelled, never hidden.

**Snapshots:** immutable records of date (user timezone), type (manual/EOD), base currency, FX rate and source, total value, per-asset quantity/price/value/weight, account and asset counts, data coverage, and — where available — cost basis and P&L. One per portfolio + local date + type; a repeat updates, never duplicates. No back-dating in the MVP. Deleting an account later never changes a historical snapshot.

**Change attribution:** where transaction history is available, the change between two valuations is split into market movement, external flows, fees, and income. The split is labelled as an estimate where data is incomplete.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-VAL-01 | Value portfolios from all active member accounts at latest prices. | Must |
| FR-VAL-02 | Merge the same asset across accounts/chains into one allocation line. | Must |
| FR-VAL-03 | Compute per-asset value and portfolio weight in the user's base currency. | Must |
| FR-VAL-04 | Compute drift per asset where a target exists. | Must |
| FR-VAL-05 | Show unpriced/unmapped assets separately, excluded from the total. | Must |
| FR-VAL-06 | Show price timestamp, FX timestamp, and last sync time with valuations. | Must |
| FR-VAL-07 | Take a manual snapshot. | Must |
| FR-VAL-08 | Create EOD snapshots for active portfolios on schedule. | Must |
| FR-VAL-09 | Repeat portfolio/date/type snapshots update, not duplicate. | Must |
| FR-VAL-10 | Browse snapshots reverse-chronologically with date, type, total, and holding count. | Must |
| FR-VAL-11 | Open a snapshot's per-asset breakdown. | Must |
| FR-VAL-12 | Time-series chart of total value over a selectable range. | Must |
| FR-VAL-13 | Filter snapshot history by type. | Should |
| FR-VAL-14 | Refresh current allocation without creating a snapshot. | Could |
| FR-VAL-15 | Snapshots survive later account changes. | Must |
| FR-VAL-16 | Value and display in the user's base currency with the FX rate and timestamp. | Must |
| FR-VAL-17 | Store the base currency and FX rate with every snapshot. | Must |
| FR-VAL-18 | Use the user's timezone (including daylight saving) for EOD dates and grouping. | Must |
| FR-VAL-19 | Split period change into market movement, flows, fees, and income where data allows. | Should |
| FR-VAL-20 | Represent an all-unpriced portfolio as zero with an explicit exclusion label, not an error. | Must |
| FR-VAL-21 | Include data coverage and P&L availability in every snapshot. | Must |
| FR-VAL-22 | Export snapshots and allocation history to CSV/PDF. | Should |
| FR-VAL-23 | Apply the retention schedule in BRD-017; never delete history silently. | Must |
| FR-VAL-24 | Skip EOD snapshots for portfolios with no active members, unless configured otherwise. | Should |

## Business rules

| ID | Rule |
|----|------|
| BR-VAL-01 | Only active accounts contribute to value and allocation. |
| BR-VAL-02 | Quantities are normalised decimals; raw integers are never summed across different decimals. |
| BR-VAL-03 | Weights sum to 100% (within display rounding) when priced assets exist. |
| BR-VAL-04 | Value, quantity, and weights are non-negative. |
| BR-VAL-05 | Available + frozen = total; frozen ≤ total. |
| BR-VAL-06 | No price → zero contribution, reported as unpriced. |
| BR-VAL-07 | No target → no drift for that asset. |
| BR-VAL-08 | Snapshots are immutable; corrections come from new snapshots. |
| BR-VAL-09 | Snapshots record account, asset, and data-coverage counts for later reconciliation. |
| BR-VAL-10 | Account deletion never alters historical snapshots. |
| BR-VAL-11 | Zero priced value displays zero, not an error. |
| BR-VAL-12 | A snapshot records its base currency and FX rate; changing display currency never rewrites history. |
| BR-VAL-13 | EOD dates follow the user's timezone at generation time. |
| BR-VAL-14 | Change attribution is labelled as an estimate when transaction history is incomplete. |
| BR-VAL-15 | Unpriced/unmapped exclusions are stated wherever a total is shown. |
| BR-VAL-16 | Retention is automatic and auditable. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-VAL-01 | BTC 0.5 @ $50k and ETH 5 @ $3k → BTC $25k (50%), ETH $15k (30%), total $40k. |
| AC-VAL-02 | The same asset in two accounts appears once, combined. |
| AC-VAL-03 | Target 40%, actual 45% → drift +5pp. |
| AC-VAL-04 | An unpriced asset is listed, excluded from the total, and the UI states the exclusion. |
| AC-VAL-05 | Re-submitting today's manual snapshot updates the record; no duplicate. |
| AC-VAL-06 | After the EOD job, every active portfolio has exactly one EOD snapshot for the local date. |
| AC-VAL-07 | Opening a snapshot shows its recorded per-asset breakdown. |
| AC-VAL-08 | A later-deleted account leaves historical snapshots unchanged. |
| AC-VAL-09 | A portfolio with no accounts shows zero without error. |
| AC-VAL-10 | A EUR-base user sees EUR values with the FX rate and timestamp, and the snapshot stores both. |
| AC-VAL-11 | A user in a DST timezone gets EOD snapshots on the correct local date across the transition. |
| AC-VAL-12 | A portfolio with no priced assets shows zero with an exclusion label. |
| AC-VAL-13 | The snapshot's data coverage matches the priced share at generation time. |
| AC-VAL-14 | A period change split reconciles to the total change within rounding. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Resolved by D-006: user timezone default, optional portfolio override. | Product |
| OQ-002 | Snapshots for empty portfolios: skip or zero? | Product |
| OQ-003 | Retention period for snapshots and prices? | Product/Compliance |
| OQ-004 | Chart from snapshots only, or interpolated daily prices? | Product |
| OQ-005 | How should an all-unpriced portfolio appear in history? | Product |
