# Business Glossary

One plain definition per term, grouped by business domain. The defining BRD is in parentheses.

## Identity & Access

- **Analyst** — post-MVP read-only role for analytics and shared portfolios. (BRD-002)
- **Viewer** — post-MVP read-only role for specific shared portfolios. (BRD-002)

## Account Management

Two account types, each owning its own holdings:

- **Account** — a read-only source of holdings, of two types: an exchange account or an on-chain account. Each account owns its own holdings. (BRD-004)
- **Exchange account** — account connected to a centralized exchange (OKX in the MVP) via read-only API credentials. (BRD-004)
- **On-chain account** — account identified by public address only, on one or more enabled chains; no private keys are ever accepted. (BRD-004)
- **Holding** — a quantity of an asset held in an account; holdings belong to their account. (BRD-004)
- **Frozen** — quantity on an exchange not currently available for trading or withdrawal. (BRD-004)

## Asset Catalog & Market Data

An **asset** (BTC is one asset) is the root: chain representations and metric series all belong to it.

- **Asset** — one canonical, tradable unit, identified by a unique symbol (BTC, ETH, USDC). Everything below belongs to exactly one asset. (BRD-005)
- **Asset contract** — the asset's representation on a particular chain: contract or mint address (EVM, Solana) or address format (native BTC taproot/legacy), with its decimals. One representation per asset per chain. (BRD-005)
- **Asset metric series** — point-in-time observations of one fact about an asset, each with timestamp and source. A new metric is a new series, never a model change. (BRD-005)
- **Price observation** — a metric series: the asset's USD market price at a timestamp. (BRD-005)
- **Outstanding supply** — a metric series: the asset's total circulating quantity at a timestamp. (BRD-005, post-MVP collection)
- **Inflation rate** — a metric series: the asset's annual rate of new supply issuance. (BRD-005, post-MVP collection)
- **On-chain interest rate** — a metric series: the asset's yield in an on-chain product (lending, staking), one rate per chain per product. (BRD-005, post-MVP collection)
- **Stablecoin** — a crypto asset designed to hold a stable value, typically fiat-pegged. (BRD-005)
- **Unmapped asset** — a token that cannot be resolved to a canonical asset; excluded from valuation. (BRD-005)

## Portfolio Management

- **Portfolio** — a user's grouping of accounts, with an optional target allocation and guardrails; the unit of valuation, drift, snapshots, and orders. Links multiple accounts; each account links to one portfolio at a time. (BRD-006)
- **Default portfolio** — the user's primary portfolio; exactly one per user. (BRD-006)
- **Target allocation** — intended percentage weight per asset; must total 100% when defined. (BRD-006)
- **Guardrails** — constraints orders and drift alerts must respect: maximum deviation, minimum and maximum order size, allowed assets. (BRD-006)

## Valuation & Snapshots

- **Valuation** — an asset's or portfolio's value in USD, from quantity × price. (BRD-007)
- **Allocation** — a derived side object of a portfolio: the point-in-time breakdown of its priced value by asset, each asset's weight in percent — always totalling 100% (e.g. BTC 10%, USDT 90%). (BRD-007)
- **Weight** — an asset's share of the portfolio's priced total, as a percentage; weights always sum to 100%. (BRD-007)
- **Drift** — an asset's actual weight minus its target weight, in percentage points. (BRD-007)
- **Base currency** — the currency value is reported in; USD in the MVP. (BRD-001)
- **Snapshot** — an immutable point-in-time record of holdings, valuation, and weights. **EOD** snapshots are taken on schedule, **manual** snapshots on request. (BRD-007)
- **Unpriced asset** — an asset with quantity but no usable price; excluded from the valued total and reported separately. (BRD-007)

## Orders & Trades

- **Order** — a user-created buy/sell instruction for an asset in a portfolio, created from the portfolio view. An instruction only: pending, filled, or cancelled — it never changes allocation by itself. (BRD-008)
- **Trade** — the executed transaction that fills an order: asset, side, quantity, venue (chain or exchange), execution time. A trade's effect on the portfolio allocation is immediate. (BRD-008)

## Reporting

- **Brief** — a scheduled daily or weekly portfolio report, composed in-app and readable in the SPA. (BRD-009)

## Administration

- **Chain** — a blockchain network; the MVP supports EVM chains and Solana. (BRD-010)
- **Token registry** — the maintained list of tokens, contracts/mints, and decimals used during sync. (BRD-010)

## Background Jobs

- **System job** — a scheduled process that refreshes data or creates snapshots without a user session. (BRD-011)
