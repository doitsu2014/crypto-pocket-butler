# BRD-007 — Valuation, Allocation & Snapshots

Draft v1.0 · Product/Finance · 2026-11-01. How value, weight, and drift are calculated, and how snapshots preserve history. **Allocation is a derived side object of a portfolio**: a point-in-time view of its priced value by asset, always totalling 100% (e.g. BTC 10%, USDT 90%). These rules must be explicit, reproducible, and shown with their data freshness.

**In scope:** aggregation across linked accounts · latest-price valuation · per-asset value and weight · drift vs target · unpriced/unmapped handling · manual and EOD snapshots · history and trends.
**Out of scope:** cost basis/P&L/tax lots · IRR/Sharpe/attribution · non-USD · hourly snapshots (reserved) · editing or back-dating snapshots.

## Calculation

1. Verify ownership; load all **active** linked accounts and their holdings.
2. Merge the same canonical asset across accounts and chains into one line.
3. Resolve the latest USD price per asset; value = quantity × price.
4. Portfolio value = sum of priced values. Weight = value ÷ total × 100.
5. Drift = actual weight − target weight (only where a target exists).

**Precision:** decimal arithmetic everywhere (never binary float); weights to ≥4 decimals internally, display may round; rounded weights summing to ≠100% is not an error.

**Exclusions:** unpriced and unmapped assets are listed separately, clearly labelled, excluded from the total — and the total says so. Stale data is labelled, never hidden: show price timestamp and last sync time.

**Snapshots:** immutable records of date, type (manual/EOD; hourly reserved), total value, per-asset quantity/price/value/weight, account and asset counts. One per portfolio + date + type; a repeat updates, never duplicates. No back-dating in the MVP. Deleting an account later never changes a historical snapshot.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-VAL-01 | Value portfolios from all active linked accounts at latest prices. | Must |
| FR-VAL-02 | Merge the same asset across accounts/chains into one allocation line. | Must |
| FR-VAL-03 | Compute per-asset USD value and portfolio weight. | Must |
| FR-VAL-04 | Compute drift per asset where a target exists. | Must |
| FR-VAL-05 | Show unpriced/unmapped assets separately, excluded from the total. | Must |
| FR-VAL-06 | Show price timestamp and last sync time with valuations. | Should |
| FR-VAL-07 | Take a manual snapshot. | Must |
| FR-VAL-08 | EOD snapshots for all portfolios on schedule. | Must |
| FR-VAL-09 | Repeat portfolio/date/type snapshots update, not duplicate. | Must |
| FR-VAL-10 | Browse snapshots reverse-chronologically with date, type, total, holding count. | Must |
| FR-VAL-11 | Open a snapshot's per-asset breakdown. | Must |
| FR-VAL-12 | Time-series chart of total value over a selectable range. | Should |
| FR-VAL-13 | Filter snapshot history by type. | Should |
| FR-VAL-14 | Refresh current allocation without creating a snapshot. | Could |
| FR-VAL-15 | Snapshots survive later account changes. | Must |

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
| BR-VAL-09 | Snapshots record account and asset counts for later reconciliation. |
| BR-VAL-10 | Account deletion never alters historical snapshots. |
| BR-VAL-11 | Zero priced value displays zero, not an error. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-VAL-01 | BTC 0.5 @ $50k and ETH 5 @ $3k → BTC $25k (50%), ETH $15k (30%), total $40k. |
| AC-VAL-02 | The same asset in two accounts appears once, combined. |
| AC-VAL-03 | Target 40%, actual 45% → drift +5pp. |
| AC-VAL-04 | An unpriced asset is listed, excluded from the total, and the UI states the exclusion. |
| AC-VAL-05 | Re-submitting today's manual snapshot updates the record; no duplicate. |
| AC-VAL-06 | After the EOD job, every active portfolio has exactly one EOD snapshot for the date. |
| AC-VAL-07 | Opening a snapshot shows its recorded per-asset breakdown. |
| AC-VAL-08 | A later-deleted account leaves historical snapshots unchanged. |
| AC-VAL-09 | A portfolio with no accounts shows zero without error. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | EOD cutoff timezone — user-local or UTC? | Product |
| OQ-002 | Snapshots for empty portfolios? | Product |
| OQ-003 | Retention period for snapshots and prices? | Product |
| OQ-004 | Chart from snapshots only, or interpolated daily prices? | Product |
| OQ-005 | Representing an all-unpriced portfolio in history? | Product |
