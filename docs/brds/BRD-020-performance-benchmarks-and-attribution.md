# BRD-020 — Performance, Benchmarks & Attribution

Draft v1.0 · Product/Finance · 2026-11-04. "Am I up?" is only half the retail question. "Am I doing better than just holding Bitcoin?" is the other half. This BRD defines returns, benchmarks, and attribution so the product can explain performance without giving advice.

**In scope:** simple and time-weighted returns · money-weighted return (MWR/IRR) · period returns · benchmark comparison (BTC, ETH, a configurable market index) · contribution and attribution · cash-flow handling · base-currency returns · methodology disclosure · data-quality labels.
**Out of scope:** forecasts, targets, recommendations, or advice · tax · risk-adjusted metrics beyond volatility and maximum drawdown at launch (Sharpe/Sortino post-MVP) · derivatives and leverage · benchmark licensing not yet secured.

## Definitions

| Metric | Definition | Notes |
|--------|------------|-------|
| Simple return | (End value − net invested) ÷ net invested | Suitable for a single deposit; misleading with flows |
| TWR | Time-weighted return, removing the effect of external cash flows | Preferred for comparing to a benchmark |
| MWR / IRR | Money-weighted return, reflecting the timing and size of flows | Preferred for "my actual experience" |
| Period return | Return over day, week, month, quarter, year, or custom range | Uses TWR by default |
| Benchmark | BTC, ETH, or a configurable broad crypto index | Source, currency, and timestamp shown |
| Contribution | Asset-level effect on portfolio return | Sums to the portfolio return within rounding |
| Attribution | Split of return into market movement, flows, fees, and income | Must reconcile |

All returns are computed in the user's base currency, with the FX policy disclosed. Benchmark comparison uses the same currency and period.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-PERF-01 | Show simple return, TWR, and MWR for a portfolio over selectable periods. | Must |
| FR-PERF-02 | Handle external deposits and withdrawals correctly in TWR and MWR. | Must |
| FR-PERF-03 | Compare portfolio return with a configurable benchmark. | Should |
| FR-PERF-04 | Show asset-level contribution to portfolio return. | Should |
| FR-PERF-05 | Decompose return into market movement, flows, fees, and income where data allows. | Should |
| FR-PERF-06 | Show the methodology and formula behind each metric. | Must |
| FR-PERF-07 | Label returns `unavailable` when transaction history is incomplete. | Must |
| FR-PERF-08 | Show the benchmark's source, currency, and observation time. | Must |
| FR-PERF-09 | Support base-currency and local-currency views with the FX rate disclosed. | Must |
| FR-PERF-10 | Show volatility and maximum drawdown for selectable periods. | Could |
| FR-PERF-11 | Show risk-adjusted metrics (Sharpe, Sortino) after methodology review. | Could |
| FR-PERF-12 | Provide a chart with accessible text alternatives and data table. | Must |
| FR-PERF-13 | Recompute on cost-basis or transaction changes and record the change. | Must |
| FR-PERF-14 | Exclude deposits, withdrawals, and internal transfers from investment return. | Must |
| FR-PERF-15 | Show the effect of fees separately from market movement. | Should |
| FR-PERF-16 | Allow a user to hide benchmarks without hiding the portfolio's own return. | Should |
| FR-PERF-17 | Provide an export of performance data to CSV/PDF. | Should |
| FR-PERF-18 | Never present performance as a forecast or recommendation. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-PERF-01 | Returns are computed from recorded transactions and valuations; missing data is labelled, never fabricated. |
| BR-PERF-02 | TWR removes external cash-flow effects; MWR reflects them. |
| BR-PERF-03 | Internal transfers are not external cash flows. |
| BR-PERF-04 | Benchmark comparisons use the same period, currency, and valuation timestamps. |
| BR-PERF-05 | Methodology is disclosed next to the number, not buried in documentation. |
| BR-PERF-06 | Performance is factual information, never advice or a forecast. |
| BR-PERF-07 | A return is not shown when the data coverage is below the configured threshold. |
| BR-PERF-08 | Fees are disclosed as a drag where the data supports it. |
| BR-PERF-09 | Benchmark licensing and attribution requirements are honoured. |
| BR-PERF-10 | Recomputations are audited and reproducible. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-PERF-01 | A portfolio with one deposit shows the same simple and TWR return for the period. |
| AC-PERF-02 | A mid-period deposit does not inflate TWR; MWR reflects its timing. |
| AC-PERF-03 | An internal transfer between own accounts changes neither TWR nor MWR. |
| AC-PERF-04 | A benchmark comparison shows the same period and currency. |
| AC-PERF-05 | Incomplete transaction history labels the return `unavailable`. |
| AC-PERF-06 | The methodology is visible from the metric. |
| AC-PERF-07 | Asset contributions sum to the portfolio return within rounding. |
| AC-PERF-08 | A change to cost basis triggers an audited recomputation. |
| AC-PERF-09 | No performance screen contains a recommendation or forecast. |
| AC-PERF-10 | Performance exports contain the methodology and timestamp. |

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which benchmarks launch: BTC, ETH, a broad index, or all three? | Product |
| OQ-002 | Benchmark data source and licensing? | Data/Legal |
| OQ-003 | Minimum data coverage before a return is shown? | Product |
| OQ-004 | Are risk-adjusted metrics in scope for the first year? | Product |
| OQ-005 | How is attribution presented without implying advice? | Product/Legal |
