# BRD-005 — Asset Catalog & Market Data

| Field | Value |
|-------|-------|
| Document ID | BRD-005 |
| Version | 1.1 |
| Status | Draft — requires product and data sign-off |
| Owner | Product / Data |
| Last updated | 2026-11-02 |
| Related documents | BRD-004, BRD-007, BRD-010, BRD-011 |

## 1. Purpose

Define the canonical asset catalogue, the mapping of on-chain tokens to assets, the collection of market prices, and the rules that make valuation consistent and auditable. Every holding, allocation, snapshot, and recommendation depends on this document's data being correct and fresh.

## 2. Scope

### In scope

- A canonical asset record per tradeable asset (symbol, name, rank, type).
- Contract/mint mappings per chain, including token decimals.
- Price collection from the OKX exchange market-data API, with last-known-price fallback.
- Asset identity resolution: converting a chain-level token into a portfolio-level asset.
- Price freshness, caching, and missing-price handling.

### Out of scope

- Cost basis, realised/unrealised P&L, and transaction-level accounting (post-MVP).
- NFT valuation and NFT metadata — later BRD revision.
- Derivatives, staking rewards, and income classification — post-MVP.
- User-supplied custom price sources — post-MVP.
- Buying, selling, or transferring assets.

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Portfolio Owner | Selects assets when defining target allocations; sees prices and values |
| Admin | Maintains the chain/token registry (BRD-010) |
| System jobs | Refresh the asset catalogue and prices |
| Exchange market data (OKX) | Supplies ticker prices, candle histories and instrument/pair metadata |

## 4. Canonical asset model

| Data element | Description | Rule |
|--------------|-------------|------|
| Symbol | Human-readable ticker (e.g., BTC, ETH, USDC) | Unique across the catalogue |
| Name | Display name | Required |
| Rank | Refresh prioritisation | Derived from exchange ticker 24h volume |
| Asset type | Classification (crypto, stablecoin, token, etc.) | Required for filtering |
| Contracts | One or more chain-specific contract/mint addresses | One address per asset per chain |
| Decimals | Token precision per contract | Required for balance normalisation |
| Price | Latest USD price and market context | Sourced from exchange ticker data |

### 4.1 Asset identity resolution

1. Receive a token identifier from an account sync (contract address or mint address, plus chain).
2. Look up the token in the registered EVM/Solana token registry.
3. If found, resolve to the canonical asset symbol.
4. If not found, apply the asset-contract mapping.
5. If still unresolved, treat the holding as an **unmapped asset** and exclude it from valuation while surfacing it to the user/admin.
6. Never guess a symbol from a contract address without a registered mapping.

## 5. Market data

### 5.1 Sources

- **Primary:** the OKX exchange market-data API — tickers, candles and instrument/pair metadata.
- **Fallback:** the last known stored price; assets without a supported exchange pair are unpriced.
- Prices are stored with source, timestamp, and asset identity.

### 5.2 Price context

For each price record, retain where available:

- USD spot price.
- 24-hour volume.
- 24-hour change percentage.
- Source and retrieval timestamp.

### 5.3 Freshness

- Scheduled price collection runs on a defined cadence (BRD-011).
- Portfolio valuation uses the latest available price and must display its timestamp.
- Stale prices must not be silently presented as current.

## 6. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-AST-01 | The system must maintain a canonical asset record for every asset used in valuation. | Must |
| FR-AST-02 | Asset symbols must be unique within the catalogue. | Must |
| FR-AST-03 | The system must store one contract/mint address per asset per chain and reject duplicates. | Must |
| FR-AST-04 | The system must store token decimals and use them to normalise raw balances. | Must |
| FR-AST-05 | The system must resolve synced tokens to canonical assets using the registry and contract mappings. | Must |
| FR-AST-06 | The system must record unmapped assets explicitly and exclude them from valuation. | Must |
| FR-AST-07 | The system must collect prices from the exchange market-data API on a schedule. | Must |
| FR-AST-08 | The system must fall back to the last known stored price when collection fails. | Must |
| FR-AST-09 | The system must store price source and timestamp with every price. | Must |
| FR-AST-10 | The system must expose the price timestamp wherever a valuation is shown. | Should |
| FR-AST-11 | An Admin must be able to refresh the asset catalogue and token registry on demand. | Should |
| FR-AST-12 | The system should retain price history sufficient to support snapshots and trend reporting. | Should |
| FR-AST-13 | The system should support multiple price sources selectable by configuration. | Could |
| FR-AST-14 | The system must not value an asset at a price it cannot source; it must mark it unpriced. | Must |

## 7. Business rules

| ID | Rule |
|----|------|
| BR-AST-01 | Asset symbols are unique; aliases must be mapped to the canonical symbol rather than creating duplicates. |
| BR-AST-02 | The same asset may have different contract addresses on different chains. |
| BR-AST-03 | A contract address maps to exactly one asset on a given chain. |
| BR-AST-04 | Decimals are immutable per contract once used in a valuation; a change requires review and re-normalisation. |
| BR-AST-05 | Prices are point-in-time observations, not guarantees; the product must always show the observation time. |
| BR-AST-06 | An unpriced asset must be visible in the holdings view and excluded from the priced total, with a clear indicator. |
| BR-AST-07 | Stablecoins are valued at their market price, not at a hard-coded $1. |
| BR-AST-08 | The canonical catalogue is shared across users; only Admins may change contract/token registry data. |
| BR-AST-09 | Price data must not be edited manually to override market data without an audit record. |

## 8. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-AST-01 | Asset ID / symbol | Canonical identity | Internal |
| DR-AST-02 | Name, type, rank | Descriptive and prioritisation data | Internal |
| DR-AST-03 | Chain and contract address | On-chain mapping | Internal |
| DR-AST-04 | Decimals | Precision for normalisation | Internal |
| DR-AST-05 | Price, volume, change | Market observations | Internal |
| DR-AST-06 | Price source and timestamp | Auditability and freshness | Internal |
| DR-AST-07 | Unmapped asset record | Tokens that could not be resolved | Internal |

## 9. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-AST-01 | Given a new asset, when added to the catalogue, then its symbol is unique and its record is available for holdings resolution. |
| AC-AST-02 | Given a token contract on an enabled chain, when a wallet is synced, then the balance resolves to the canonical asset and the correct decimals are applied. |
| AC-AST-03 | Given an unregistered token, when a wallet is synced, then the holding is marked unmapped and excluded from the valued total. |
| AC-AST-04 | Given a market-data outage, when valuation runs, then the last known price is used and its timestamp is displayed. |
| AC-AST-05 | Given a valuation, when a user inspects it, then the price source and observation time are available. |
| AC-AST-06 | Given duplicate contract registration, when an Admin attempts to save it, then the change is rejected with a clear error. |

## 10. Assumptions, dependencies, constraints

- The exchange market-data API is rate-limited; a shared request budget applies to collection jobs.
- Asset metadata is refreshed daily; prices are refreshed more frequently (BRD-011).
- The system is USD-denominated in the MVP.
- Exchange data may contain inconsistencies; the product must prefer deterministic, recorded mappings over heuristics.

## 11. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Which tracked assets lack a supported exchange pair at launch, and how are they surfaced? | Data |
| OQ-002 | How long should price history be retained? | Product |
| OQ-003 | What is the policy for delisted or renamed assets? | Data |
| OQ-004 | Should users be able to manually price an unmapped asset? | Product |
| OQ-005 | What constitutes "stale" price for display and alerting? | Product |
