# Database Schema Documentation

This document describes the PostgreSQL database schema for the Crypto Pocket Butler application using SeaORM.

## Overview

The database uses a normalized schema with the following main entities:

**User Portfolio Management:**
- **Users**: Keycloak-authenticated users
- **Accounts**: Exchange accounts, wallets, or DeFi protocols
- **Portfolios**: User-defined groupings of accounts
- **Portfolio_Accounts**: Many-to-many join table
- **Snapshots**: Point-in-time portfolio snapshots (EOD, manual, etc.)

**Market Reference Data:**
- **Assets**: Crypto asset metadata (symbols, names, types)
- **Asset_Contracts**: Chain-specific contract addresses
- **Asset_Prices**: Time-series price data for valuation
- **Asset_Rankings**: Top-100 market cap ranking snapshots

## Entity Relationship Diagram

```
┌─────────────┐
│   Users     │
│─────────────│
│ id (PK)     │◄──┐
│ keycloak_id │   │
│ email       │   │
│ username    │   │
└─────────────┘   │
                  │
        ┌─────────┴──────────────┐
        │                        │
┌───────┴───────┐       ┌────────┴──────┐
│   Accounts    │       │  Portfolios   │
│───────────────│       │───────────────│
│ id (PK)       │◄──┐   │ id (PK)       │◄──┐
│ user_id (FK)  │   │   │ user_id (FK)  │   │
│ name          │   │   │ name          │   │
│ account_type  │   │   │ is_default    │   │
│ exchange_name │   │   └───────────────┘   │
│ credentials   │   │           ▲            │
└───────────────┘   │           │            │
                    │   ┌───────┴───────┐    │
                    │   │Portfolio_Accts│    │
                    │   │───────────────│    │
                    └───┤ account_id(FK)│    │
                        │ portfolio_id  ├────┘
                        │  (FK)         │
                        └───────────────┘
                                ▲
                                │
                        ┌───────┴───────┐
                        │  Snapshots    │
                        │───────────────│
                        │ id (PK)       │
                        │ portfolio_id  │
                        │  (FK)         │
                        │ snapshot_date │
                        │ snapshot_type │
                        │ total_value   │
                        │ holdings(JSON)│
                        └───────────────┘

┌──────────────────┐
│   Assets         │     Market Reference Data
│──────────────────│     (Powers valuation & allocation)
│ id (PK)          │
│ symbol (UNIQUE)  │
│ name             │
│ asset_type       │
│ coingecko_id     │
└────────┬─────────┘
         │
         ├──────────────┐──────────────┐
         │              │              │
         ▼              ▼              ▼
┌────────┴────────┐ ┌──┴───────────┐ ┌┴──────────────┐
│AssetContracts   │ │AssetPrices   │ │AssetRankings  │
│─────────────────│ │──────────────│ │───────────────│
│ id (PK)         │ │ id (PK)      │ │ id (PK)       │
│ asset_id (FK)   │ │ asset_id(FK) │ │ asset_id (FK) │
│ chain           │ │ timestamp    │ │ snapshot_date │
│ contract_addr   │ │ price_usd    │ │ rank          │
│ token_standard  │ │ volume_24h   │ │ market_cap    │
└─────────────────┘ │ market_cap   │ │ price_usd     │
                    │ source       │ │ dominance     │
                    └──────────────┘ └───────────────┘
```

## Tables

### users

User accounts linked to Keycloak authentication.

| Column              | Type        | Constraints           | Description                    |
|---------------------|-------------|-----------------------|--------------------------------|
| id                  | UUID        | PRIMARY KEY           | Auto-generated UUID            |
| keycloak_user_id    | VARCHAR     | UNIQUE, NOT NULL      | Keycloak user ID (sub claim)   |
| email               | VARCHAR     | NULL                  | User email                     |
| preferred_username  | VARCHAR     | NULL                  | Preferred username             |
| created_at          | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp      |
| updated_at          | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Last update timestamp          |

**Indexes:**
- `idx_users_keycloak_user_id` on `keycloak_user_id`

### accounts

Exchange accounts, wallets, or DeFi protocol connections.

| Column                 | Type        | Constraints           | Description                       |
|------------------------|-------------|-----------------------|-----------------------------------|
| id                     | UUID        | PRIMARY KEY           | Auto-generated UUID               |
| user_id                | UUID        | NOT NULL, FK          | References users.id               |
| name                   | VARCHAR     | NOT NULL              | User-defined account name         |
| account_type           | VARCHAR     | NOT NULL              | "exchange", "wallet", "defi"      |
| exchange_name          | VARCHAR     | NULL                  | e.g., "okx", "binance"            |
| api_key_encrypted      | TEXT        | NULL                  | Encrypted API key                 |
| api_secret_encrypted   | TEXT        | NULL                  | Encrypted API secret              |
| passphrase_encrypted   | TEXT        | NULL                  | Encrypted passphrase              |
| wallet_address         | VARCHAR     | NULL                  | Wallet address (for wallet type)  |
| is_active              | BOOLEAN     | NOT NULL, DEFAULT true| Whether account is active         |
| last_synced_at         | TIMESTAMPTZ | NULL                  | Last successful sync              |
| created_at             | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp         |
| updated_at             | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Last update timestamp             |

**Foreign Keys:**
- `fk_accounts_user_id`: `user_id` → `users.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_accounts_user_id` on `user_id`
- `idx_accounts_account_type` on `account_type`

**Security Note:** API credentials should be encrypted at rest using a secure encryption key management system.

### portfolios

User-defined portfolio groupings to organize accounts.

| Column      | Type        | Constraints           | Description                    |
|-------------|-------------|-----------------------|--------------------------------|
| id          | UUID        | PRIMARY KEY           | Auto-generated UUID            |
| user_id     | UUID        | NOT NULL, FK          | References users.id            |
| name        | VARCHAR     | NOT NULL              | Portfolio name                 |
| description | TEXT        | NULL                  | Optional description           |
| is_default  | BOOLEAN     | NOT NULL, DEFAULT false| One default portfolio per user |
| created_at  | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp      |
| updated_at  | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Last update timestamp          |

**Foreign Keys:**
- `fk_portfolios_user_id`: `user_id` → `users.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_portfolios_user_id` on `user_id`
- `idx_portfolios_user_id_is_default` on `user_id WHERE is_default = true` (UNIQUE, partial) - Ensures only one default portfolio per user while allowing multiple non-default portfolios

### portfolio_accounts

Join table for many-to-many relationship between portfolios and accounts.

| Column       | Type        | Constraints           | Description                 |
|--------------|-------------|-----------------------|-----------------------------|
| id           | UUID        | PRIMARY KEY           | Auto-generated UUID         |
| portfolio_id | UUID        | NOT NULL, FK          | References portfolios.id    |
| account_id   | UUID        | NOT NULL, FK          | References accounts.id      |
| added_at     | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | When account was added      |

**Foreign Keys:**
- `fk_portfolio_accounts_portfolio_id`: `portfolio_id` → `portfolios.id` (CASCADE on DELETE/UPDATE)
- `fk_portfolio_accounts_account_id`: `account_id` → `accounts.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_portfolio_accounts_unique` on `(portfolio_id, account_id)` (UNIQUE) - Prevents duplicate associations
- `idx_portfolio_accounts_portfolio_id` on `portfolio_id`
- `idx_portfolio_accounts_account_id` on `account_id`

### snapshots

Point-in-time portfolio value snapshots for historical tracking and analysis.

| Column         | Type        | Constraints           | Description                       |
|----------------|-------------|-----------------------|-----------------------------------|
| id             | UUID        | PRIMARY KEY           | Auto-generated UUID               |
| portfolio_id   | UUID        | NOT NULL, FK          | References portfolios.id          |
| snapshot_date  | DATE        | NOT NULL              | Date of snapshot                  |
| snapshot_type  | VARCHAR     | NOT NULL              | "eod", "manual", "hourly"         |
| total_value_usd| DECIMAL     | NOT NULL              | Total portfolio value in USD      |
| holdings       | JSON        | NOT NULL              | Array of asset holdings           |
| metadata       | JSON        | NULL                  | Exchange rates, prices, etc.      |
| created_at     | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp         |

**Foreign Keys:**
- `fk_snapshots_portfolio_id`: `portfolio_id` → `portfolios.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_snapshots_portfolio_id` on `portfolio_id`
- `idx_snapshots_snapshot_date` on `snapshot_date` - For time-series queries
- `idx_snapshots_unique` on `(portfolio_id, snapshot_date, snapshot_type)` (UNIQUE) - Prevents duplicate snapshots

**Holdings JSON Structure:**
```json
[
  {
    "asset": "BTC",
    "quantity": "1.5",
    "price_usd": "50000.00",
    "value_usd": "75000.00",
    "account_id": "uuid",
    "account_name": "My Exchange"
  },
  ...
]
```

**Metadata JSON Structure:**
```json
{
  "exchange_rates": {
    "BTC/USD": "50000.00",
    "ETH/USD": "3000.00"
  },
  "snapshot_version": "1.0",
  "sync_errors": []
}
```

## Migration Management

### Setup

1. Install PostgreSQL (or use Docker Compose):
```bash
docker-compose up -d
```

2. Set DATABASE_URL:
```bash
export DATABASE_URL="postgres://postgres:postgres@localhost:5432/crypto_pocket_butler"
```

### Running Migrations

```bash
cd migration

# Apply all pending migrations
cargo run -- up

# Rollback last migration
cargo run -- down

# Check migration status
cargo run -- status

# Refresh (down + up last migration)
cargo run -- refresh

# Reset database (down all + up all)
cargo run -- reset
```

### Creating New Migrations

To generate a new migration:
```bash
cd migration
cargo run -- generate MIGRATION_NAME
```

Edit the generated file in `migration/src/` with your schema changes.

## SeaORM Entity Usage

### Basic Queries

```rust
use crypto_pocket_butler_backend::entities::*;
use sea_orm::*;

// Find user by Keycloak ID
let user = users::Entity::find()
    .filter(users::Column::KeycloakUserId.eq("keycloak-id"))
    .one(&db)
    .await?;

// Get all active accounts for a user
let accounts = accounts::Entity::find()
    .filter(accounts::Column::UserId.eq(user_id))
    .filter(accounts::Column::IsActive.eq(true))
    .all(&db)
    .await?;

// Get user's default portfolio
let default_portfolio = portfolios::Entity::find()
    .filter(portfolios::Column::UserId.eq(user_id))
    .filter(portfolios::Column::IsDefault.eq(true))
    .one(&db)
    .await?;
```

### Relationship Queries

```rust
// Get portfolios with their accounts (many-to-many)
let portfolios_with_accounts = portfolios::Entity::find()
    .filter(portfolios::Column::UserId.eq(user_id))
    .find_with_related(accounts::Entity)
    .all(&db)
    .await?;

// Get latest snapshot for a portfolio
let latest_snapshot = snapshots::Entity::find()
    .filter(snapshots::Column::PortfolioId.eq(portfolio_id))
    .order_by_desc(snapshots::Column::SnapshotDate)
    .one(&db)
    .await?;

// Get EOD snapshots for a date range
let snapshots = snapshots::Entity::find()
    .filter(snapshots::Column::PortfolioId.eq(portfolio_id))
    .filter(snapshots::Column::SnapshotType.eq("eod"))
    .filter(snapshots::Column::SnapshotDate.between(start_date, end_date))
    .order_by_asc(snapshots::Column::SnapshotDate)
    .all(&db)
    .await?;
```

### Insert Operations

```rust
use sea_orm::ActiveValue::Set;

// Create a new account
let account = accounts::ActiveModel {
    user_id: Set(user_id),
    name: Set("My OKX Account".to_string()),
    account_type: Set("exchange".to_string()),
    exchange_name: Set(Some("okx".to_string())),
    is_active: Set(true),
    ..Default::default()
};
let result = account.insert(&db).await?;

// Add account to portfolio
let portfolio_account = portfolio_accounts::ActiveModel {
    portfolio_id: Set(portfolio_id),
    account_id: Set(account_id),
    ..Default::default()
};
portfolio_account.insert(&db).await?;

// Create a snapshot
let snapshot = snapshots::ActiveModel {
    portfolio_id: Set(portfolio_id),
    snapshot_date: Set(chrono::Utc::now().date_naive()),
    snapshot_type: Set("eod".to_string()),
    total_value_usd: Set(rust_decimal::Decimal::from(100000)),
    holdings: Set(serde_json::json!([...])),
    ..Default::default()
};
snapshot.insert(&db).await?;
```

## Performance Considerations

1. **Indexes**: All foreign keys and commonly queried columns have indexes
2. **JSON Columns**: Consider using JSONB if doing complex JSON queries
3. **Partitioning**: For large snapshots tables, consider partitioning by date
4. **Archival**: Implement archival strategy for old snapshots

## Security Considerations

1. **Credential Encryption**: Always encrypt API keys/secrets before storing
2. **Row-Level Security**: Consider implementing RLS policies in PostgreSQL
3. **Audit Logging**: Add audit triggers for sensitive operations
4. **Backup**: Regular backups of the database
5. **User Isolation**: Always filter by user_id to prevent data leakage

## Future Enhancements

- Add transaction history table
- Add audit log table
- Implement soft deletes with deleted_at columns
- Add full-text search on account/portfolio names

## Market Reference Data Tables

The following tables provide reference data for crypto assets, contracts, prices, and market rankings. These tables power valuation calculations and allocation construction.

### assets

Metadata for crypto assets (coins, tokens, stablecoins).

| Column              | Type        | Constraints           | Description                            |
|---------------------|-------------|-----------------------|----------------------------------------|
| id                  | UUID        | PRIMARY KEY           | Auto-generated UUID                    |
| symbol              | VARCHAR     | UNIQUE, NOT NULL      | Asset symbol (e.g., "BTC", "ETH")      |
| name                | VARCHAR     | NOT NULL              | Full asset name (e.g., "Bitcoin")      |
| asset_type          | VARCHAR     | NOT NULL              | "cryptocurrency", "token", "stablecoin"|
| coingecko_id        | VARCHAR     | NULL                  | CoinPaprika API identifier (legacy field name) |
| coinmarketcap_id    | VARCHAR     | NULL                  | CoinMarketCap identifier               |
| logo_url            | VARCHAR     | NULL                  | URL to asset logo/icon                 |
| description         | TEXT        | NULL                  | Asset description                      |
| decimals            | INTEGER     | NULL                  | Token decimals (e.g., 18 for ERC20)    |
| is_active           | BOOLEAN     | NOT NULL, DEFAULT true| Whether asset is actively tracked      |
| created_at          | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp              |
| updated_at          | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Last update timestamp                  |

**Indexes:**
- `idx_assets_symbol` on `symbol` (UNIQUE) - Fast lookups and prevent duplicates
- `idx_assets_asset_type` on `asset_type` - Filter by asset type
- `idx_assets_coingecko_id` on `coingecko_id` - API integrations (stores CoinPaprika IDs)

### asset_contracts

Chain-specific contract addresses for assets.

| Column           | Type        | Constraints           | Description                            |
|------------------|-------------|-----------------------|----------------------------------------|
| id               | UUID        | PRIMARY KEY           | Auto-generated UUID                    |
| asset_id         | UUID        | NOT NULL, FK          | References assets.id                   |
| chain            | VARCHAR     | NOT NULL              | Blockchain name (e.g., "ethereum")     |
| contract_address | VARCHAR     | NOT NULL              | Contract address on the chain          |
| token_standard   | VARCHAR     | NULL                  | e.g., "ERC20", "BEP20", "ERC721"       |
| decimals         | INTEGER     | NULL                  | Token decimals (overrides asset)       |
| is_verified      | BOOLEAN     | NOT NULL, DEFAULT false| Whether contract is verified          |
| created_at       | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp              |
| updated_at       | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Last update timestamp                  |

**Foreign Keys:**
- `fk_asset_contracts_asset_id`: `asset_id` → `assets.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_asset_contracts_asset_id` on `asset_id` - Find contracts by asset
- `idx_asset_contracts_chain` on `chain` - Filter by blockchain
- `idx_asset_contracts_unique` on `(chain, contract_address)` (UNIQUE) - Prevent duplicates

### asset_prices

Comprehensive time-series market data for assets. Consolidated table storing price, volume, market cap, rank, supply metrics, and ATH information.

**Note**: As of Feb 2024, this table has been enhanced to include ranking and supply data, consolidating functionality previously split between `asset_prices` and `asset_rankings`. See [COINPAPRIKA_REFACTORING.md](./COINPAPRIKA_REFACTORING.md) for details.

| Column                | Type        | Constraints           | Description                                |
|-----------------------|-------------|-----------------------|--------------------------------------------|
| id                    | UUID        | PRIMARY KEY           | Auto-generated UUID                        |
| asset_id              | UUID        | NOT NULL, FK          | References assets.id                       |
| timestamp             | TIMESTAMPTZ | NOT NULL              | Time of price snapshot                     |
| price_usd             | DECIMAL     | NOT NULL              | Spot price in USD                          |
| volume_24h_usd        | DECIMAL     | NULL                  | 24-hour trading volume in USD              |
| market_cap_usd        | DECIMAL     | NULL                  | Market capitalization in USD               |
| change_percent_24h    | DECIMAL     | NULL                  | 24-hour price change percentage            |
| source                | VARCHAR     | NOT NULL              | Data source (e.g., "coinpaprika")          |
| created_at            | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp                  |
| **rank**              | INTEGER     | NULL                  | Market cap rank (e.g., 1 for Bitcoin)      |
| **circulating_supply**| DECIMAL     | NULL                  | Circulating supply                         |
| **total_supply**      | DECIMAL     | NULL                  | Total supply                               |
| **max_supply**        | DECIMAL     | NULL                  | Maximum supply                             |
| **beta_value**        | DECIMAL     | NULL                  | Beta value (volatility metric)             |
| **percent_change_1h** | DECIMAL     | NULL                  | 1-hour price change percentage             |
| **percent_change_7d** | DECIMAL     | NULL                  | 7-day price change percentage              |
| **percent_change_30d**| DECIMAL     | NULL                  | 30-day price change percentage             |
| **ath_price**         | DECIMAL     | NULL                  | All-time high price                        |
| **ath_date**          | TIMESTAMPTZ | NULL                  | Date of all-time high                      |
| **percent_from_price_ath** | DECIMAL | NULL                  | Percentage from ATH                        |

**Foreign Keys:**
- `fk_asset_prices_asset_id`: `asset_id` → `assets.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_asset_prices_asset_id` on `asset_id` - Find prices by asset
- `idx_asset_prices_timestamp` on `timestamp` - Time-series queries
- `idx_asset_prices_asset_timestamp` on `(asset_id, timestamp)` - Efficient per-asset queries
- `idx_asset_prices_unique` on `(asset_id, timestamp, source)` (UNIQUE) - Prevent duplicates
- `idx_asset_prices_rank` on `rank` - Top-N ranking queries **(New)**
- `idx_asset_prices_timestamp_rank` on `(timestamp, rank)` - Date-based ranking queries **(New)**

### asset_rankings

**Note**: As of Feb 2024, this table's functionality has been consolidated into `asset_prices`. It remains for backward compatibility but is no longer actively populated by the unified price collection job. See [COINPAPRIKA_REFACTORING.md](./COINPAPRIKA_REFACTORING.md) for migration details.

Historical top-100 ranking snapshots (**Deprecated** - use `asset_prices.rank` instead).

| Column            | Type        | Constraints           | Description                            |
|-------------------|-------------|-----------------------|----------------------------------------|
| id                | UUID        | PRIMARY KEY           | Auto-generated UUID                    |
| asset_id          | UUID        | NOT NULL, FK          | References assets.id                   |
| snapshot_date     | DATE        | NOT NULL              | Date of ranking snapshot               |
| rank              | INTEGER     | NOT NULL              | Market cap rank (1-100+)               |
| market_cap_usd    | DECIMAL     | NOT NULL              | Market cap at snapshot time            |
| price_usd         | DECIMAL     | NOT NULL              | Price at snapshot time                 |
| volume_24h_usd    | DECIMAL     | NULL                  | 24-hour volume at snapshot time        |
| change_percent_24h| DECIMAL     | NULL                  | 24-hour change at snapshot time        |
| dominance         | DECIMAL     | NULL                  | Market dominance percentage            |
| source            | VARCHAR     | NOT NULL              | Data source (e.g., "coinpaprika")      |
| created_at        | TIMESTAMPTZ | NOT NULL, DEFAULT NOW | Record creation timestamp              |

**Foreign Keys:**
- `fk_asset_rankings_asset_id`: `asset_id` → `assets.id` (CASCADE on DELETE/UPDATE)

**Indexes:**
- `idx_asset_rankings_asset_id` on `asset_id` - Find rankings by asset
- `idx_asset_rankings_snapshot_date` on `snapshot_date` - Time-series queries
- `idx_asset_rankings_rank` on `rank` - Top-N queries
- `idx_asset_rankings_date_rank` on `(snapshot_date, rank)` - Top assets on specific dates
- `idx_asset_rankings_unique` on `(asset_id, snapshot_date, source)` (UNIQUE) - Prevent duplicates

## Usage Examples - Market Reference Data

### Query Current Asset Price

```rust
use crypto_pocket_butler_backend::entities::*;
use sea_orm::*;

// Get latest price for an asset
let btc_asset = assets::Entity::find()
    .filter(assets::Column::Symbol.eq("BTC"))
    .one(&db)
    .await?;

if let Some(asset) = btc_asset {
    let latest_price = asset_prices::Entity::find()
        .filter(asset_prices::Column::AssetId.eq(asset.id))
        .order_by_desc(asset_prices::Column::Timestamp)
        .one(&db)
        .await?;
}
```

### Query Top-100 Assets by Market Cap

```rust
// Get top 100 assets from latest prices (NEW - Recommended)
let latest_time = chrono::Utc::now();
let top_100 = asset_prices::Entity::find()
    .filter(asset_prices::Column::Timestamp.gte(latest_time - chrono::Duration::hours(1)))
    .filter(asset_prices::Column::Rank.is_not_null())
    .filter(asset_prices::Column::Rank.lte(100))
    .order_by_asc(asset_prices::Column::Rank)
    .find_also_related(assets::Entity)
    .all(&db)
    .await?;

// Alternative: Get top 100 from rankings table (DEPRECATED)
// Use only if you need historical daily snapshots
let top_100_legacy = asset_rankings::Entity::find()
    .filter(asset_rankings::Column::SnapshotDate.eq(date))
    .filter(asset_rankings::Column::Rank.lte(100))
    .order_by_asc(asset_rankings::Column::Rank)
    .find_also_related(assets::Entity)
    .all(&db)
    .await?;
```

### Query Asset Contract Addresses

```rust
// Get all contract addresses for an asset
let asset = assets::Entity::find()
    .filter(assets::Column::Symbol.eq("USDT"))
    .find_with_related(asset_contracts::Entity)
    .all(&db)
    .await?;

// Get specific contract on Ethereum
let eth_contract = asset_contracts::Entity::find()
    .filter(asset_contracts::Column::AssetId.eq(asset_id))
    .filter(asset_contracts::Column::Chain.eq("ethereum"))
    .one(&db)
    .await?;
```

### Query Price History

```rust
// Get price history for last 30 days
let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
let price_history = asset_prices::Entity::find()
    .filter(asset_prices::Column::AssetId.eq(asset_id))
    .filter(asset_prices::Column::Timestamp.gte(thirty_days_ago))
    .order_by_asc(asset_prices::Column::Timestamp)
    .all(&db)
    .await?;
```
