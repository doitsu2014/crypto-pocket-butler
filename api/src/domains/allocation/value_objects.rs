/// Allocation domain value objects: AllocationData, SnapshotData, UnpricedAsset
///
/// These value objects represent complete, immutable data structures used in
/// allocation computation and snapshot persistence.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::entities::{AllocationItem, SnapshotHolding, SnapshotMetadata};

/// Complete allocation data for a portfolio.
///
/// Contains all holdings with their values, total portfolio value, and metadata.
///
/// # JSON Schema (for database storage)
/// The `items` field is serialized as a JSON array when stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationData {
    /// All holdings in the allocation
    pub items: Vec<AllocationItem>,

    /// Total portfolio value in USD (excludes unpriced assets)
    pub total_value_usd: rust_decimal::Decimal,

    /// Timestamp when allocation was computed
    pub as_of: chrono::DateTime<chrono::FixedOffset>,
}

impl AllocationData {
    /// Get all unpriced assets
    pub fn unpriced_assets(&self) -> Vec<&AllocationItem> {
        self.items.iter().filter(|item| item.unpriced).collect()
    }

    /// Get all priced assets
    pub fn priced_assets(&self) -> Vec<&AllocationItem> {
        self.items.iter().filter(|item| !item.unpriced).collect()
    }

    /// Count of holdings
    pub fn holdings_count(&self) -> usize {
        self.items.len()
    }
}

/// Reference to an unpriced asset in an allocation.
///
/// Used for reporting which assets lack pricing data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnpricedAsset {
    /// Asset symbol
    pub asset: String,
    /// Quantity held
    pub quantity: String,
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
