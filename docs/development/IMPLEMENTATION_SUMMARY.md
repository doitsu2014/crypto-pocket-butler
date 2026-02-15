# Job Runner Framework Implementation Summary

## Overview

This implementation adds a common job runner framework with idempotency guarantees for the crypto-pocket-butler backend. The framework ensures that background jobs can be safely re-run without creating duplicate data.

## Changes Made

### 1. Job Runner Framework (`src/jobs/runner.rs`)

**New Module**: Created a reusable job runner framework with the following features:

- **JobRunner**: Main struct that wraps job execution
- **JobResult**: Standard result structure with execution metadata
- **JobMetrics**: Consistent metrics structure for all jobs

**Key Features**:
- ✅ Automatic timing measurement (duration_ms)
- ✅ Structured logging (start, completion, errors)
- ✅ Standard metrics (processed, created, updated, skipped)
- ✅ Custom metrics support via JSON
- ✅ Graceful error handling
- ✅ Comprehensive unit tests

**API**:
```rust
let runner = JobRunner::new("job_name");
let result = runner.execute(|| async {
    // Job logic here
    Ok(JobMetrics { ... })
}).await;
```

### 2. Idempotency Implementation

**Database Constraints** (Already existed in migrations):
- `asset_rankings`: UNIQUE constraint on `(asset_id, snapshot_date, source)`
- `asset_prices`: UNIQUE constraint on `(asset_id, timestamp, source)`
- `asset_contracts`: UNIQUE constraint on `(chain, contract_address)`

**Code Changes**: Replaced manual duplicate checking with PostgreSQL `ON CONFLICT` clauses:

#### Top Coins Collection
- **Before**: SELECT to check existence, then INSERT or UPDATE
- **After**: `INSERT ... ON CONFLICT (asset_id, snapshot_date, source) DO UPDATE`
- **Benefit**: Atomic operation, no race conditions, better performance

#### Price Collection
- **Before**: SELECT to check existence, skip if found
- **After**: `INSERT ... ON CONFLICT (asset_id, timestamp, source) DO UPDATE`
- **Benefit**: Updates existing prices with latest data, prevents duplicates

#### Contract Addresses Collection
- **Before**: SELECT to check existence, then INSERT or UPDATE
- **After**: `INSERT ... ON CONFLICT (chain, contract_address) DO UPDATE`
- **Benefit**: Updates contract metadata when re-run

### 3. Job Refactoring

All three jobs now use the JobRunner framework:

**`top_coins_collection.rs`**:
- Uses JobRunner for execution
- ON CONFLICT for asset_rankings
- Returns standardized CollectionResult

**`price_collection.rs`**:
- Uses JobRunner for execution
- ON CONFLICT for asset_prices
- Timestamp rounding to nearest minute for consistent time buckets

**`contract_addresses_collection.rs`**:
- Uses JobRunner for execution
- ON CONFLICT for asset_contracts
- Rate limiting (1.5s delay) to respect API limits

### 4. Documentation

**`src/jobs/README.md`**: Comprehensive documentation including:
- Framework architecture
- Usage examples
- Idempotency guarantees
- Available jobs with their configurations
- Testing instructions
- Best practices

## Testing

### Unit Tests
- ✅ `JobRunner::execute` success case
- ✅ `JobRunner::execute` failure case
- ✅ All 22 library tests pass

### Idempotency Verification
The implementation ensures idempotency through:
1. **Database constraints**: Unique indexes prevent duplicates at DB level
2. **ON CONFLICT clauses**: Atomic upsert operations
3. **Consistent time buckets**: Prices rounded to nearest minute

## API Compatibility

**No Breaking Changes**: The job endpoints maintain backward compatibility:
- POST `/api/v1/jobs/collect-top-coins` - Same request/response structure
- POST `/api/v1/jobs/collect-contract-addresses` - Same request/response structure

Internal `CollectionResult` structs remain unchanged for backward compatibility.

## Acceptance Criteria

✅ **Re-running jobs does not duplicate data**
- Database unique constraints prevent duplicates
- ON CONFLICT clauses update existing records
- All three jobs (top100, prices, contracts) are idempotent

✅ **Job endpoints return consistent result payload**
- JobRunner provides standardized execution flow
- All jobs return consistent metrics structure
- Backward compatible with existing endpoint contracts

## Performance Improvements

1. **Reduced Database Queries**: ON CONFLICT eliminates SELECT before INSERT
2. **Atomic Operations**: No race conditions between SELECT and INSERT
3. **Single Round Trip**: Upsert happens in one database operation

## Code Quality

- ✅ No compilation warnings
- ✅ All tests passing
- ✅ Follows Rust best practices
- ✅ Comprehensive documentation
- ✅ Type-safe implementation

## Future Enhancements

The framework is designed to support:
- Job scheduling history
- Retry logic with exponential backoff
- Job dependency management
- Monitoring dashboard
- Alerting for failures
- Concurrency limits

## Migration Path

No database migrations required. The implementation uses existing unique constraints that were already in place in the schema.

## How to Use

### For New Jobs
```rust
use crate::jobs::runner::{JobRunner, JobMetrics};

pub async fn my_new_job(db: &DatabaseConnection) -> Result<MyResult, Error> {
    let runner = JobRunner::new("my_new_job");
    
    let result = runner.execute(|| async {
        // Your job logic
        Ok(JobMetrics { ... })
    }).await;
    
    // Convert to your result type
    Ok(MyResult::from(result))
}
```

### Testing Idempotency
```bash
# Run job twice
curl -X POST http://localhost:3001/api/v1/jobs/collect-top-coins \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"limit": 10}'

# Verify no duplicates
psql -c "SELECT asset_id, snapshot_date, source, COUNT(*) 
         FROM asset_rankings 
         GROUP BY asset_id, snapshot_date, source 
         HAVING COUNT(*) > 1;"
# Should return 0 rows
```
