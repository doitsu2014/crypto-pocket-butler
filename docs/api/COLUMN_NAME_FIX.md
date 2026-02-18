# Column Name Fix Migration (m20260218_000001)

## Problem

The original database migration (`m20240101_000003_create_assets_system`) used SeaORM's `DeriveIden` trait to create the `asset_prices` table. However, there was a mismatch in how column names were converted:

- **DeriveIden behavior**: Converts `Volume24hUsd` → `volume24h_usd` (treats "24h" as a single word)
- **Entity expectation**: Field `volume_24h_usd` expects database column `volume_24h_usd` (with underscore before "24")

### Affected Columns

1. `volume24h_usd` → `volume_24h_usd`
2. `change_percent24h` → `change_percent_24h`
3. `percent_change1h` → `percent_change_1h` (added in m20240217)
4. `percent_change7d` → `percent_change_7d` (added in m20240217)
5. `percent_change30d` → `percent_change_30d` (added in m20240217)

## Solution

Migration `m20260218_000001_fix_column_names` renames these columns to match what the entity model expects. The migration is designed to be idempotent - if the columns are already correctly named, the ALTER statements will fail silently.

## Impact

This fix resolves the error:
```
column "volume_24h_usd" of relation "asset_prices" does not exist
```

This error was occurring in the `fetch_all_coins` job when attempting to batch insert/update prices with the `on_conflict` clause.

## Testing

To verify the migration works:

1. Run migrations: `cd api/migration && cargo run -- up`
2. Check columns: `psql -d crypto_pocket_butler -c "\d asset_prices"`
3. Verify the job runs: Test the `fetch_all_coins` job

## Future Prevention

When creating new migrations with numeric suffixes in column names (e.g., `24h`, `7d`, `1h`), consider:

1. Using explicit `#[sea_orm(column_name = "...")]` attributes in the entity
2. Or manually specifying column names in migrations with string literals
3. Or using enum variants with underscores: `Volume_24h_Usd` to ensure correct snake_case conversion
