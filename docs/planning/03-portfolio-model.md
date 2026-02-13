# 03 — Portfolio Model (Schema)

## Core entities

- **User**
  - `user_id` = Keycloak `sub` (string)
  - optional: `username`, `email`

- **Account** (data source)
  - `account_id` (uuid)
  - `user_id`
  - `type`: `wallet` | `exchange`
  - `name`
  - identifiers:
    - wallet: `address`, `chain`
    - exchange: `exchange_name`, `account_ref`

- **Asset**
  - `asset_id` (uuid)
  - `symbol`
  - `network`
  - `contract` (nullable)
  - `decimals`
  - tags (category/sector)

- **Holding** (latest state per account)
  - `account_id`, `asset_id`
  - `qty`
  - `price`
  - `value_usd`
  - `as_of` timestamp

## Portfolio entities (user-configurable)

- **Portfolio**
  - `portfolio_id` (uuid)
  - `user_id`
  - `name`
  - `base_currency` (USD)
  - `created_at`

- **PortfolioAccount** (join)
  - `portfolio_id`
  - `account_id`

## Snapshots (EOD)

- **PortfolioSnapshot**
  - `snapshot_id` (uuid)
  - `portfolio_id`
  - `snapshot_date` (date)
  - `snapshot_time` (timestamp)
  - `total_value_usd`

- **PortfolioSnapshotHolding**
  - `snapshot_id`
  - `asset_id`
  - `qty`
  - `price`
  - `value_usd`

## Normalization rules

1. Base currency is **USD**.
2. Map symbols + networks to a canonical asset id.
3. Use consistent pricing source + timestamp for each snapshot.

## Outputs

- Allocation tables: by asset / by account / by tag.
- Risk metrics: concentration, stablecoin %, drawdown proxy.
- Snapshot series: portfolio value over time.

## Portfolio Allocation (latest-only)

- **PortfolioAllocation**
  - `portfolio_id` (unique)
  - `as_of` (timestamp)
  - `total_value_usd`
  - `allocation` (JSON: assets with qty, price, value, weight)
