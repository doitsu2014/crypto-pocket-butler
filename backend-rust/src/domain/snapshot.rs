/// Domain model for portfolio snapshots (point-in-time records)
///
/// Represents immutable snapshots of portfolio allocations at specific dates.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A holding in a snapshot.
///
/// Similar to AllocationItem but represents historical data.
/// Snapshots preserve the allocation data as it was at snapshot time.
///
/// # JSON Schema
/// ```json
/// {
///   "asset": "BTC",
///   "quantity": "1.5",
///   "price_usd": 50000.0,
///   "value_usd": 75000.0,
///   "weight": 45.5,
///   "unpriced": false
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, ToSchema)]
pub struct SnapshotHolding {
    /// Asset symbol
    pub asset: String,
    
    /// Quantity held at snapshot time
    pub quantity: String,
    
    /// Price per unit in USD at snapshot time
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    
    /// Value in USD at snapshot time
    /// Note: f64 is used for API compatibility and historical preservation
    pub value_usd: f64,
    
    /// Weight as percentage of portfolio (0-100)
    pub weight: f64,
    
    /// Whether asset was unpriced at snapshot time
    #[serde(default)]
    pub unpriced: bool,
}

impl From<crate::domain::allocation::AllocationItem> for SnapshotHolding {
    fn from(item: crate::domain::allocation::AllocationItem) -> Self {
        Self {
            asset: item.asset,
            quantity: item.quantity,
            price_usd: item.price_usd,
            value_usd: item.value_usd,
            weight: item.weight,
            unpriced: item.unpriced,
        }
    }
}

/// Metadata for a snapshot providing context about when and how it was created.
///
/// # JSON Schema
/// ```json
/// {
///   "portfolio_name": "My Portfolio",
///   "allocation_as_of": "2024-01-01T12:00:00Z",
///   "snapshot_time": "2024-01-01T16:00:00Z",
///   "created_at": "2024-01-01T16:00:00Z"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SnapshotMetadata {
    /// Name of the portfolio at snapshot time
    pub portfolio_name: String,
    
    /// Timestamp when the underlying allocation was computed
    pub allocation_as_of: String,
    
    /// Timestamp when the snapshot was taken
    pub snapshot_time: String,
    
    /// Timestamp when the snapshot record was created
    pub created_at: String,
}

/// Complete snapshot data including holdings, metadata, and totals.
///
/// This is the internal representation used before serialization to the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotData {
    /// Portfolio ID
    pub portfolio_id: Uuid,
    
    /// Date of the snapshot (without time)
    pub snapshot_date: chrono::NaiveDate,
    
    /// Type of snapshot (e.g., "eod", "manual", "hourly")
    pub snapshot_type: String,
    
    /// Holdings at snapshot time
    pub holdings: Vec<SnapshotHolding>,
    
    /// Total portfolio value in USD
    pub total_value_usd: rust_decimal::Decimal,
    
    /// Additional metadata
    pub metadata: Option<SnapshotMetadata>,
    
    /// Reference to the allocation this snapshot was created from
    pub allocation_id: Option<Uuid>,
}

impl SnapshotData {
    /// Get count of holdings in snapshot
    pub fn holdings_count(&self) -> usize {
        self.holdings.len()
    }
    
    /// Get all unpriced holdings
    pub fn unpriced_holdings(&self) -> Vec<&SnapshotHolding> {
        self.holdings.iter().filter(|h| h.unpriced).collect()
    }
}
