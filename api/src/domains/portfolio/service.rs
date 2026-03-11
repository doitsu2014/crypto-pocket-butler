/// Portfolio domain services
///
/// Pure domain logic for portfolio operations that spans multiple entities.

use rust_decimal::Decimal;
use std::collections::HashMap;

/// Compute portfolio weights from a map of asset → USD value.
///
/// # Returns
/// A map of asset symbol to weight in the range [0.0, 1.0].
/// If total value is zero, all weights are zero.
///
/// # Invariants
/// - Weights sum to 1.0 (within floating-point tolerance).
/// - Each weight is in [0.0, 1.0].
pub fn compute_weights(values: &HashMap<String, Decimal>) -> HashMap<String, Decimal> {
    let total: Decimal = values.values().sum();
    if total.is_zero() {
        return values.keys().map(|k| (k.clone(), Decimal::ZERO)).collect();
    }
    values
        .iter()
        .map(|(asset, &value)| (asset.clone(), value / total))
        .collect()
}

/// Verify that a set of weights sums to 1.0 within a tolerance.
pub fn weights_sum_to_one(weights: &HashMap<String, Decimal>) -> bool {
    let total: Decimal = weights.values().sum();
    let tolerance = Decimal::new(1, 4); // 0.0001
    (total - Decimal::ONE).abs() <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_weights_basic() {
        let mut values = HashMap::new();
        values.insert("BTC".into(), Decimal::new(75000, 0));
        values.insert("ETH".into(), Decimal::new(25000, 0));

        let weights = compute_weights(&values);
        assert_eq!(weights["BTC"], Decimal::new(75, 2)); // 0.75
        assert_eq!(weights["ETH"], Decimal::new(25, 2)); // 0.25
        assert!(weights_sum_to_one(&weights));
    }

    #[test]
    fn test_compute_weights_zero_total() {
        let mut values = HashMap::new();
        values.insert("BTC".into(), Decimal::ZERO);
        let weights = compute_weights(&values);
        assert_eq!(weights["BTC"], Decimal::ZERO);
    }

    #[test]
    fn test_weights_sum_to_one() {
        let mut weights = HashMap::new();
        weights.insert("BTC".into(), Decimal::new(6, 1));
        weights.insert("ETH".into(), Decimal::new(4, 1));
        assert!(weights_sum_to_one(&weights));
    }

    #[test]
    fn test_weights_dont_sum_to_one() {
        let mut weights = HashMap::new();
        weights.insert("BTC".into(), Decimal::new(5, 1));
        weights.insert("ETH".into(), Decimal::new(3, 1));
        assert!(!weights_sum_to_one(&weights));
    }
}
