use crate::entities::{portfolio_allocations, portfolios, snapshots};
use chrono::{Utc, NaiveDate};
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};
use serde_json::json;
use std::error::Error;
use tracing;
use uuid::Uuid;

/// Result of creating a snapshot
#[derive(Debug)]
pub struct SnapshotResult {
    pub portfolio_id: Uuid,
    pub snapshot_id: Option<Uuid>,
    pub success: bool,
    pub error: Option<String>,
    pub holdings_count: usize,
    pub total_value_usd: String,
}

/// Create a snapshot from a portfolio's persisted allocation
///
/// This function:
/// 1. Fetches the latest portfolio allocation
/// 2. Creates a snapshot from the allocation data
/// 3. Persists the snapshot to the database
///
/// # Arguments
/// * `db` - Database connection
/// * `portfolio_id` - UUID of the portfolio to snapshot
/// * `snapshot_date` - Date for the snapshot (defaults to today if None)
/// * `snapshot_type` - Type of snapshot ("eod", "manual")
///
/// # Returns
/// Result containing SnapshotResult with success status and details
pub async fn create_portfolio_snapshot(
    db: &DatabaseConnection,
    portfolio_id: Uuid,
    snapshot_date: Option<NaiveDate>,
    snapshot_type: &str,
) -> Result<SnapshotResult, Box<dyn Error + Send + Sync>> {
    tracing::info!(
        "Creating {} snapshot for portfolio {}",
        snapshot_type,
        portfolio_id
    );

    // Use provided date or default to today
    let snapshot_date = snapshot_date.unwrap_or_else(|| Utc::now().date_naive());

    // Fetch portfolio to ensure it exists
    let portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .one(db)
        .await?
        .ok_or_else(|| format!("Portfolio {} not found", portfolio_id))?;

    tracing::info!(
        "Creating snapshot for portfolio '{}' (ID: {})",
        portfolio.name,
        portfolio_id
    );

    // Fetch the latest persisted allocation for this portfolio
    // Note: There's a unique constraint on portfolio_id in portfolio_allocations,
    // so only one allocation per portfolio exists, but we order by as_of for safety
    let allocation = portfolio_allocations::Entity::find()
        .filter(portfolio_allocations::Column::PortfolioId.eq(portfolio_id))
        .order_by_desc(portfolio_allocations::Column::AsOf)
        .one(db)
        .await?
        .ok_or_else(|| {
            format!(
                "No allocation found for portfolio {}. Please run construct first.",
                portfolio_id
            )
        })?;

    tracing::info!(
        "Found allocation {} for portfolio {} with total value: {}",
        allocation.id,
        portfolio_id,
        allocation.total_value_usd
    );

    // Deserialize holdings from allocation
    let holdings: Vec<serde_json::Value> =
        serde_json::from_value(serde_json::Value::from(allocation.holdings.clone()))
            .map_err(|e| format!("Failed to deserialize allocation holdings: {}", e))?;

    let holdings_count = holdings.len();
    let total_value_usd = allocation.total_value_usd;

    tracing::info!(
        "Portfolio {} snapshot: {} assets, total value: ${}",
        portfolio_id,
        holdings_count,
        total_value_usd
    );

    // Create snapshot record
    let now = Utc::now();
    let snapshot = snapshots::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        portfolio_id: ActiveValue::Set(portfolio_id),
        snapshot_date: ActiveValue::Set(snapshot_date),
        snapshot_type: ActiveValue::Set(snapshot_type.to_string()),
        total_value_usd: ActiveValue::Set(total_value_usd),
        holdings: ActiveValue::Set(json!(holdings).into()),
        allocation_id: ActiveValue::Set(Some(allocation.id)),
        metadata: ActiveValue::Set(Some(
            json!({
                "portfolio_name": portfolio.name,
                "allocation_as_of": allocation.as_of.to_rfc3339(),
                "snapshot_time": now.to_rfc3339(),
                "created_at": now.to_rfc3339(),
            })
            .into(),
        )),
        created_at: ActiveValue::Set(now.into()),
    };

    // Insert snapshot into database
    let inserted = snapshot.insert(db).await?;

    tracing::info!(
        "Successfully created {} snapshot {} for portfolio {}",
        snapshot_type,
        inserted.id,
        portfolio_id
    );

    Ok(SnapshotResult {
        portfolio_id,
        snapshot_id: Some(inserted.id),
        success: true,
        error: None,
        holdings_count,
        total_value_usd: total_value_usd.to_string(),
    })
}

/// Create EOD snapshots for all portfolios
///
/// This function creates EOD snapshots for all portfolios in the system.
/// It's designed to be run as a scheduled job at the configured cutover time.
///
/// # Arguments
/// * `db` - Database connection
/// * `snapshot_date` - Date for the snapshots (defaults to today if None)
///
/// # Returns
/// Result containing a vector of SnapshotResults for all portfolios
pub async fn create_all_portfolio_snapshots(
    db: &DatabaseConnection,
    snapshot_date: Option<NaiveDate>,
) -> Result<Vec<SnapshotResult>, Box<dyn Error + Send + Sync>> {
    tracing::info!("Starting EOD snapshot creation for all portfolios");

    let snapshot_date = snapshot_date.unwrap_or_else(|| Utc::now().date_naive());

    // Fetch all portfolios
    let all_portfolios = portfolios::Entity::find().all(db).await?;

    tracing::info!(
        "Found {} portfolios to snapshot for date {}",
        all_portfolios.len(),
        snapshot_date
    );

    let mut results = Vec::new();

    for portfolio in all_portfolios {
        match create_portfolio_snapshot(db, portfolio.id, Some(snapshot_date), "eod").await {
            Ok(result) => {
                tracing::info!(
                    "Successfully created snapshot for portfolio {} ({}): {} holdings, ${} total",
                    portfolio.name,
                    portfolio.id,
                    result.holdings_count,
                    result.total_value_usd
                );
                results.push(result);
            }
            Err(e) => {
                tracing::error!(
                    "Failed to create snapshot for portfolio {} ({}): {}",
                    portfolio.name,
                    portfolio.id,
                    e
                );
                results.push(SnapshotResult {
                    portfolio_id: portfolio.id,
                    snapshot_id: None,
                    success: false,
                    error: Some(format!("Snapshot failed: {}", e)),
                    holdings_count: 0,
                    total_value_usd: "0".to_string(),
                });
            }
        }
    }

    let successful = results.iter().filter(|r| r.success).count();
    let failed = results.iter().filter(|r| !r.success).count();

    tracing::info!(
        "Completed EOD snapshot creation: {} successful, {} failed",
        successful,
        failed
    );

    Ok(results)
}
