# Integration Tests for fetch_all_coins Job

This directory contains integration tests for the `fetch_all_coins` job that require a database connection.

## Running Integration Tests

Integration tests are marked with `#[ignore]` and require a PostgreSQL database to run.

### Setup

1. Start a PostgreSQL database (e.g., using Docker):
```bash
docker-compose -f api/docker-compose.yml up -d postgres
```

2. Run migrations to set up the schema:
```bash
cd api
cargo run --bin migration
```

3. Set the database URL (or use default):
```bash
export DATABASE_URL=postgres://postgres:postgres@localhost/crypto_pocket_butler
```

### Running Tests

Run all integration tests:
```bash
cd api
cargo test --test fetch_all_coins_integration_test -- --ignored
```

Run a specific integration test:
```bash
cargo test --test fetch_all_coins_integration_test test_batch_insert_with_duplicates -- --ignored
```

Run both unit and integration tests:
```bash
cargo test --lib jobs::fetch_all_coins::tests
cargo test --test fetch_all_coins_integration_test -- --ignored
```

## Test Coverage

### Unit Tests (in `api/src/jobs/fetch_all_coins.rs`)

1. **test_collection_result_creation** - Tests CollectionResult structure
2. **test_deduplicate_prices_no_duplicates** - Tests deduplication with unique prices
3. **test_deduplicate_prices_with_duplicates** - Tests deduplication keeps last duplicate
4. **test_deduplicate_prices_different_sources** - Tests different sources are not deduplicated
5. **test_deduplicate_prices_different_timestamps** - Tests different timestamps are not deduplicated
6. **test_deduplicate_prices_multiple_duplicates** - Tests multiple duplicate groups
7. **test_deduplicate_prices_empty_vec** - Tests empty input
8. **test_batch_insert_deduplication_scenario** - Simulates real-world duplicate scenario
9. **test_large_batch_with_scattered_duplicates** - Tests large batch with scattered duplicates
10. **test_parse_decimal_from_f64_valid** - Tests decimal parsing
11. **test_parse_decimal_from_f64_none** - Tests None handling

### Integration Tests (in `api/tests/fetch_all_coins_integration_test.rs`)

1. **test_batch_insert_with_duplicates** - Tests database insert with deduplicated prices
2. **test_batch_insert_without_duplicates** - Tests database insert with unique prices
3. **test_fetch_all_coins_job_execution** - Smoke test for full job execution

## Understanding the Fix

The fix addresses the PostgreSQL error: "ON CONFLICT DO UPDATE command cannot affect row a second time"

This error occurs when a batch INSERT contains multiple rows with the same unique key. The `deduplicate_prices()` function:
- Groups prices by (asset_id, timestamp, source) - the unique constraint key
- Keeps only the last occurrence of each duplicate
- Returns a deduplicated vector safe for batch INSERT

The deduplication happens before both batch insert points in the job:
1. Every 500 prices (line ~214)
2. Remaining prices at the end (line ~259)
