use crate::entities::{accounts, portfolio_accounts, portfolios, snapshots};
use chrono::{Utc, NaiveDate};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter,
};
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::str::FromStr;
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

/// Holding data structure for deserialization
#[derive(Debug, serde::Deserialize)]
struct HoldingData {
    asset: String,
    quantity: String,
    available: String,
    frozen: String,
    #[serde(default)]
    price_usd: f64,
    #[serde(default)]
    value_usd: f64,
}

/// Parse a decimal string, returning 0 if parsing fails
fn parse_decimal_or_zero(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap_or_else(|_| Decimal::ZERO)
}

/// Create an EOD snapshot for a single portfolio
///
/// This function:
/// 1. Fetches all accounts linked to the portfolio
/// 2. Aggregates holdings from all accounts
/// 3. Calculates total portfolio value
/// 4. Persists the snapshot to the database
///
/// # Arguments
/// * `db` - Database connection
/// * `portfolio_id` - UUID of the portfolio to snapshot
/// * `snapshot_date` - Date for the snapshot (defaults to today if None)
/// * `snapshot_type` - Type of snapshot ("eod", "manual", "hourly")
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

    // Get all accounts linked to this portfolio
    let portfolio_accounts_list = portfolio_accounts::Entity::find()
        .filter(portfolio_accounts::Column::PortfolioId.eq(portfolio_id))
        .all(db)
        .await?;

    let account_ids: Vec<Uuid> = portfolio_accounts_list
        .iter()
        .map(|pa| pa.account_id)
        .collect();
    
    let account_count = account_ids.len();

    tracing::info!(
        "Found {} accounts linked to portfolio {}",
        account_count,
        portfolio_id
    );

    // Fetch all accounts
    let accounts_list = accounts::Entity::find()
        .filter(accounts::Column::Id.is_in(account_ids))
        .all(db)
        .await?;

    // Aggregate holdings by asset
    let mut holdings_map: HashMap<String, serde_json::Value> = HashMap::new();
    let mut total_holdings_count = 0;

    for account in accounts_list {
        if let Some(holdings_json) = account.holdings {
            // Deserialize holdings
            let holdings: Vec<HoldingData> =
                match serde_json::from_value(serde_json::Value::from(holdings_json)) {
                    Ok(h) => h,
                    Err(e) => {
                        tracing::warn!(
                            "Failed to deserialize holdings for account {}: {}",
                            account.id,
                            e
                        );
                        continue;
                    }
                };

            for holding in holdings {
                // Skip holdings with empty asset names
                if holding.asset.is_empty() {
                    continue;
                }

                total_holdings_count += 1;

                let entry = holdings_map
                    .entry(holding.asset.clone())
                    .or_insert_with(|| {
                        json!({
                            "asset": holding.asset.clone(),
                            "total_quantity": "0",
                            "total_available": "0",
                            "total_frozen": "0",
                            "price_usd": holding.price_usd,
                            "value_usd": 0.0,
                        })
                    });

                // Add to totals using Decimal for precision
                let qty = parse_decimal_or_zero(&holding.quantity);
                let avail = parse_decimal_or_zero(&holding.available);
                let frz = parse_decimal_or_zero(&holding.frozen);

                if let Some(obj) = entry.as_object_mut() {
                    let curr_qty = parse_decimal_or_zero(
                        obj.get("total_quantity")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0"),
                    );
                    let curr_avail = parse_decimal_or_zero(
                        obj.get("total_available")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0"),
                    );
                    let curr_frz = parse_decimal_or_zero(
                        obj.get("total_frozen")
                            .and_then(|v| v.as_str())
                            .unwrap_or("0"),
                    );
                    let curr_value = obj
                        .get("value_usd")
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0);

                    obj.insert("total_quantity".to_string(), json!((curr_qty + qty).to_string()));
                    obj.insert(
                        "total_available".to_string(),
                        json!((curr_avail + avail).to_string()),
                    );
                    obj.insert(
                        "total_frozen".to_string(),
                        json!((curr_frz + frz).to_string()),
                    );
                    obj.insert("value_usd".to_string(), json!(curr_value + holding.value_usd));
                }
            }
        }
    }

    // Convert holdings map to array
    let holdings_array: Vec<serde_json::Value> = holdings_map.into_values().collect();

    // Calculate total portfolio value
    let total_value_usd: f64 = holdings_array
        .iter()
        .filter_map(|h| h.get("value_usd").and_then(|v| v.as_f64()))
        .sum();

    let total_value_decimal = Decimal::from_str(&total_value_usd.to_string())
        .unwrap_or_else(|_| Decimal::ZERO);

    tracing::info!(
        "Portfolio {} snapshot: {} unique assets, total value: ${:.2}",
        portfolio_id,
        holdings_array.len(),
        total_value_usd
    );

    // Create snapshot record
    let snapshot = snapshots::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        portfolio_id: ActiveValue::Set(portfolio_id),
        snapshot_date: ActiveValue::Set(snapshot_date),
        snapshot_type: ActiveValue::Set(snapshot_type.to_string()),
        total_value_usd: ActiveValue::Set(total_value_decimal),
        holdings: ActiveValue::Set(json!(holdings_array).into()),
        metadata: ActiveValue::Set(Some(
            json!({
                "portfolio_name": portfolio.name,
                "account_count": account_count,
                "holdings_count": total_holdings_count,
                "unique_assets": holdings_array.len(),
                "created_at": Utc::now().to_rfc3339(),
            })
            .into(),
        )),
        created_at: ActiveValue::Set(Utc::now().into()),
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
        holdings_count: holdings_array.len(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_decimal_or_zero() {
        assert_eq!(parse_decimal_or_zero("123.45"), Decimal::from_str("123.45").unwrap());
        assert_eq!(parse_decimal_or_zero("0"), Decimal::ZERO);
        assert_eq!(parse_decimal_or_zero("invalid"), Decimal::ZERO);
        assert_eq!(parse_decimal_or_zero(""), Decimal::ZERO);
    }
}
