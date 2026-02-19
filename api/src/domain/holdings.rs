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
///   "frozen": "0",        // Optional, defaults to "0" if not present
///   "decimals": 8         // Optional, number of decimal places for normalization
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
    
    /// Number of decimal places for this token (e.g., 18 for ETH, 6 for USDC)
    /// Used for normalizing raw balances to human-readable values
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
    
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_with_all_fields() {
        let json = r#"{
            "asset": "BTC",
            "quantity": "1.5",
            "available": "1.2",
            "frozen": "0.3"
        }"#;

        let holding: AccountHolding = serde_json::from_str(json).unwrap();
        assert_eq!(holding.asset, "BTC");
        assert_eq!(holding.quantity, "1.5");
        assert_eq!(holding.available_quantity(), "1.2");
        assert_eq!(holding.frozen_quantity(), "0.3");
    }

    #[test]
    fn test_deserialize_without_available_frozen() {
        // This is the current format from account_sync
        let json = r#"{
            "asset": "BTC",
            "quantity": "1.5"
        }"#;

        let holding: AccountHolding = serde_json::from_str(json).unwrap();
        assert_eq!(holding.asset, "BTC");
        assert_eq!(holding.quantity, "1.5");
        // Should default to quantity for available
        assert_eq!(holding.available_quantity(), "1.5");
        // Should default to "0" for frozen
        assert_eq!(holding.frozen_quantity(), "0");
    }

    #[test]
    fn test_deserialize_partial_fields() {
        // Only available field present
        let json = r#"{
            "asset": "ETH",
            "quantity": "10.0",
            "available": "9.5"
        }"#;

        let holding: AccountHolding = serde_json::from_str(json).unwrap();
        assert_eq!(holding.asset, "ETH");
        assert_eq!(holding.quantity, "10.0");
        assert_eq!(holding.available_quantity(), "9.5");
        assert_eq!(holding.frozen_quantity(), "0");
    }

    #[test]
    fn test_quantity_decimal() {
        let holding = AccountHolding {
            asset: "BTC".to_string(),
            quantity: "1.5".to_string(),
            available: None,
            frozen: None,
            decimals: None,
            price_usd: None,
            value_usd: None,
        };

        let decimal = holding.quantity_decimal();
        assert_eq!(decimal.to_string(), "1.5");
    }

    #[test]
    fn test_serialize_minimal() {
        let holding = AccountHolding {
            asset: "BTC".to_string(),
            quantity: "1.5".to_string(),
            available: None,
            frozen: None,
            decimals: None,
            price_usd: None,
            value_usd: None,
        };

        let json = serde_json::to_string(&holding).unwrap();
        // Should only have asset and quantity since others are None
        assert!(json.contains(r#""asset":"BTC"#));
        assert!(json.contains(r#""quantity":"1.5"#));
        // Optional fields should not be present
        assert!(!json.contains(r#""available"#));
        assert!(!json.contains(r#""frozen"#));
    }
}

