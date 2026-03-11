/// Portfolio domain entities: PortfolioAccount, PortfolioAllocation, Snapshot
///
/// These are entities (have identity) within the Portfolio aggregate boundary.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A link between a portfolio and one of its accounts.
///
/// Enforces that each account appears at most once in a portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioAccount {
    pub portfolio_id: Uuid,
    pub account_id: Uuid,
    pub added_at: DateTime<Utc>,
}

impl PortfolioAccount {
    pub fn new(portfolio_id: Uuid, account_id: Uuid) -> Self {
        Self {
            portfolio_id,
            account_id,
            added_at: Utc::now(),
        }
    }
}

/// A computed allocation snapshot stored inside a portfolio.
///
/// Captures the aggregated holdings with prices and weights at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortfolioAllocation {
    pub id: Uuid,
    pub portfolio_id: Uuid,
    /// Serialised `AllocationData` JSON.
    pub data: serde_json::Value,
    pub calculated_at: DateTime<Utc>,
}

/// A point-in-time immutable snapshot of a portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub snapshot_date: chrono::NaiveDate,
    pub snapshot_type: String,
    /// Serialised `SnapshotData` JSON.
    pub data: serde_json::Value,
    pub total_value_usd: Decimal,
    pub created_at: DateTime<Utc>,
}
