/// Domain model for portfolio allocations (with price, value, and weight)
///
/// Represents computed allocations after aggregating holdings and enriching with price data.

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
    
    /// Total quantity across all accounts (decimal string)
    pub quantity: String,
    
    /// Current price per unit in USD
    /// None if asset is unpriced
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    
    /// Total value in USD (quantity * price)
    pub value_usd: f64,
    
    /// Percentage of total portfolio value (0-100)
    /// Computed only for priced assets
    pub weight: f64,
    
    /// Flag indicating if this asset has no price data
    #[serde(default)]
    pub unpriced: bool,
}

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
