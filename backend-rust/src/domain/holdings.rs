/// Domain model for account holdings (quantity-only)
///
/// Represents raw holdings data from accounts, before price enrichment.

use serde::{Deserialize, Serialize};

/// A holding in an account with quantity information only.
///
/// This represents the raw data structure as stored in account holdings JSON.
/// Price and value information is added later during allocation construction.
///
/// # JSON Schema
/// ```json
/// {
///   "asset": "BTC",
///   "quantity": "1.5",
///   "available": "1.5",
///   "frozen": "0"
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountHolding {
    /// Asset symbol (e.g., "BTC", "ETH")
    pub asset: String,
    
    /// Total quantity as a decimal string
    pub quantity: String,
    
    /// Available (unfrozen) quantity as a decimal string
    pub available: String,
    
    /// Frozen (locked) quantity as a decimal string
    pub frozen: String,
    
    /// Optional price from account data (usually not present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub price_usd: Option<f64>,
    
    /// Optional value from account data (usually not present)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_usd: Option<f64>,
}

impl AccountHolding {
    /// Parse quantity as a Decimal
    pub fn quantity_decimal(&self) -> rust_decimal::Decimal {
        use std::str::FromStr;
        rust_decimal::Decimal::from_str(&self.quantity).unwrap_or(rust_decimal::Decimal::ZERO)
    }
}
