# 🛠️ Refactor Job Runner to Use Apalis

## Overview

This issue tracks the refactoring of the current job system to use [Apalis](https://github.com/rapoth/apalis), while preserving all existing business logic.

## Current Situation

**Current Job System:**
- Uses `tokio-cron-scheduler` for cron-based scheduling
- Custom `JobRunner` framework for consistent logging, timing, and metrics
- Jobs: `fetch_all_coins`, `portfolio_snapshot` (EOD), manual triggers via API

**Target System:**
- Use `apalis` as the primary job orchestration framework
- Keep business logic unchanged (same functions, same database operations)
- Maintain current API endpoints and scheduling behavior

## Planned Changes

### What Stays the Same

| Feature | Status |
|---------|--------|
| Job business logic | ✅ Unchanged |
| API endpoints | ✅ Unchanged |
| Database operations | ✅ Unchanged |
| Logging/metrics | ✅ Preserved |
| Manual triggers | ✅ Same behavior |

### What Changes

| Feature | Current | Apalis |
|---------|---------|--------|
| Scheduler | `tokio-cron-scheduler` | `apalis` |
| Cron format | 7 fields (incl. year) | 6 fields (no year) |
| Storage | In-memory scheduler | Database-backed (PostgreSQL) |
| Visibility | Minimal | Full job queue visibility |

## Cron Format Differences

| Action | Current Format | Apalis Format |
|--------|---------------|---------------|
| Every 15 minutes | `0 */15 * * * *` | `*/15 * * * *` |
| Daily at 23:00 UTC | `0 0 23 * * *` | `0 0 23 * * *` |
| Hourly | `0 0 * * * *` | `0 * * * *` |

**Note:** Apalis uses the standard 6-field cron format (no year field).

## Implementation Plan

### Phase 1: Core Migration

1. Add Apalis dependencies to `Cargo.toml`
2. Create `apalis_runner.rs` with same interface as `runner.rs`
3. Update job functions to work with Apalis context
4. Update main.rs to register and start Apalis workers
5. Test both systems in parallel
6. Update documentation

### Phase 2: Cleanup & Optimization

7. Remove `tokio-cron-scheduler` dependency
8. Add job queue visibility (database-backed)
9. Implement retry logic with exponential backoff
10. Add job metrics to existing dashboard

## Migration Steps

### 1. Add Dependencies

```toml
# In api/Cargo.toml

# Apalis core
apalis = { version = "0.4", features = ["full"] }

# PostgreSQL storage backend
apalis-sql = { version = "0.4" }
tokio-postgres = { version = "0.7", features = ["with-uuid-0_8", "with-serde_json-1"] }

# Keep tokio-cron-scheduler for backward compatibility during migration
# Remove in Phase 2
tokio-cron-scheduler = "0.13"
```

### 2. Create Apalis Runner

```rust
// api/src/jobs/apalis_runner.rs

use apalis::{Job, JobContext};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApalisJobResult {
    pub success: bool,
    pub job_name: String,
    pub duration_ms: u64,
    pub started_at: chrono::DateTime<Utc>,
    pub completed_at: chrono::DateTime<Utc>,
    pub metrics: JobMetrics,
    pub error: Option<String>,
}

pub struct ApalisJobRunner {
    job_name: String,
}

impl ApalisJobRunner {
    pub fn new(job_name: impl Into<String>) -> Self {
        Self { job_name: job_name.into() }
    }

    pub async fn execute<F, Fut>(&self, job_fn: F) -> ApalisJobResult
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<JobMetrics, String>>,
    {
        let started_at = Utc::now();
        let start_instant = Instant::now();

        tracing::info!("Starting job: {}", self.job_name);

        let (success, metrics, error) = match job_fn().await {
            Ok(metrics) => {
                tracing::info!(
                    "Job '{}' completed successfully: {} processed, {} created, {} updated, {} skipped",
                    self.job_name,
                    metrics.items_processed,
                    metrics.items_created,
                    metrics.items_updated,
                    metrics.items_skipped
                );
                (true, metrics, None)
            }
            Err(e) => {
                tracing::error!("Job '{}' failed: {}", self.job_name, e);
                (false, JobMetrics::default(), Some(e))
            }
        };

        let completed_at = Utc::now();
        let duration_ms = start_instant.elapsed().as_millis() as u64;

        tracing::info!(
            "Job '{}' execution time: {} ms",
            self.job_name,
            duration_ms
        );

        ApalisJobResult {
            success,
            job_name: self.job_name.clone(),
            duration_ms,
            started_at,
            completed_at,
            metrics,
            error,
        }
    }
}
```

### 3. Register Jobs in main.rs

```rust
// In main.rs, replace/supplement tokio-cron-scheduler setup

use apalis::prelude::*;
use apalis_sql::SqlStorage;

#[tokio::main]
async fn main() {
    // ... existing setup code ...

    // Initialize Apalis with PostgreSQL storage
    let storage = SqlStorage::new(db_config()).await?;
    let worker = WorkerBuilder::new()
        .with_storage(storage)
        .build();

    // Register fetch_all_coins job
    worker.register("fetch_all_coins", move |ctx: JobContext| {
        let db = db.clone();
        async move {
            let runner = ApalisJobRunner::new("fetch_all_coins");
            let result = runner.execute(|| async {
                fetch_all_coins::fetch_all_coins(&db).await
            }).await;
            
            Ok(result)
        }
    });

    // Register EOD snapshot job
    worker.register("portfolio_snapshot", move |ctx: JobContext| {
        let db = db.clone();
        async move {
            let runner = ApalisJobRunner::new("portfolio_snapshot");
            let result = runner.execute(|| async {
                portfolio_snapshot::create_all_portfolio_snapshots(&db, None).await
            }).await;
            
            Ok(result)
        }
    });

    // Start worker
    worker.run().await?;
    
    // ... rest of existing main() ...
}
```

### 4. Configuration Changes

Update `.env.example`:

```bash
# Apalis job scheduling (replaces tokio-cron-scheduler)
APALIS_FETCH_ALL_COINS_ENABLED=true
APALIS_FETCH_ALL_COINS_CRON="*/15 * * * *"  # Every 15 minutes

APALIS_EOD_SNAPSHOT_ENABLED=true
APALIS_EOD_SNAPSHOT_CRON="0 0 23 * * *"  # Daily at 23:00 UTC
```

## Testing Strategy

### Unit Tests

```rust
#[tokio::test]
async fn test_apalis_runner_matches_current_runner() {
    let current_runner = JobRunner::new("test");
    let apalis_runner = ApalisJobRunner::new("test");
    
    let current_result = current_runner.execute(|| async {
        Ok(JobMetrics {
            items_processed: 100,
            items_created: 50,
            items_updated: 30,
            items_skipped: 20,
            custom: serde_json::json!({}),
        })
    }).await;

    let apalis_result = apalis_runner.execute(|| async {
        Ok(JobMetrics {
            items_processed: 100,
            items_created: 50,
            items_updated: 30,
            items_skipped: 20,
            custom: serde_json::json!({}),
        })
    }).await;

    assert_eq!(current_result.job_name, apalis_result.job_name);
    assert_eq!(current_result.success, apalis_result.success);
    assert_eq!(current_result.metrics, apalis_result.metrics);
}
```

### Integration Tests

- Verify scheduled execution matches cron behavior
- Test no duplicate jobs during worker restart
- Verify database state after multiple runs

## Rollback Plan

If issues arise:

1. **Quick rollback:** Comment out Apalis registration in main.rs
2. **Feature flag:** Use `USE_APALIS=false` environment variable
3. **Feature flag (Cargo):** Add `apalis` feature flag, default off

```rust
#[cfg(feature = "apalis")]
use apalis::prelude::*;

#[cfg(not(feature = "apalis"))]
use tokio_cron_scheduler::JobScheduler;
```

## Success Criteria

✅ All existing jobs run with same frequency and results  
✅ No change to API endpoints or manual triggers  
✅ Job timing, logging, and metrics preserved  
✅ Database operations remain idempotent  
✅ Backward compatibility maintained during migration  

## Related Issues

- #0: Portfolio rebalancing feature (uses job system for calculations)
- #0: Notion reporting integration (will use job system)
- #0: Distributed job queue (future enhancement)

## Labels

`type: refactoring`, `priority: medium`, `area: backend`, `area: jobs`, `docs: required`
