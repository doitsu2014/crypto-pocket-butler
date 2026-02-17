# CoinPaprika API Refactoring

## Overview

This document describes the refactoring of asset data collection to use CoinPaprika's unified `/tickers` API endpoint for comprehensive market data retrieval.

## Motivation

The previous implementation had several inefficiencies:

1. **Data Duplication**: `asset_prices` and `asset_rankings` stored overlapping data (price, market_cap, volume)
2. **Multiple API Calls**: Separate jobs made redundant API calls to fetch similar data
3. **Incomplete Data**: Not all available CoinPaprika fields were captured
4. **Job Complexity**: Two separate jobs (`top_coins_collection` and `price_collection`) with overlapping responsibilities

## Changes Made

### 1. Database Schema Consolidation

**Added fields to `asset_prices` table** (Migration: `m20240217_000001_refactor_assets_coinpaprika`):

```sql
-- Market rank
rank INTEGER

-- Supply information
circulating_supply DECIMAL
total_supply DECIMAL
max_supply DECIMAL

-- Market metrics
beta_value DECIMAL
percent_change_1h DECIMAL
percent_change_7d DECIMAL
percent_change_30d DECIMAL

-- All-Time High (ATH) information
ath_price DECIMAL
ath_date TIMESTAMP WITH TIME ZONE
percent_from_price_ath DECIMAL
```

**Benefits**:
- Single table now contains all market data per timestamp
- Eliminates need for `asset_rankings` table (kept for backward compatibility)
- More efficient queries (no joins needed)
- Comprehensive historical data

**Indices Added**:
- `idx_asset_prices_rank` - For top-N ranking queries
- `idx_asset_prices_timestamp_rank` - For date-based ranking queries

### 2. Unified Data Collection Job

**`price_collection.rs`** now handles:

1. **Asset Discovery & Management**
   - Fetches top N coins from CoinPaprika
   - Creates new asset records
   - Updates existing asset metadata

2. **Comprehensive Price Collection**
   - Collects data for top N coins
   - Collects data for portfolio holdings
   - Stores all market data (price, rank, supply, ATH, etc.)

**Single API Call Strategy**:
```rust
// Fetch top N coins (pre-sorted by market cap rank)
let top_coins = connector.fetch_top_coins(limit).await?;

// Upsert asset records
for coin in &top_coins {
    upsert_asset(db, coin).await?;
}

// Collect prices for all tracked assets (including portfolio holdings)
let price_data = fetch_prices_for_assets(&connector, &tracked_assets, top_n_limit).await?;

// Store comprehensive price data
store_prices(db, &price_data).await?;
```

**Old Flow (2 jobs)**:
```
top_coins_collection:
  ├─ GET /tickers?limit=100  (fetch top 100)
  ├─ Upsert assets
  └─ Insert asset_rankings

price_collection:
  ├─ GET /tickers?limit=100  (fetch top 100 again!)
  ├─ GET /tickers/{id}       (fetch each portfolio asset)
  └─ Insert asset_prices
```

**New Flow (1 job)**:
```
price_collection:
  ├─ GET /tickers?limit=100  (fetch top 100 once)
  ├─ Upsert assets (creates/updates)
  ├─ GET /tickers/{id}       (fetch only missing portfolio assets)
  └─ Insert asset_prices (with rank and all market data)
```

### 3. Enhanced Data Model

**Updated `CoinMarketData` struct** to capture all CoinPaprika fields:

```rust
pub struct CoinMarketData {
    pub id: String,              // CoinPaprika ID (e.g., "btc-bitcoin")
    pub name: String,
    pub symbol: String,
    pub rank: u32,
    
    // Supply information
    pub circulating_supply: Option<f64>,
    pub total_supply: Option<f64>,
    pub max_supply: Option<f64>,
    pub beta_value: Option<f64>,
    
    // Timestamps
    pub first_data_at: Option<String>,
    pub last_updated: Option<String>,
    
    pub quotes: Quotes {
        usd: UsdQuote {
            // Price and volume
            price: f64,
            volume_24h: Option<f64>,
            market_cap: f64,
            
            // Percent changes
            percent_change_24h: Option<f64>,
            percent_change_1h: Option<f64>,
            percent_change_7d: Option<f64>,
            percent_change_30d: Option<f64>,
            
            // All-Time High
            ath_price: Option<f64>,
            ath_date: Option<String>,
            percent_from_price_ath: Option<f64>,
        }
    }
}
```

## Asset ID Mapping

### Chain-Specific Assets

The system now properly handles chain-specific asset identifiers like `USDC-ETHEREUM`:

**Normalization Flow**:
```rust
// Account holds "USDC-ETHEREUM"
let result = normalizer.normalize_from_symbol("USDC-ETHEREUM").await;

// Flow:
// 1. Detects chain suffix "ETHEREUM" (known EVM chain)
// 2. Extracts base symbol "USDC"
// 3. Looks up in assets table by symbol
// 4. Returns AssetIdentity with canonical ID

// Result: Mapped to USDC asset with CoinPaprika ID "usdc-usd-coin"
```

**Supported Chain Suffixes**:
- `ethereum`
- `arbitrum`
- `optimism`
- `base`
- `bsc` (Binance Smart Chain)

### Contract Address Mapping

For EVM tokens, contract addresses are still supported via `asset_contracts` table:

```rust
// Map contract address to asset
normalizer.normalize_from_evm_contract(
    "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",  // USDC contract
    "ethereum"
).await;
// Returns: USDC asset identity
```

## Migration Guide

### For Existing Deployments

1. **Run Migration**:
   ```bash
   cd api
   cargo run -- migrate up
   ```

2. **Update Environment Variables** (optional):
   ```bash
   # Disable old top_coins_collection job (now handled by price_collection)
   TOP_COINS_COLLECTION_ENABLED=false
   
   # Ensure price_collection is enabled
   PRICE_COLLECTION_ENABLED=true
   PRICE_COLLECTION_SCHEDULE="0 */15 * * * *"  # Every 15 minutes
   PRICE_COLLECTION_LIMIT=100
   ```

3. **No Code Changes Required**:
   - Existing queries continue to work
   - Portfolio construction unchanged
   - Asset normalization backward compatible

### For New Deployments

Simply run migrations and configure:
```bash
PRICE_COLLECTION_ENABLED=true
PRICE_COLLECTION_LIMIT=100  # Or 2000 for CoinPaprika Pro
```

## Benefits Summary

✅ **Performance**:
- Reduced API calls (~50% fewer requests)
- Single batch operation for data storage
- Faster job execution

✅ **Data Completeness**:
- All CoinPaprika fields now captured
- Historical ATH data preserved
- Supply metrics available

✅ **Maintainability**:
- Single job to maintain
- No data duplication
- Clearer data flow

✅ **Scalability**:
- Works with free tier (up to 2000 coins)
- Efficient for Pro tier
- Handles portfolio assets seamlessly

## API Rate Limits

### Free Tier
- 1,000 calls/day
- No authentication required
- Limit: 250 coins per `/tickers` call

### Pro Tier
- Higher rate limits
- Authentication via `COINPAPRIKA_API_KEY`
- Limit: Up to 2000 coins per `/tickers` call

**Configuration**:
```bash
# Optional: Enable Pro API
COINPAPRIKA_API_KEY=your-api-key-here

# Adjust limit based on tier
PRICE_COLLECTION_LIMIT=100   # Free tier
# or
PRICE_COLLECTION_LIMIT=2000  # Pro tier
```

## Backward Compatibility

### Deprecated (but still functional)

- `top_coins_collection` job - Can still run but duplicates work
- `asset_rankings` table - Still exists but not populated by new job

### Query Updates (Optional)

Old queries using `asset_rankings` can be updated to use `asset_prices`:

**Before**:
```sql
SELECT a.symbol, ar.rank, ar.market_cap_usd
FROM asset_rankings ar
JOIN assets a ON a.id = ar.asset_id
WHERE ar.snapshot_date = CURRENT_DATE
ORDER BY ar.rank;
```

**After**:
```sql
SELECT a.symbol, ap.rank, ap.market_cap_usd
FROM asset_prices ap
JOIN assets a ON a.id = ap.asset_id
WHERE DATE(ap.timestamp) = CURRENT_DATE
  AND ap.rank IS NOT NULL
ORDER BY ap.rank;
```

## Testing

### Verify Migration
```bash
cd api
cargo test
```

### Test Price Collection
```bash
# Run job manually
curl -X POST http://localhost:3001/api/v1/jobs/price-collection
```

### Verify Data
```sql
-- Check new fields are populated
SELECT 
    COUNT(*) as total_records,
    COUNT(rank) as with_rank,
    COUNT(circulating_supply) as with_supply,
    COUNT(ath_price) as with_ath
FROM asset_prices
WHERE created_at > NOW() - INTERVAL '1 hour';
```

## References

- [CoinPaprika API Documentation](https://api.coinpaprika.com/)
- [SeaORM Documentation](https://www.sea-ql.org/SeaORM/)
- [Database Schema](./DATABASE_SCHEMA.md)
