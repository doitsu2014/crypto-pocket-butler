# Job Runner Framework

This module provides a common framework for running background jobs with consistent logging, timing, and metrics collection.

## Overview

The job runner framework ensures:
- **Consistent execution**: All jobs follow the same execution pattern
- **Timing**: Automatic measurement of job execution time
- **Logging**: Structured logging of job start, completion, and errors
- **Metrics**: Standard metrics for items processed, created, updated, and skipped
- **Error handling**: Graceful error handling with detailed logging
- **Idempotency**: Database operations use ON CONFLICT to prevent duplicates

## Architecture

```
JobRunner
  └─> execute(job_fn)
       ├─> Measure timing
       ├─> Log start/completion
       └─> Return JobResult
            ├─> success: bool
            ├─> job_name: String
            ├─> duration_ms: u64
            ├─> started_at: DateTime<Utc>
            ├─> completed_at: DateTime<Utc>
            ├─> metrics: JobMetrics
            │    ├─> items_processed: usize
            │    ├─> items_created: usize
            │    ├─> items_updated: usize
            │    ├─> items_skipped: usize
            │    └─> custom: serde_json::Value
            └─> error: Option<String>
```

## Usage

### Basic Usage

```rust
use crate::jobs::runner::{JobRunner, JobMetrics};

async fn my_job(db: &DatabaseConnection) -> Result<(), Box<dyn Error + Send + Sync>> {
    let runner = JobRunner::new("my_job");
    
    let result = runner.execute(|| async {
        // Perform job work
        let items_processed = 100;
        let items_created = 50;
        
        Ok(JobMetrics {
            items_processed,
            items_created,
            items_updated: 0,
            items_skipped: 0,
            custom: serde_json::json!({
                "additional_metric": "value",
            }),
        })
    }).await;
    
    if !result.success {
        return Err(result.error.unwrap_or("Unknown error".to_string()).into());
    }
    
    Ok(())
}
```

### With Error Handling

```rust
let runner = JobRunner::new("data_sync_job");

let result = runner.execute(|| async {
    // Fetch data from API
    let data = fetch_data().await
        .map_err(|e| format!("Failed to fetch data: {}", e))?;
    
    // Process and store data
    let stored = store_data(db, &data).await
        .map_err(|e| format!("Failed to store data: {}", e))?;
    
    Ok(JobMetrics {
        items_processed: data.len(),
        items_created: stored,
        items_updated: 0,
        items_skipped: 0,
        custom: serde_json::json!({}),
    })
}).await;

// Result contains detailed metrics and timing information
println!("Job completed in {} ms", result.duration_ms);
```

## Idempotency Guarantees

All jobs in this module are designed to be idempotent, meaning they can be safely re-run without creating duplicate data. This is achieved through:

### 1. Database Constraints

The database schema includes unique constraints on natural keys:

- **asset_rankings**: `UNIQUE (asset_id, snapshot_date, source)`
- **asset_prices**: `UNIQUE (asset_id, timestamp, source)`
- **asset_contracts**: `UNIQUE (chain, contract_address)`

### 2. ON CONFLICT Clauses

Instead of manual duplicate checking, jobs use PostgreSQL's `ON CONFLICT` clause:

```rust
use sea_orm::{Insert, sea_query::OnConflict};

Insert::one(new_record)
    .on_conflict(
        OnConflict::columns([
            TableColumn::AssetId,
            TableColumn::SnapshotDate,
            TableColumn::Source,
        ])
        .update_columns([
            TableColumn::Rank,
            TableColumn::MarketCapUsd,
            TableColumn::PriceUsd,
        ])
        .to_owned(),
    )
    .exec(db)
    .await?;
```

This approach:
- ✅ Prevents duplicate records
- ✅ Updates existing records with latest data
- ✅ Is atomic (no race conditions)
- ✅ Is more efficient than SELECT + INSERT/UPDATE

## Available Jobs

### 1. Top Coins Collection (`top_coins_collection.rs`)

Fetches top N coins by market cap from CoinGecko.

**Idempotency**: ON CONFLICT on `(asset_id, snapshot_date, source)` for rankings
**Schedule**: Daily at 00:00 UTC
**Metrics**:
- `coins_collected`: Number of coins fetched from API
- `assets_created`: New asset records created
- `assets_updated`: Existing asset records updated
- `rankings_created`: Ranking records upserted

### 2. Price Collection (`price_collection.rs`)

Collects current prices for tracked assets.

**Idempotency**: ON CONFLICT on `(asset_id, timestamp, source)` with timestamp rounded to nearest minute
**Schedule**: Every 15 minutes
**Metrics**:
- `assets_tracked`: Number of unique assets tracked
- `prices_collected`: Number of prices fetched from API
- `prices_stored`: Number of prices upserted to database

### 3. Contract Addresses Collection (`contract_addresses_collection.rs`)

Fetches smart contract addresses for assets across multiple chains.

**Idempotency**: ON CONFLICT on `(chain, contract_address)`
**Schedule**: Daily at 01:00 UTC
**Metrics**:
- `assets_processed`: Number of assets processed
- `contracts_created`: Number of contract records upserted
- `assets_skipped`: Number of assets skipped (errors, no coingecko_id)

## Testing

### Unit Tests

The framework includes comprehensive unit tests:

```bash
cargo test --lib jobs::runner
```

### Integration Tests

To test idempotency, run a job multiple times:

```bash
# First run
curl -X POST http://localhost:3001/api/v1/jobs/collect-top-coins \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"limit": 10}'

# Second run (should not create duplicates)
curl -X POST http://localhost:3001/api/v1/jobs/collect-top-coins \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"limit": 10}'

# Verify no duplicates in database
psql -d crypto_pocket_butler -c "
  SELECT asset_id, snapshot_date, source, COUNT(*) as count
  FROM asset_rankings
  GROUP BY asset_id, snapshot_date, source
  HAVING COUNT(*) > 1;
"
```

## Configuration

Jobs are configured via environment variables:

```bash
# Top Coins Collection
TOP_COINS_COLLECTION_ENABLED=true
TOP_COINS_COLLECTION_SCHEDULE="0 0 0 * * *"  # Daily at 00:00 UTC
TOP_COINS_COLLECTION_LIMIT=100

# Price Collection
PRICE_COLLECTION_ENABLED=true
PRICE_COLLECTION_SCHEDULE="0 */15 * * * *"  # Every 15 minutes
PRICE_COLLECTION_LIMIT=100

# Contract Addresses Collection
CONTRACT_ADDRESSES_COLLECTION_ENABLED=true
CONTRACT_ADDRESSES_COLLECTION_SCHEDULE="0 0 1 * * *"  # Daily at 01:00 UTC
CONTRACT_ADDRESSES_COLLECTION_LIMIT=
```

## Monitoring

Job execution is logged with structured logging:

```
INFO  Starting job: top_coins_collection(limit=100)
DEBUG Updated asset: BTC (uuid)
DEBUG Upserted ranking: BTC (rank 1) on 2024-01-15
INFO  Job 'top_coins_collection(limit=100)' completed successfully: 100 processed, 0 created, 100 updated, 0 skipped
INFO  Job 'top_coins_collection(limit=100)' execution time: 5432 ms
```

## Best Practices

1. **Always use the JobRunner framework** for new jobs
2. **Design for idempotency** from the start
3. **Use database constraints** to enforce natural keys
4. **Prefer ON CONFLICT over manual checks** for better performance
5. **Include custom metrics** for job-specific data
6. **Log errors with context** to aid debugging
7. **Test idempotency** by running jobs multiple times
8. **Document natural keys** in migration comments

## Future Improvements

- [ ] Add job scheduling history table
- [ ] Implement job retry logic with exponential backoff
- [ ] Add job dependency management
- [ ] Create dashboard for job monitoring
- [ ] Add alerting for job failures
- [ ] Implement job concurrency limits
