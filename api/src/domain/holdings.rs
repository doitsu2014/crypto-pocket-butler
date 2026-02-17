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
///   "available": "1.5",   // Optional, defaults to quantity if not present
///   "frozen": "0"         // Optional, defaults to "0" if not present
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountHolding {
    /// Asset symbol (e.g., "BTC", "ETH")
    pub asset: String,
    
    /// Total quantity as a decimal string
    pub quantity: String,
    
    /// Available (unfrozen) quantity as a decimal string
    /// Defaults to quantity if not specified (for legacy data compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<String>,
    
    /// Frozen (locked) quantity as a decimal string
    /// Defaults to "0" if not specified (for legacy data compatibility)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen: Option<String>,
    
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
    
    /// Get available quantity, defaulting to total quantity if not specified
    /// This handles legacy data where available field might not exist
    pub fn available_quantity(&self) -> &str {
        self.available.as_deref().unwrap_or(&self.quantity)
    }
    
    /// Get frozen quantity, defaulting to "0" if not specified
    /// This handles legacy data where frozen field might not exist
    pub fn frozen_quantity(&self) -> &str {
        self.frozen.as_deref().unwrap_or("0")
    }
}
