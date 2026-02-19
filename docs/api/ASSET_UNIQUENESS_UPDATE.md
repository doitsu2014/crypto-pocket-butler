# Asset Uniqueness Constraint Update

**Date:** 2026-02-19  
**Migration:** `m20260219_000001_symbol_name_uniqueness.rs`  
**Issue:** Update asset detection logic: Combine Symbol and Name for uniqueness

## Background

Previously, the asset detection system relied solely on the **Symbol** field to determine asset uniqueness. This approach had several limitations:

1. **Symbol collisions**: Multiple distinct assets can share the same symbol (e.g., "BTC" could represent Bitcoin, Wrapped Bitcoin, or a Bitcoin-pegged token)
2. **False positives**: Assets with the same symbol but fundamentally different characteristics would be incorrectly treated as the same asset
3. **Data integrity issues**: When fetching market data, assets could be incorrectly merged or updated

## Changes

### Database Schema

**Before:**
```sql
CREATE UNIQUE INDEX idx_assets_symbol ON assets (symbol);
```

**After:**
```sql
CREATE UNIQUE INDEX idx_assets_symbol_name_unique ON assets (symbol, name);
```

This change allows multiple assets to share the same symbol as long as they have different names, while still preventing exact duplicates.

### Migration Details

The migration (`m20260219_000001_symbol_name_uniqueness.rs`) performs the following steps:

1. Drops the old unique index on `symbol` only
2. Creates a new unique index on `(symbol, name)` combination
3. Includes a rollback (down migration) to restore the previous state if needed

### Code Changes

#### 1. Asset Identity Helper (`api/src/helpers/asset_identity.rs`)

**New Method:**
```rust
pub async fn normalize_from_symbol_and_name(
    &self,
    symbol: &str,
    name: &str,
) -> NormalizationResult
```

This method enforces the new uniqueness constraint by checking both symbol AND name fields together.

**Updated Method:**
- `normalize_from_okx()`: Added documentation noting it uses symbol-only lookup (OKX doesn't provide asset names)

#### 2. Price Collection Job (`api/src/jobs/price_collection.rs`)

**Updated Logic:**
```rust
// Before
assets::Entity::find()
    .filter(
        assets::Column::Symbol.eq(&coin.symbol.to_uppercase())
            .or(assets::Column::CoingeckoId.eq(&coin.id))
    )

// After
assets::Entity::find()
    .filter(
        assets::Column::Symbol.eq(&coin.symbol.to_uppercase())
            .and(assets::Column::Name.eq(&coin.name))
            .or(assets::Column::CoingeckoId.eq(&coin.id))
    )
```

#### 3. Fetch All Coins Job (`api/src/jobs/fetch_all_coins.rs`)

**Updated Logic:**
```rust
// Before (fallback)
assets::Entity::find()
    .filter(assets::Column::Symbol.eq(&coin.symbol.to_uppercase()))

// After (fallback)
assets::Entity::find()
    .filter(
        assets::Column::Symbol.eq(&coin.symbol.to_uppercase())
            .and(assets::Column::Name.eq(&coin.name))
    )
```

### Testing

Three new integration tests were added (`api/tests/asset_uniqueness_test.rs`):

1. **`test_same_symbol_different_names`**: Verifies that assets with the same symbol but different names can coexist
2. **`test_same_symbol_and_name_fails`**: Verifies that assets with the same symbol AND name cannot be created (unique constraint violation)
3. **`test_normalize_from_symbol_and_name`**: Tests the new normalization method to ensure it correctly differentiates assets

## Backward Compatibility

### Considerations

1. **Existing Data**: The migration will succeed if there are no existing assets with duplicate (symbol, name) combinations. If duplicates exist, the migration will fail and manual data cleanup will be required.

2. **API Integrations**: External systems that rely on symbol-only lookups will continue to work, but may return only the first matching asset if multiple assets share a symbol.

3. **CoinPaprika ID**: The system still prioritizes CoinPaprika ID (stored in `coingecko_id` field) for asset identification, with (symbol, name) used as a fallback. This provides continuity for existing assets.

### Migration Path

If you have existing data with duplicate (symbol, name) combinations:

1. Identify duplicates:
   ```sql
   SELECT symbol, name, COUNT(*) 
   FROM assets 
   GROUP BY symbol, name 
   HAVING COUNT(*) > 1;
   ```

2. Review and deduplicate manually before running the migration

3. Run the migration:
   ```bash
   cd api
   cargo run --bin migration up
   ```

## Impact

### Positive Changes

✅ **Improved accuracy**: Assets are now uniquely identified by both symbol and name  
✅ **Better data integrity**: Prevents false merging of distinct assets  
✅ **Flexible symbol usage**: Multiple assets can share a symbol (e.g., wrapped tokens)  
✅ **Robust fallback**: CoinPaprika ID still serves as primary identifier

### Potential Issues

⚠️ **Existing duplicates**: Migration will fail if duplicate (symbol, name) combinations exist  
⚠️ **Symbol-only queries**: Code using symbol-only lookups may return unexpected results  
⚠️ **OKX integration**: OKX API doesn't provide asset names, so OKX normalization still uses symbol-only

## Examples

### Valid Asset Combinations (Post-Migration)

| Symbol | Name                | Allowed? |
|--------|---------------------|----------|
| BTC    | Bitcoin             | ✅       |
| BTC    | Wrapped Bitcoin     | ✅       |
| ETH    | Ethereum            | ✅       |
| ETH    | Wrapped Ethereum    | ✅       |

### Invalid Asset Combinations (Post-Migration)

| Symbol | Name                | Allowed? |
|--------|---------------------|----------|
| BTC    | Bitcoin             | ✅ (first)|
| BTC    | Bitcoin             | ❌ (duplicate)|

## Rollback

If you need to rollback this change:

```bash
cd api
cargo run --bin migration down
```

This will:
1. Drop the composite unique index on (symbol, name)
2. Restore the unique index on symbol only

**Note:** Rollback will fail if you have created assets with duplicate symbols (but different names) after applying this migration.

## References

- Issue: Update asset detection logic: Combine Symbol and Name for uniqueness
- Migration: `api/migration/src/m20260219_000001_symbol_name_uniqueness.rs`
- Tests: `api/tests/asset_uniqueness_test.rs`
- Asset Entity: `api/src/entities/assets.rs`
- Asset Identity Helper: `api/src/helpers/asset_identity.rs`
