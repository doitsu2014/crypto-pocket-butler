# Fix for fetch_all_coins Duplicate Row Error

## Problem

The `fetch_all_coins` job was failing with the following PostgreSQL error:

```
Failed to batch store prices: Execution Error: error returned from database: 
ON CONFLICT DO UPDATE command cannot affect row a second time
```

## Root Cause

PostgreSQL raises this error when a batch INSERT statement contains multiple rows with the same unique key combination. The `asset_prices` table has a unique constraint on:
- `asset_id`
- `timestamp`
- `source`

When the `fetch_all_coins` job processed coins from CoinPaprika's API, it would create multiple price records in the batch with identical (asset_id, timestamp, source) values if:
1. The same coin appeared multiple times in the API response
2. Multiple coins were processed in the same second (same timestamp)
3. The asset lookup/creation resulted in the same asset_id being reused

## Solution

Added a `deduplicate_prices()` function that:
1. Uses a HashMap with (asset_id, timestamp_millis, source) as the key
2. Iterates through all prices and keeps only the last occurrence of each duplicate
3. Returns a deduplicated vector that's safe for batch INSERT

The deduplication is applied before both batch insert operations:
- When accumulating 500 prices (line ~215)
- When inserting remaining prices (line ~260)

## Code Changes

### api/src/jobs/fetch_all_coins.rs

**Added deduplication function:**
```rust
pub fn deduplicate_prices(prices: Vec<asset_prices::ActiveModel>) -> Vec<asset_prices::ActiveModel> {
    use std::collections::HashMap;
    
    let mut price_map: HashMap<(Uuid, i64, String), asset_prices::ActiveModel> = HashMap::new();
    
    for price in prices {
        let asset_id = match &price.asset_id {
            ActiveValue::Set(id) => *id,
            _ => continue,
        };
        let timestamp_millis = match &price.timestamp {
            ActiveValue::Set(ts) => ts.timestamp_millis(),
            _ => continue,
        };
        let source = match &price.source {
            ActiveValue::Set(s) => s.clone(),
            _ => continue,
        };
        
        let key = (asset_id, timestamp_millis, source);
        price_map.insert(key, price);
    }
    
    price_map.into_values().collect()
}
```

**Applied deduplication before batch inserts:**
```rust
// Before
let count = prices_to_store.len();
match Insert::many(prices_to_store)

// After
let deduplicated = deduplicate_prices(prices_to_store);
let count = deduplicated.len();
match Insert::many(deduplicated)
```

## Testing

### Unit Tests (11 tests)

All comprehensive unit tests validate the deduplication logic:

1. **test_deduplicate_prices_no_duplicates** - Validates no data loss when no duplicates
2. **test_deduplicate_prices_with_duplicates** - Validates last duplicate is kept
3. **test_deduplicate_prices_different_sources** - Different sources aren't merged
4. **test_deduplicate_prices_different_timestamps** - Different timestamps aren't merged
5. **test_deduplicate_prices_multiple_duplicates** - Multiple duplicate groups handled
6. **test_deduplicate_prices_empty_vec** - Empty input handled correctly
7. **test_batch_insert_deduplication_scenario** - Real-world duplicate scenario
8. **test_large_batch_with_scattered_duplicates** - Large batch with scattered duplicates
9. **test_parse_decimal_from_f64_valid** - Decimal parsing works
10. **test_parse_decimal_from_f64_none** - None values handled
11. **test_collection_result_creation** - Result structure works

Run with:
```bash
cd api
cargo test --lib jobs::fetch_all_coins::tests
```

### Integration Tests (3 tests)

Integration tests validate database operations:

1. **test_batch_insert_with_duplicates** - Validates DB insert with duplicates
2. **test_batch_insert_without_duplicates** - Validates DB insert without duplicates
3. **test_fetch_all_coins_job_execution** - Full job execution smoke test

Run with:
```bash
cd api
export DATABASE_URL=postgres://postgres:postgres@localhost/crypto_pocket_butler
cargo test --test fetch_all_coins_integration_test -- --ignored
```

See [api/tests/README.md](../tests/README.md) for detailed integration test setup instructions.

## Test Results

All 41 unit tests in the library pass:
```
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured
```

## Impact

- **Minimal code changes**: Only added deduplication function and applied it before batch inserts
- **No behavior change**: The job still processes the same coins, just handles duplicates gracefully
- **Performance**: HashMap lookup is O(1), so deduplication adds minimal overhead
- **Safety**: Last duplicate wins, ensuring the most recent data is stored

## Future Considerations

While this fix handles duplicates gracefully, it may be worth investigating:
1. Why duplicates occur in the first place (API issue, race condition, etc.)
2. Whether to log warnings when duplicates are detected
3. Whether to choose first vs last duplicate based on some criteria
