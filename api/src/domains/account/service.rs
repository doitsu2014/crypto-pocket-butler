/// Account domain services
///
/// Domain services implement business logic that spans multiple aggregates or
/// cannot naturally belong to a single aggregate root.
///
/// The actual orchestration (calling connectors, scheduling jobs) lives in the
/// application layer (`crate::application::services`). This module contains
/// only pure domain logic.

use super::{aggregate::AccountError, entities::AccountHolding};

/// Calculate the net change between old and new holdings lists.
///
/// Returns:
/// - `added`: holdings present in `new` but not in `old`
/// - `removed`: asset symbols present in `old` but not in `new`
/// - `updated`: holdings whose quantity changed
pub fn diff_holdings(
    old: &[AccountHolding],
    new: &[AccountHolding],
) -> HoldingsDiff {
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut updated = Vec::new();

    for new_holding in new {
        match old.iter().find(|h| h.asset == new_holding.asset) {
            None => added.push(new_holding.clone()),
            Some(old_holding) if old_holding.quantity != new_holding.quantity => {
                updated.push(new_holding.clone());
            }
            _ => {}
        }
    }

    for old_holding in old {
        if !new.iter().any(|h| h.asset == old_holding.asset) {
            removed.push(old_holding.asset.clone());
        }
    }

    HoldingsDiff { added, removed, updated }
}

/// Result of comparing two holdings snapshots.
#[derive(Debug, Default)]
pub struct HoldingsDiff {
    pub added: Vec<AccountHolding>,
    pub removed: Vec<String>,
    pub updated: Vec<AccountHolding>,
}

/// Validate that an `account_type` string is a recognised value.
pub fn validate_account_type(s: &str) -> Result<(), AccountError> {
    use super::value_objects::AccountType;
    AccountType::from_str(s)
        .map(|_| ())
        .ok_or_else(|| AccountError::PersistenceError(format!("invalid account_type: {s}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_holdings_added() {
        let old = vec![AccountHolding::new("BTC", "1.0")];
        let new = vec![
            AccountHolding::new("BTC", "1.0"),
            AccountHolding::new("ETH", "5.0"),
        ];
        let diff = diff_holdings(&old, &new);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.added[0].asset, "ETH");
        assert!(diff.removed.is_empty());
        assert!(diff.updated.is_empty());
    }

    #[test]
    fn test_diff_holdings_removed() {
        let old = vec![
            AccountHolding::new("BTC", "1.0"),
            AccountHolding::new("ETH", "5.0"),
        ];
        let new = vec![AccountHolding::new("BTC", "1.0")];
        let diff = diff_holdings(&old, &new);
        assert!(diff.added.is_empty());
        assert_eq!(diff.removed, vec!["ETH"]);
        assert!(diff.updated.is_empty());
    }

    #[test]
    fn test_diff_holdings_updated() {
        let old = vec![AccountHolding::new("BTC", "1.0")];
        let new = vec![AccountHolding::new("BTC", "2.0")];
        let diff = diff_holdings(&old, &new);
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert_eq!(diff.updated.len(), 1);
        assert_eq!(diff.updated[0].quantity, "2.0");
    }
}
