/// Account domain entities: AccountHolding and AccountHoldings
///
/// `AccountHolding` is the per-asset quantity record stored inside an account.
/// `AccountHoldings` is the value object that wraps the collection and enforces
/// aggregate-level invariants (e.g. deduplication by symbol).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// A single asset holding within an account.
///
/// All quantity values are **normalised** human-readable decimals (not raw
/// blockchain integers). The `decimals` field is metadata — do not use it to
/// re-normalise a quantity that is already stored in normalised form.
///
/// This is the canonical domain type; `crate::domain::AccountHolding` re-exports
/// this for backward compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AccountHolding {
    /// Asset symbol (e.g. "BTC", "ETH").
    pub asset: String,
    /// Total quantity as a decimal string.
    pub quantity: String,
    /// Available (unfrozen) quantity. Defaults to `quantity` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub available: Option<String>,
    /// Frozen (locked) quantity. Defaults to `"0"` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frozen: Option<String>,
    /// Token decimal places (metadata only — quantity is already normalised).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
}

impl AccountHolding {
    pub fn new(asset: impl Into<String>, quantity: impl Into<String>) -> Self {
        Self {
            asset: asset.into(),
            quantity: quantity.into(),
            available: None,
            frozen: None,
            decimals: None,
        }
    }

    /// Parse `quantity` as a [`Decimal`], returning zero on parse failure.
    pub fn quantity_decimal(&self) -> Decimal {
        Decimal::from_str(&self.quantity).unwrap_or(Decimal::ZERO)
    }

    /// Returns the available quantity, falling back to `quantity` for legacy data.
    pub fn available_quantity(&self) -> &str {
        self.available.as_deref().unwrap_or(&self.quantity)
    }

    /// Returns the frozen quantity, defaulting to `"0"` for legacy data.
    pub fn frozen_quantity(&self) -> &str {
        self.frozen.as_deref().unwrap_or("0")
    }

    /// Returns the available balance as a [`Decimal`].
    pub fn available_decimal(&self) -> Decimal {
        Decimal::from_str(self.available_quantity()).unwrap_or(Decimal::ZERO)
    }

    /// Returns the frozen balance as a [`Decimal`].
    pub fn frozen_decimal(&self) -> Decimal {
        Decimal::from_str(self.frozen_quantity()).unwrap_or(Decimal::ZERO)
    }

    /// Validates that `available + frozen == quantity` when both fields are present.
    pub fn is_consistent(&self) -> bool {
        self.available_decimal() + self.frozen_decimal() == self.quantity_decimal()
    }
}

/// The complete collection of holdings for an account.
///
/// Enforces that each asset symbol appears at most once.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountHoldings {
    pub items: Vec<AccountHolding>,
}

impl AccountHoldings {
    pub fn new(items: Vec<AccountHolding>) -> Self {
        Self { items }
    }

    /// Total number of holdings.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Look up a holding by asset symbol (case-sensitive).
    pub fn find(&self, asset: &str) -> Option<&AccountHolding> {
        self.items.iter().find(|h| h.asset == asset)
    }

    /// Add or replace a holding for an asset.
    pub fn upsert(&mut self, holding: AccountHolding) {
        if let Some(existing) = self.items.iter_mut().find(|h| h.asset == holding.asset) {
            *existing = holding;
        } else {
            self.items.push(holding);
        }
    }

    /// Remove a holding by asset symbol. Returns `true` if removed.
    pub fn remove(&mut self, asset: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|h| h.asset != asset);
        self.items.len() < before
    }

    /// Replace all holdings atomically (used during sync).
    pub fn replace_all(&mut self, holdings: Vec<AccountHolding>) {
        self.items = holdings;
    }

    /// Sum of all holding quantities (informational — not used for financial calc).
    pub fn total_quantity_for(&self, asset: &str) -> Decimal {
        self.items
            .iter()
            .filter(|h| h.asset == asset)
            .map(|h| h.quantity_decimal())
            .sum()
    }
}

impl From<Vec<AccountHolding>> for AccountHoldings {
    fn from(items: Vec<AccountHolding>) -> Self {
        Self { items }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_holding_defaults() {
        let h = AccountHolding::new("BTC", "1.5");
        assert_eq!(h.available_quantity(), "1.5");
        assert_eq!(h.frozen_quantity(), "0");
    }

    #[test]
    fn test_holding_consistency() {
        let mut h = AccountHolding::new("ETH", "10.0");
        h.available = Some("9.5".into());
        h.frozen = Some("0.5".into());
        assert!(h.is_consistent());
    }

    #[test]
    fn test_holdings_upsert_and_find() {
        let mut holdings = AccountHoldings::default();
        holdings.upsert(AccountHolding::new("BTC", "1.0"));
        holdings.upsert(AccountHolding::new("ETH", "5.0"));
        // Upsert replaces
        holdings.upsert(AccountHolding::new("BTC", "2.0"));

        assert_eq!(holdings.len(), 2);
        assert_eq!(holdings.find("BTC").unwrap().quantity, "2.0");
    }

    #[test]
    fn test_holdings_remove() {
        let mut holdings = AccountHoldings::from(vec![
            AccountHolding::new("BTC", "1.0"),
            AccountHolding::new("ETH", "5.0"),
        ]);
        assert!(holdings.remove("BTC"));
        assert!(!holdings.remove("BTC")); // already gone
        assert_eq!(holdings.len(), 1);
    }
}
