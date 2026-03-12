/// Allocation domain entities: AllocationItem, SnapshotHolding, SnapshotMetadata
///
/// `AllocationItem` represents a single asset in a computed portfolio allocation
/// after price enrichment and weight calculation.
///
/// `SnapshotHolding` is the point-in-time equivalent used in snapshots.
///
/// `SnapshotMetadata` carries context about when and how a snapshot was created.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// A single asset holding in a portfolio allocation with complete pricing information.
///
/// This represents an asset after:
/// - Aggregating quantities across all accounts
/// - Looking up current market prices
/// - Computing USD values
/// - Calculating portfolio weights
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
pub struct AllocationItem {
    /// Asset symbol
    pub asset: String,

    /// Chain label when asset is chain-specific (e.g. "ethereum", "bsc", "solana").
    /// None for exchange accounts (OKX) where no chain context applies.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,

    /// Total quantity across all accounts (decimal string)
    pub quantity: String,

    /// Current price per unit in USD
    /// None if asset is unpriced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,

    /// Total value in USD (quantity * price)
    /// Note: f64 is used for API compatibility. For internal calculations,
    /// use Decimal types. This field is computed from Decimal values.
    pub value_usd: f64,

    /// Percentage of total portfolio value (0-100)
    /// Computed only for priced assets
    pub weight: f64,

    /// Flag indicating if this asset has no price data
    #[serde(default)]
    pub unpriced: bool,
}

/// A holding in a snapshot.
///
/// Similar to [`AllocationItem`] but represents historical data.
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

impl From<AllocationItem> for SnapshotHolding {
    fn from(item: AllocationItem) -> Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation_item_serialization() {
        let item = AllocationItem {
            asset: "BTC".to_string(),
            chain: None,
            quantity: "1.5".to_string(),
            price_usd: Some(50000.0),
            value_usd: 75000.0,
            weight: 45.5,
            unpriced: false,
        };

        let json = serde_json::to_string(&item).unwrap();
        let deserialized: AllocationItem = serde_json::from_str(&json).unwrap();
        assert_eq!(item, deserialized);
    }

    #[test]
    fn test_allocation_item_unpriced_skips_price() {
        let item = AllocationItem {
            asset: "UNKNOWN".to_string(),
            chain: None,
            quantity: "100.0".to_string(),
            price_usd: None,
            value_usd: 0.0,
            weight: 0.0,
            unpriced: true,
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(!json.contains("price_usd"), "price_usd should be omitted when None");
    }

    #[test]
    fn test_snapshot_holding_from_allocation_item() {
        let item = AllocationItem {
            asset: "ETH".to_string(),
            chain: Some("ethereum".to_string()),
            quantity: "10.0".to_string(),
            price_usd: Some(3000.0),
            value_usd: 30000.0,
            weight: 30.0,
            unpriced: false,
        };

        let snapshot = SnapshotHolding::from(item);
        assert_eq!(snapshot.asset, "ETH");
        assert_eq!(snapshot.quantity, "10.0");
        assert_eq!(snapshot.price_usd, Some(3000.0));
        assert_eq!(snapshot.value_usd, 30000.0);
        assert_eq!(snapshot.weight, 30.0);
        assert!(!snapshot.unpriced);
    }

    #[test]
    fn test_snapshot_metadata_roundtrip() {
        let meta = SnapshotMetadata {
            portfolio_name: "My Portfolio".to_string(),
            allocation_as_of: "2024-01-01T12:00:00Z".to_string(),
            snapshot_time: "2024-01-01T16:00:00Z".to_string(),
            created_at: "2024-01-01T16:00:00Z".to_string(),
        };

        let json = serde_json::to_string(&meta).unwrap();
        let deserialized: SnapshotMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(meta.portfolio_name, deserialized.portfolio_name);
        assert_eq!(meta.allocation_as_of, deserialized.allocation_as_of);
    }
}
