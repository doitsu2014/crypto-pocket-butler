//! Apalis-based job runner (apalis v1.0.0-rc.4).
//!
//! Provides cron worker definitions and handler functions that wrap the
//! existing business logic from [`crate::jobs::fetch_all_coins`] and
//! [`crate::jobs::portfolio_snapshot`].  The scheduler is an Apalis
//! [`Monitor`] running a [`CronStream`] per job; all business logic and
//! logging remain identical to the previous `tokio-cron-scheduler` integration.

use crate::jobs::{fetch_all_coins, portfolio_snapshot};
use apalis_core::{
    error::BoxDynError,
    monitor::Monitor,
    task::data::Data,
    worker::builder::WorkerBuilder,
};
use apalis_cron::{CronStream, Tick};
use cron::Schedule;
use sea_orm::DatabaseConnection;
use std::str::FromStr;

// ---------------------------------------------------------------------------
// Job handlers
// ---------------------------------------------------------------------------

/// Handler executed by Apalis for every fetch-all-coins cron tick.
///
/// Delegates to [`fetch_all_coins::fetch_all_coins`] and preserves all
/// existing logging behaviour.
pub async fn handle_fetch_all_coins(
    tick: Tick,
    db: Data<DatabaseConnection>,
) -> Result<(), BoxDynError> {
    tracing::info!("Running scheduled fetch all coins job (tick: {})", tick.get_timestamp());
    match fetch_all_coins::fetch_all_coins(&db).await {
        Ok(result) => {
            if result.success {
                tracing::info!(
                    "Fetch all coins job completed successfully: {} coins fetched, {} assets created, {} updated, {} prices stored",
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
        Err(e) => {
            tracing::error!("Fetch all coins job failed with error: {}", e);
        }
    }
    Ok(())
}

/// Handler executed by Apalis for every EOD snapshot cron tick.
///
/// Delegates to [`portfolio_snapshot::create_all_portfolio_snapshots`] and
/// preserves all existing logging behaviour.
pub async fn handle_eod_snapshot(
    tick: Tick,
    db: Data<DatabaseConnection>,
) -> Result<(), BoxDynError> {
    tracing::info!("Running scheduled EOD snapshot job (tick: {})", tick.get_timestamp());
    match portfolio_snapshot::create_all_portfolio_snapshots(&db, None).await {
        Ok(results) => {
            let successful = results.iter().filter(|r| r.success).count();
            let failed = results.iter().filter(|r| !r.success).count();

            tracing::info!(
                "EOD snapshot job completed: {} portfolios processed, {} successful, {} failed",
                results.len(),
                successful,
                failed
            );

            for result in results.iter().filter(|r| !r.success) {
                if let Some(error) = &result.error {
                    tracing::error!(
                        "Failed to create EOD snapshot for portfolio {}: {}",
                        result.portfolio_id,
                        error
                    );
                }
            }
        }
        Err(e) => {
            tracing::error!("EOD snapshot job failed with error: {}", e);
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Monitor construction
// ---------------------------------------------------------------------------

/// Build and return an Apalis [`Monitor`] containing all enabled cron workers.
///
/// Each worker is wired with the supplied `db` as [`Data`] so that job
/// handlers can retrieve it via the `Data<DatabaseConnection>` extractor.
///
/// # Cron format
///
/// The scheduler uses the same 6-field `cron` expression format already
/// employed in the rest of the application:
/// ```text
/// sec  min  hour  day-of-month  month  day-of-week
/// ```
/// Examples:
/// - `"0 */15 * * * *"` – every 15 minutes (on the 0-second mark)
/// - `"0 0 23 * * *"` – daily at 23:00 UTC
pub fn build_monitor(db: DatabaseConnection) -> Monitor {
    let mut monitor = Monitor::new();

    // -----------------------------------------------------------------------
    // Fetch-all-coins worker
    // -----------------------------------------------------------------------
    let fetch_enabled = std::env::var("APALIS_FETCH_ALL_COINS_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if fetch_enabled {
        let fetch_cron = std::env::var("APALIS_FETCH_ALL_COINS_CRON")
            .unwrap_or_else(|_| "0 */15 * * * *".to_string());

        tracing::info!(
            "Scheduling Apalis fetch-all-coins worker: cron='{}'",
            fetch_cron
        );

        let schedule = Schedule::from_str(&fetch_cron)
            .unwrap_or_else(|_| panic!("Invalid cron expression for APALIS_FETCH_ALL_COINS_CRON: '{}'", fetch_cron));

        let db_clone = db.clone();
        monitor = monitor.register(move |_| {
            WorkerBuilder::new("apalis-fetch-all-coins")
                .backend(CronStream::new(schedule.clone()))
                .data(db_clone.clone())
                .build(handle_fetch_all_coins)
        });
        tracing::info!("Apalis fetch-all-coins worker registered");
    } else {
        tracing::info!("Apalis fetch-all-coins worker is disabled");
    }

    // -----------------------------------------------------------------------
    // EOD snapshot worker
    // -----------------------------------------------------------------------
    let snapshot_enabled = std::env::var("APALIS_EOD_SNAPSHOT_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    if snapshot_enabled {
        let snapshot_cron = std::env::var("APALIS_EOD_SNAPSHOT_CRON")
            .unwrap_or_else(|_| "0 0 23 * * *".to_string());

        tracing::info!(
            "Scheduling Apalis EOD snapshot worker: cron='{}'",
            snapshot_cron
        );

        let schedule = Schedule::from_str(&snapshot_cron)
            .unwrap_or_else(|_| panic!("Invalid cron expression for APALIS_EOD_SNAPSHOT_CRON: '{}'", snapshot_cron));

        let db_clone = db.clone();
        monitor = monitor.register(move |_| {
            WorkerBuilder::new("apalis-eod-snapshot")
                .backend(CronStream::new(schedule.clone()))
                .data(db_clone.clone())
                .build(handle_eod_snapshot)
        });
        tracing::info!("Apalis EOD snapshot worker registered");
    } else {
        tracing::info!("Apalis EOD snapshot worker is disabled");
    }

    monitor
}
