# BRD-005 — Asset Catalog & Market Data

Draft v1.2 · Product/Data · 2026-11-04. The canonical asset catalogue, token→asset mapping, price collection, and FX. Every valuation, snapshot, P&L, order, and benchmark depends on this data being correct, fresh, and lawful to use. v1.2 adds multi-source pricing, FX, long-tail coverage, stablecoin de-peg handling, and data licensing.

**In scope:** canonical asset records · chain representations and decimals · metric series (price, FX, and future supply/inflation/interest) · multi-source price collection with priority and fallback · regional pair availability · long-tail and unpriced handling · stablecoin de-peg detection · price alerts thresholds · watchlists · source health · data licensing and attribution.
**Out of scope:** cost basis/P&L (BRD-013) · NFT valuation · derivatives · full DeFi position pricing · custom user price feeds · trading.

## Model

The **asset** is the root (BTC is one asset): unique symbol + name, type, and rank. Everything else hangs off it in two shapes.

**Chain representations** — one per asset per chain, with fixed decimals: a contract or mint address (EVM, Solana, Tron), an address format (native Bitcoin), or a native symbol. A synced token resolves via the registry → contract mapping; still unresolved = **unmapped asset**: visible, excluded from valuation, never guessed.

**Metric series** — point-in-time observations, each with timestamp, source, and confidence:

| Series | MVP | Notes |
|--------|-----|-------|
| Price (USD spot, 24h volume, 24h change) | Yes | Multi-source with priority and fallback; stale prices are labelled |
| FX rate (USD ↔ user currencies) | Yes | Daily or hourly series; historical rates for transactions and snapshots |
| Outstanding supply | Post-MVP | Circulating quantity over time |
| Inflation rate | Post-MVP | Annual new-supply issuance rate |
| On-chain interest rate | Post-MVP | One series per chain **and per product** |

All series share one shape — asset or currency pair + timestamp + value + source — so a new metric is a new series, never a model change.

## Price source priority

| Priority | Source | Use |
|----------|--------|-----|
| 1 | Primary exchange pair | Latest tradable price for listed assets |
| 2 | Secondary exchange / aggregator | Cross-check and fallback |
| 3 | DEX or on-chain reference | Long-tail assets, clearly labelled |
| 4 | Last stored price | Outage fallback only; labelled with age |
| — | None | `unpriced`, excluded from totals |

A price is never silently presented as current. The UI shows the observation time, source, and staleness state.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-AST-01 | Maintain a canonical asset record for every valued asset. | Must |
| FR-AST-02 | Asset symbols are unique in the catalogue. | Must |
| FR-AST-03 | One contract/mint per asset per chain; duplicates rejected. | Must |
| FR-AST-04 | Store decimals and use them to normalise balances. | Must |
| FR-AST-05 | Resolve synced tokens via registry and contract mappings. | Must |
| FR-AST-06 | Record unmapped assets explicitly; exclude from valuation. | Must |
| FR-AST-07 | Collect prices from at least two sources with priority and fallback. | Must |
| FR-AST-08 | Fall back to the last stored price when collection fails; label its age. | Must |
| FR-AST-09 | Store source, timestamp, and confidence with every metric observation. | Must |
| FR-AST-10 | Show the price timestamp and source wherever valuation or P&L is shown. | Must |
| FR-AST-11 | Admin can refresh the catalogue and registry on demand. | Should |
| FR-AST-12 | Retain observation history for snapshots, P&L, and trends. | Must |
| FR-AST-13 | Multiple configurable price sources per asset and region. | Must |
| FR-AST-14 | Never value an asset without a sourceable price — mark it unpriced. | Must |
| FR-AST-15 | Prioritise sources per asset and fall back deterministically. | Must |
| FR-AST-16 | Collect and store FX rates for every supported base currency. | Must |
| FR-AST-17 | Provide historical FX rates for transaction-date and snapshot conversion. | Must |
| FR-AST-18 | Respect regional pair availability and do not imply a pair that does not exist. | Must |
| FR-AST-19 | Detect stablecoin de-peg beyond a threshold and trigger an alert. | Should |
| FR-AST-20 | Cover long-tail assets through a secondary or on-chain source, or mark them unpriced with a reason. | Should |
| FR-AST-21 | Allow an Admin manual price override with an audit record; never expose it as a market price. | Could |
| FR-AST-22 | Display source attribution and honour data-licensing terms. | Must |
| FR-AST-23 | Apply freshness tiers by asset liquidity and rank. | Should |
| FR-AST-24 | Expose user-configurable price-alert thresholds. | Should |
| FR-AST-25 | Surface source health, outage, and last-success time to Admins. | Should |
| FR-AST-26 | Let users maintain a watchlist of assets with price and change, independent of holdings. | Should |

## Business rules

| ID | Rule |
|----|------|
| BR-AST-01 | Symbols unique; aliases map to the canonical symbol. |
| BR-AST-02 | Same asset, different contracts per chain: allowed. |
| BR-AST-03 | One contract maps to exactly one asset per chain. |
| BR-AST-04 | Decimals are immutable per contract once used in a valuation; changes require review and re-normalisation. |
| BR-AST-05 | Metric observations are points in time, not guarantees; always show the observation time. |
| BR-AST-06 | Unpriced assets are visible with a clear indicator and excluded from the priced total. |
| BR-AST-07 | Stablecoins value at market price, never a hard-coded $1. |
| BR-AST-08 | The catalogue is shared; only Admins change registry data. |
| BR-AST-09 | No manual price overrides without an audit record. |
| BR-AST-10 | Stale prices are labelled and never presented as current. |
| BR-AST-11 | Source priority is deterministic and recorded with every observation. |
| BR-AST-12 | FX rates are data, not a display toggle; the rate and timestamp are disclosed. |
| BR-AST-13 | A de-peg alert is factual; it does not recommend a trade. |
| BR-AST-14 | Data licensing and attribution requirements are enforced before display. |
| BR-AST-15 | A manual override never contributes to a benchmark or market-history series. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-AST-01 | A new asset saves with a unique symbol, usable for resolution. |
| AC-AST-02 | A registered token on an enabled chain resolves to its asset with correct decimals. |
| AC-AST-03 | An unregistered token is marked unmapped and excluded from the valued total. |
| AC-AST-04 | During a market-data outage, valuation uses the last known price and shows its timestamp. |
| AC-AST-05 | A valuation exposes price source and observation time. |
| AC-AST-06 | A duplicate contract registration is rejected with a clear error. |
| AC-AST-07 | The primary source fails; the secondary source is used and recorded. |
| AC-AST-08 | An asset without any source is `unpriced`, with a reason and exclusion from the total. |
| AC-AST-09 | An FX rate is stored with timestamp and source, and used for historical conversion. |
| AC-AST-10 | A stablecoin crossing the de-peg threshold produces an alert. |
| AC-AST-11 | A region without the pair does not show a fabricated price. |
| AC-AST-12 | A manual override is audited and visually distinct from market data. |
| AC-AST-13 | A watchlist shows price and change without affecting portfolio valuation. |

## Notes

- Price collection is rate-limited: a shared request budget applies; metadata refreshes daily, prices more often (BRD-011).
- USD remains the accounting fallback; user base currencies are converted through the FX series.
- Deterministic recorded mappings beat heuristics.
- Data licensing and attribution are legal requirements, not implementation details.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which tracked assets lack an exchange pair at launch, and how are they surfaced? | Data |
| OQ-002 | Price-history retention period? | Product |
| OQ-003 | Policy for delisted/renamed assets? | Data |
| OQ-004 | Allow manual pricing of unmapped assets? | Product |
| OQ-005 | What counts as "stale" for display and alerting per asset tier? | Product |
| OQ-006 | Which secondary price sources and licences are approved? | Data/Legal |
| OQ-007 | Stablecoin de-peg threshold and alert cadence? | Product |
