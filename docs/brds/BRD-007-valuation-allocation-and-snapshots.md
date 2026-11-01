# BRD-007 — Valuation, Allocation & Snapshots

| Field | Value |
|-------|-------|
| Document ID | BRD-007 |
| Version | 1.0 |
| Status | Draft — requires product and finance sign-off |
| Owner | Product / Finance |
| Last updated | 2026-11-01 |
| Related documents | BRD-004, BRD-005, BRD-006, BRD-008, BRD-011 |

## 1. Purpose

Define how the product calculates portfolio value and allocation, how drift against target is measured, and how point-in-time snapshots are created and retained. These calculations underpin the dashboard, recommendations, and historical reporting, so their rules must be explicit, reproducible, and displayed with their data freshness.

## 2. Scope

### In scope

- Aggregating holdings across all accounts linked to a portfolio.
- Resolving assets and applying the latest USD prices.
- Computing per-asset value and portfolio weights.
- Comparing actual weights with target allocation to derive drift.
- Handling unpriced and unmapped assets.
- Manual and scheduled (EOD) snapshots.
- Snapshot history and trend presentation.

### Out of scope

- Cost basis, realised/unrealised P&L, and tax lots (post-MVP).
- Performance metrics such as IRR, CAGR, Sharpe ratio, attribution (post-MVP).
- Non-USD base currencies (post-MVP).
- Hourly snapshots (reserved as a future snapshot type; not required in MVP).
- Editing or back-dating historical snapshots.

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Portfolio Owner | Views value/allocation, takes manual snapshots, browses history |
| System jobs | Creates EOD snapshots and supplies prices |
| Admin | Can trigger jobs; sees operational outcomes |

## 4. Definitions

| Term | Definition |
|------|------------|
| **Holding** | An asset quantity held in an account. |
| **Enriched holding** | A holding combined with price, USD value, and weight. |
| **Portfolio value** | Sum of USD values of all priced assets in the portfolio. |
| **Weight** | An asset's USD value divided by portfolio value, expressed as a percentage. |
| **Target** | The user-defined intended weight for an asset. |
| **Drift** | Actual weight minus target weight, in percentage points. |
| **Snapshot** | An immutable record of portfolio holdings and values at a point in time. |
| **Unpriced asset** | An asset with a quantity but no usable price. |
| **Unmapped asset** | A token that cannot be resolved to a canonical asset. |

## 5. Valuation rules

### 5.1 Aggregation

1. Load the portfolio and verify ownership.
2. Load all active linked accounts.
3. Read each account's current holdings.
4. Merge holdings by canonical asset: quantities of the same asset across accounts and chains are summed.
5. For each merged asset, resolve the latest price.
6. Compute value as `quantity × price_usd`.
7. Sum values to obtain portfolio value.
8. Compute weight as `value_usd ÷ portfolio_value × 100`.
9. Compute drift as `actual_weight − target_weight` where a target exists.

### 5.2 Rounding and precision

- Internal calculations must use decimal arithmetic, not binary floating point, for quantities and values.
- Weights are computed to at least four decimal places; display precision may be lower.
- Displayed percentages may show rounded values, but the underlying stored values must remain precise.
- The sum of displayed weights may differ from 100% by display rounding; this must not be treated as an error.

### 5.3 Unpriced and unmapped assets

- Unpriced or unmapped holdings must not be silently dropped.
- They must be listed separately, excluded from the priced total, and clearly labelled.
- The portfolio total must state that it excludes unpriced assets.

### 5.4 Data freshness

- Every valuation must show the price timestamp (or the oldest price used).
- The UI must show the last account sync time so users understand balance freshness.
- Stale data must be labelled, not hidden.

## 6. Snapshot rules

| Rule | Description |
|------|-------------|
| Types | Manual and EOD are required; hourly is a reserved future type. |
| Uniqueness | One snapshot per portfolio, snapshot date, and type. A repeat for the same key updates the existing record rather than duplicating it. |
| Immutability | Once created, snapshot contents must not be edited. |
| Contents | Date, type, total USD value, per-asset quantity/price/value/weight, account count, asset count, creation time. |
| Source | Manual snapshots reflect data at the time of creation; EOD snapshots reflect end-of-day data. |
| Deletion | Snapshots are retained with the portfolio; deletion policy on portfolio deletion follows BRD-006. |
| Back-dating | Users may not back-date snapshots in the MVP. |

## 7. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-VAL-01 | The system must calculate portfolio value from all active linked accounts using the latest available prices. | Must |
| FR-VAL-02 | The system must merge the same canonical asset held across accounts and chains into a single allocation line. | Must |
| FR-VAL-03 | The system must compute per-asset USD value and portfolio weight. | Must |
| FR-VAL-04 | The system must compute drift per asset when a target allocation exists. | Must |
| FR-VAL-05 | The system must display unpriced and unmapped assets separately and exclude them from the valued total. | Must |
| FR-VAL-06 | The system must display the price timestamp and last sync time alongside valuations. | Should |
| FR-VAL-07 | The user must be able to take a manual snapshot for a portfolio. | Must |
| FR-VAL-08 | The system must create EOD snapshots for all portfolios on schedule. | Must |
| FR-VAL-09 | A repeat snapshot for the same portfolio/date/type must update the existing record, not create a duplicate. | Must |
| FR-VAL-10 | The user must be able to browse snapshots in reverse chronological order with date, type, total value, and holding count. | Must |
| FR-VAL-11 | The user must be able to open a snapshot and view its per-asset breakdown. | Must |
| FR-VAL-12 | The snapshot history view should include a time-series chart of total value over a selectable range. | Should |
| FR-VAL-13 | The snapshot view should allow filtering by snapshot type. | Should |
| FR-VAL-14 | The system should provide a manual refresh of the current allocation without creating a snapshot. | Could |
| FR-VAL-15 | The system must retain snapshots even when the portfolio's accounts change later. | Must |

## 8. Business rules

| ID | Rule |
|----|------|
| BR-VAL-01 | Only active accounts contribute to portfolio value and allocation. |
| BR-VAL-02 | Quantities are normalised decimals; raw integer units must not be summed across assets with different decimals. |
| BR-VAL-03 | Weights must sum to 100% (within display rounding) when priced assets exist. |
| BR-VAL-04 | Value, quantity, and weights must be non-negative. |
| BR-VAL-05 | Frozen quantity must not exceed total quantity, and available + frozen must equal total for exchange holdings. |
| BR-VAL-06 | An asset with no price contributes zero to the valued total and is reported as unpriced. |
| BR-VAL-07 | A missing target for an asset means drift is not calculated for that asset. |
| BR-VAL-08 | Snapshots are immutable after creation; corrections are made by creating a new snapshot, not editing. |
| BR-VAL-09 | A snapshot must record the account and asset counts used, so it can be reconciled later. |
| BR-VAL-10 | Deleting an account after a snapshot must not alter the historical snapshot. |
| BR-VAL-11 | A portfolio with zero priced value must display zero rather than failing. |

## 9. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-VAL-01 | Portfolio ID and timestamp | Identity of the valuation point | Internal |
| DR-VAL-02 | Per-asset quantity and price | Inputs to value | Sensitive |
| DR-VAL-03 | Per-asset value and weight | Calculated outputs | Sensitive |
| DR-VAL-04 | Target weight and drift | Comparison outputs | Sensitive |
| DR-VAL-05 | Snapshot type and date | Classification of the record | Internal |
| DR-VAL-06 | Snapshot metadata | Account count, asset count, creation time | Internal |
| DR-VAL-07 | Unpriced/unmapped list | Items excluded from valuation | Internal |
| DR-VAL-08 | Data freshness markers | Price timestamp and last sync time | Internal |

## 10. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-VAL-01 | Given BTC 0.5 at $50,000 and ETH 5 at $3,000, when valued, then BTC is $25,000 (50%), ETH is $15,000 (30%), and the total is $40,000. |
| AC-VAL-02 | Given the same asset held in two accounts, when aggregated, then it appears as one asset with combined quantity. |
| AC-VAL-03 | Given a target of 40% and an actual weight of 45%, when drift is calculated, then drift is +5 percentage points. |
| AC-VAL-04 | Given an asset with no price, when valued, then it is listed as unpriced, excluded from the total, and the UI states the exclusion. |
| AC-VAL-05 | Given a manual snapshot for today, when the same snapshot key is submitted again, then the existing record is updated and no duplicate exists. |
| AC-VAL-06 | Given an EOD job run, when it completes, then every active portfolio has exactly one EOD snapshot for that date. |
| AC-VAL-07 | Given a snapshot, when the user opens it, then the per-asset breakdown displays quantity, price, value, and weight as recorded. |
| AC-VAL-08 | Given an account that is later deleted, when the historical snapshot is viewed, then its recorded holdings remain unchanged. |
| AC-VAL-09 | Given a portfolio with no accounts, when viewed, then the value is zero and no error is shown. |

## 11. Assumptions, dependencies, constraints

- Prices are supplied under BRD-005; missing prices are expected and must be handled.
- Holdings are cached balances from BRD-004, not real-time ledger state.
- Snapshot storage must preserve the exact calculated values, not recompute them later.
- The MVP supports USD only.

## 12. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | What is the exact EOD cutoff timezone, and should it follow the user or UTC? | Product |
| OQ-002 | Should snapshots be created for empty portfolios? | Product |
| OQ-003 | What is the retention period for snapshots and historical prices? | Product |
| OQ-004 | Should the chart use snapshot values only, or also interpolated daily prices? | Product |
| OQ-005 | How should a portfolio with all assets unpriced be represented in history? | Product |
