//! Apalis-based job runner (apalis v1.0.0-rc.4, Phase 2).
//!
//! Phase 2 uses [`PostgresStorage`] as the persistent backend for each job
//! type.  A [`CronStream`] "scheduler" worker pushes job structs into
//! PostgreSQL on the configured cron schedule; a separate "execution" worker
//! consumes from that queue and runs the business logic.
//!
//! This architecture allows [`apalis-board`] to query job history, queue
//! depths, and worker state via standard `apalis-board-api` routes.

use crate::jobs::{fetch_all_coins, portfolio_snapshot};
use apalis_board_api::framework::{ApiBuilder, RegisterRoute};
use apalis_core::{
    backend::TaskSink,
    error::BoxDynError,
    monitor::Monitor,
    task::data::Data,
    worker::builder::WorkerBuilder,
};
use apalis_cron::{CronStream, Tick};
use apalis_postgres::{Config as PgConfig, PostgresStorage, PgPool};
use axum::Router;
use cron::Schedule;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Job payload types
// ---------------------------------------------------------------------------

/// Payload pushed to the `apalis-fetch-all-coins` PostgreSQL queue on each
/// cron tick.  The type is intentionally empty – the scheduler only needs to
/// signal "run now".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchAllCoinsJob;

/// Payload pushed to the `apalis-eod-snapshot` PostgreSQL queue on each
/// cron tick.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EodSnapshotJob;

// ---------------------------------------------------------------------------
// Scheduler handlers (CronStream → push to PostgreSQL)
// ---------------------------------------------------------------------------

/// Cron-triggered handler: pushes a [`FetchAllCoinsJob`] to the PostgreSQL
/// queue so it can be picked up by the execution worker.
///
/// `PostgresStorage` is Clone (it wraps a reference-counted pool), so we
/// clone it out of the `Data` wrapper to obtain `&mut self` for the `push`
/// call without the overhead of a mutex.
pub async fn schedule_fetch_all_coins(
    _tick: Tick,
    storage: Data<PostgresStorage<FetchAllCoinsJob>>,
) -> Result<(), BoxDynError> {
    let mut s = (*storage).clone();
    s.push(FetchAllCoinsJob).await?;
    Ok(())
}

/// Cron-triggered handler: pushes an [`EodSnapshotJob`] to the PostgreSQL
/// queue so it can be picked up by the execution worker.
pub async fn schedule_eod_snapshot(
    _tick: Tick,
    storage: Data<PostgresStorage<EodSnapshotJob>>,
) -> Result<(), BoxDynError> {
    let mut s = (*storage).clone();
    s.push(EodSnapshotJob).await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Execution handlers (PostgresStorage → run business logic)
// ---------------------------------------------------------------------------

/// Execution handler: consumes a [`FetchAllCoinsJob`] from the PostgreSQL
/// queue and delegates to [`fetch_all_coins::fetch_all_coins`].
pub async fn handle_fetch_all_coins(
    _job: FetchAllCoinsJob,
    db: Data<DatabaseConnection>,
) -> Result<(), BoxDynError> {
    tracing::info!("Running scheduled fetch all coins job");
    match fetch_all_coins::fetch_all_coins(&db).await {
        Ok(result) => {
            if result.success {
                tracing::info!(
                    "Fetch all coins job completed: {} fetched, {} created, {} updated, {} prices stored",
                    result.coins_fetched,
                    result.assets_created,
                    result.assets_updated,
                    result.prices_stored
                );
            } else {
                tracing::error!(
                    "Fetch all coins job failed: {}",
                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                );
            }
        }
        Err(e) => tracing::error!("Fetch all coins job error: {}", e),
    }
    Ok(())
}

/// Execution handler: consumes an [`EodSnapshotJob`] from the PostgreSQL
/// queue and delegates to [`portfolio_snapshot::create_all_portfolio_snapshots`].
pub async fn handle_eod_snapshot(
    _job: EodSnapshotJob,
    db: Data<DatabaseConnection>,
) -> Result<(), BoxDynError> {
    tracing::info!("Running scheduled EOD snapshot job");
    match portfolio_snapshot::create_all_portfolio_snapshots(&db, None).await {
        Ok(results) => {
            let successful = results.iter().filter(|r| r.success).count();
            let failed = results.iter().filter(|r| !r.success).count();
            tracing::info!(
                "EOD snapshot job completed: {} portfolios, {} ok, {} failed",
                results.len(),
                successful,
                failed
            );
            for result in results.iter().filter(|r| !r.success) {
                if let Some(error) = &result.error {
                    tracing::error!(
                        "EOD snapshot failed for portfolio {}: {}",
                        result.portfolio_id,
                        error
                    );
                }
            }
        }
        Err(e) => tracing::error!("EOD snapshot job error: {}", e),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Components returned by build_monitor
// ---------------------------------------------------------------------------

/// Contains the running [`Monitor`] and the two [`PostgresStorage`] instances
/// needed to build the `apalis-board` API routes.
pub struct ApalisComponents {
    pub monitor: Monitor,
    pub fetch_storage: PostgresStorage<FetchAllCoinsJob>,
    pub snapshot_storage: PostgresStorage<EodSnapshotJob>,
}

// ---------------------------------------------------------------------------
// Monitor construction
// ---------------------------------------------------------------------------

/// Build an Apalis [`Monitor`] and return the associated [`PostgresStorage`]
/// instances for use with `apalis-board`.
///
/// For each enabled job the function registers **two** workers:
/// 1. A `CronStream`-backed "scheduler" that fires on the configured cron
///    expression and pushes a job struct into PostgreSQL.
/// 2. A `PostgresStorage`-backed "execution" worker that polls the queue and
///    runs the actual business logic.
///
/// # Cron format
///
/// Six-field `cron` expression: `sec  min  hour  dom  month  dow`.
/// Examples:
/// - `"0 */15 * * * *"` — every 15 minutes
/// - `"0 0 23 * * *"`   — daily at 23:00 UTC
pub fn build_monitor(db: DatabaseConnection, pg_pool: PgPool) -> ApalisComponents {
    let mut monitor = Monitor::new();

    // -----------------------------------------------------------------------
    // Fetch-all-coins
    // -----------------------------------------------------------------------
    let fetch_enabled = std::env::var("APALIS_FETCH_ALL_COINS_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let fetch_config = PgConfig::new("apalis-fetch-all-coins");
    let fetch_storage =
        PostgresStorage::<FetchAllCoinsJob>::new_with_config(&pg_pool, &fetch_config);

    if fetch_enabled {
        let fetch_cron = std::env::var("APALIS_FETCH_ALL_COINS_CRON")
            .unwrap_or_else(|_| "0 */15 * * * *".to_string());

        tracing::info!(
            "Scheduling fetch-all-coins: cron='{}'",
            fetch_cron
        );

        let schedule = Schedule::from_str(&fetch_cron).unwrap_or_else(|_| {
            panic!(
                "Invalid cron for APALIS_FETCH_ALL_COINS_CRON: '{}'",
                fetch_cron
            )
        });

        let fetch_scheduler_storage = fetch_storage.clone();
        let db_clone = db.clone();
        let fetch_storage_exec = fetch_storage.clone();

        monitor = monitor
            .register(move |_| {
                WorkerBuilder::new("apalis-fetch-all-coins-scheduler")
                    .backend(CronStream::new(schedule.clone()))
                    .data(fetch_scheduler_storage.clone())
                    .build(schedule_fetch_all_coins)
            })
            .register(move |_| {
                WorkerBuilder::new("apalis-fetch-all-coins")
                    .backend(fetch_storage_exec.clone())
                    .data(db_clone.clone())
                    .build(handle_fetch_all_coins)
            });

        tracing::info!("Apalis fetch-all-coins workers registered");
    } else {
        tracing::info!("Apalis fetch-all-coins worker is disabled");
    }

    // -----------------------------------------------------------------------
    // EOD snapshot
    // -----------------------------------------------------------------------
    let snapshot_enabled = std::env::var("APALIS_EOD_SNAPSHOT_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let snapshot_config = PgConfig::new("apalis-eod-snapshot");
    let snapshot_storage =
        PostgresStorage::<EodSnapshotJob>::new_with_config(&pg_pool, &snapshot_config);

    if snapshot_enabled {
        let snapshot_cron = std::env::var("APALIS_EOD_SNAPSHOT_CRON")
            .unwrap_or_else(|_| "0 0 23 * * *".to_string());

        tracing::info!(
            "Scheduling EOD snapshot: cron='{}'",
            snapshot_cron
        );

        let schedule = Schedule::from_str(&snapshot_cron).unwrap_or_else(|_| {
            panic!(
                "Invalid cron for APALIS_EOD_SNAPSHOT_CRON: '{}'",
                snapshot_cron
            )
        });

        let snapshot_scheduler_storage = snapshot_storage.clone();
        let db_clone = db.clone();
        let snapshot_storage_exec = snapshot_storage.clone();

        monitor = monitor
            .register(move |_| {
                WorkerBuilder::new("apalis-eod-snapshot-scheduler")
                    .backend(CronStream::new(schedule.clone()))
                    .data(snapshot_scheduler_storage.clone())
                    .build(schedule_eod_snapshot)
            })
            .register(move |_| {
                WorkerBuilder::new("apalis-eod-snapshot")
                    .backend(snapshot_storage_exec.clone())
                    .data(db_clone.clone())
                    .build(handle_eod_snapshot)
            });

        tracing::info!("Apalis EOD snapshot workers registered");
    } else {
        tracing::info!("Apalis EOD snapshot worker is disabled");
    }

    ApalisComponents {
        monitor,
        fetch_storage,
        snapshot_storage,
    }
}

// ---------------------------------------------------------------------------
// apalis-board API router
// ---------------------------------------------------------------------------

/// Build an Axum [`Router`] exposing `apalis-board` management API routes for
/// the two job queues.
///
/// Routes added (relative to the mount point):
/// - `GET /queues` — list all queues
/// - `GET /overview` — aggregate statistics
/// - `GET /workers` — all running workers
/// - `GET /tasks` — all tasks (cross-queue)
/// - `GET /events` — SSE stream of tracing events
/// - `GET /queues/{queue}/tasks` — per-queue task list
/// - `GET /queues/{queue}/stats` — per-queue statistics
/// - `GET /queues/{queue}/workers` — per-queue workers
/// - `PUT /queues/{queue}/tasks` — push a new task
/// - `GET /queues/{queue}/tasks/{id}` — fetch a single task
///
/// Mount the returned router under `/api/v1` so the pre-built board SPA
/// (which hard-codes `/api/v1` as its API base) can reach these routes.
pub fn build_board_router(
    fetch_storage: PostgresStorage<FetchAllCoinsJob>,
    snapshot_storage: PostgresStorage<EodSnapshotJob>,
) -> Router {
    ApiBuilder::new(Router::new())
        .register(fetch_storage)
        .register(snapshot_storage)
        .build()
}
