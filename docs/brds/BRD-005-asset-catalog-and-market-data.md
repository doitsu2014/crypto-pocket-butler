# BRD-005 — Asset Catalog & Market Data

Draft v1.1 · Product/Data · 2026-11-02. The canonical asset catalogue, token→asset mapping, and price collection. Every valuation, snapshot, and order depends on this data being correct and fresh.

**In scope:** canonical asset records (the root) · chain representations with decimals · the metric-series model — price collection (OKX, last-known fallback) in the MVP, supply/inflation/interest series later · asset resolution · freshness handling.
**Out of scope:** cost basis/P&L · NFT valuation · derivatives/staking income · custom price sources · trading.

## Model

The **asset** is the root (BTC is one asset): unique symbol + name, type, and rank (from 24h volume). Everything else hangs off it in two shapes:

**Chain representations** — one per asset per chain, with fixed decimals: a contract or mint address (EVM, Solana) or an address format (native BTC taproot/legacy). A synced token resolves via the registry → contract mapping; still unresolved = **unmapped asset**: visible, excluded from valuation, never guessed.

**Metric series** — point-in-time observations, each with timestamp and source:

| Series | MVP | Notes |
|--------|-----|-------|
| Price (USD spot, 24h volume, 24h change) | Yes | From the OKX market-data API; on outage fall back to the last stored price, never silently shown as current |
| Outstanding supply | Post-MVP | Circulating quantity over time |
| Inflation rate | Post-MVP | Annual new-supply issuance rate |
| On-chain interest rate | Post-MVP | One series per chain **and per product** (lending, staking) — rates differ per chain and per product |

All series share one shape — asset + timestamp + value + source — so a new metric is a new series, never a model change.

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-AST-01 | Maintain a canonical asset record for every valued asset. | Must |
| FR-AST-02 | Asset symbols are unique in the catalogue. | Must |
| FR-AST-03 | One contract/mint per asset per chain; duplicates rejected. | Must |
| FR-AST-04 | Store decimals and use them to normalise balances. | Must |
| FR-AST-05 | Resolve synced tokens via registry and contract mappings. | Must |
| FR-AST-06 | Record unmapped assets explicitly; exclude from valuation. | Must |
| FR-AST-07 | Collect prices from the exchange API on a schedule (BRD-011). | Must |
| FR-AST-08 | Fall back to last stored price when collection fails. | Must |
| FR-AST-09 | Store source and timestamp with every metric observation. | Must |
| FR-AST-10 | Show the price timestamp wherever valuation is shown. | Should |
| FR-AST-11 | Admin can refresh the catalogue and registry on demand. | Should |
| FR-AST-12 | Retain observation history for snapshots and trends (prices in the MVP; supply, inflation, and interest-rate series later). | Should |
| FR-AST-13 | Multiple configurable price sources. | Could |
| FR-AST-14 | Never value an asset without a sourceable price — mark it unpriced. | Must |

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

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-AST-01 | A new asset saves with a unique symbol, usable for resolution. |
| AC-AST-02 | A registered token on an enabled chain resolves to its asset with correct decimals. |
| AC-AST-03 | An unregistered token is marked unmapped and excluded from the valued total. |
| AC-AST-04 | During a market-data outage, valuation uses the last known price and shows its timestamp. |
| AC-AST-05 | A valuation exposes price source and observation time. |
| AC-AST-06 | A duplicate contract registration is rejected with a clear error. |

## Notes

- The exchange API is rate-limited: a shared request budget applies; metadata refreshes daily, prices more often (BRD-011).
- USD-denominated in the MVP. Deterministic recorded mappings beat heuristics.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which tracked assets lack an exchange pair at launch, and how are they surfaced? | Data |
| OQ-002 | Price-history retention period? | Product |
| OQ-003 | Policy for delisted/renamed assets? | Data |
| OQ-004 | Allow manual pricing of unmapped assets? | Product |
| OQ-005 | What counts as "stale" for display and alerting? | Product |
