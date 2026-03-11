/// Portfolio value objects: TargetAllocation and Guardrails

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Target allocation defines the desired percentage weight for each asset.
///
/// # Invariants
/// - All weights must be non-negative.
/// - The sum of all weights should equal 1.0 (100 %).
///   Call `validate()` to enforce this.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetAllocation {
    /// Map from asset symbol to target weight in the range [0.0, 1.0].
    pub weights: HashMap<String, Decimal>,
}

impl TargetAllocation {
    pub fn new(weights: HashMap<String, Decimal>) -> Self {
        Self { weights }
    }

    /// Returns the target weight for `asset`, or `Decimal::ZERO` if not set.
    pub fn get_weight(&self, asset: &str) -> Decimal {
        self.weights.get(asset).copied().unwrap_or(Decimal::ZERO)
    }

    /// Validate that all weights are non-negative and sum to 1.0.
    ///
    /// We allow a tolerance of 0.001 to accommodate rounding.
    pub fn validate(&self) -> Result<(), TargetAllocationError> {
        let mut total = Decimal::ZERO;
        for (asset, &weight) in &self.weights {
            if weight < Decimal::ZERO {
                return Err(TargetAllocationError::NegativeWeight(asset.clone()));
            }
            total += weight;
        }
        if self.weights.is_empty() {
            return Ok(()); // Empty allocation is valid (no target set)
        }
        let tolerance = Decimal::new(1, 3); // 0.001
        let diff = (total - Decimal::ONE).abs();
        if diff > tolerance {
            return Err(TargetAllocationError::WeightsDontSumToOne(total));
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum TargetAllocationError {
    #[error("weight for asset '{0}' is negative")]
    NegativeWeight(String),
    #[error("weights sum to {0}, expected 1.0")]
    WeightsDontSumToOne(Decimal),
}

/// Guardrails define risk constraints for a portfolio.
///
/// When the actual allocation deviates from the target by more than
/// `max_deviation`, it is considered violated and a rebalance may be triggered.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Guardrails {
    /// Maximum allowed deviation from target weight (e.g. `Decimal::new(5, 2)` = 5 %).
    pub max_deviation: Option<Decimal>,
    /// If set, only these asset symbols are permitted in the portfolio.
    pub allowed_assets: Vec<String>,
}

impl Guardrails {
    pub fn new(max_deviation: Option<Decimal>, allowed_assets: Vec<String>) -> Self {
        Self {
            max_deviation,
            allowed_assets,
        }
    }

    /// Returns `true` if the portfolio violates guardrail constraints.
    ///
    /// Checks that no asset in `actual` deviates from `target` by more than
    /// `max_deviation`. Assets not in the target are treated as target = 0.
    pub fn is_violated(
        &self,
        target: &TargetAllocation,
        actual: &HashMap<String, Decimal>,
    ) -> bool {
        let Some(max_dev) = self.max_deviation else {
            return false;
        };
        for (asset, &actual_weight) in actual {
            let target_weight = target.get_weight(asset);
            if (actual_weight - target_weight).abs() > max_dev {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_allocation_valid() {
        let mut weights = HashMap::new();
        weights.insert("BTC".into(), Decimal::new(6, 1)); // 0.6
        weights.insert("ETH".into(), Decimal::new(4, 1)); // 0.4
        let ta = TargetAllocation::new(weights);
        assert!(ta.validate().is_ok());
    }

    #[test]
    fn test_target_allocation_invalid_sum() {
        let mut weights = HashMap::new();
        weights.insert("BTC".into(), Decimal::new(7, 1));
        weights.insert("ETH".into(), Decimal::new(4, 1));
        let ta = TargetAllocation::new(weights);
        assert!(matches!(
            ta.validate(),
            Err(TargetAllocationError::WeightsDontSumToOne(_))
        ));
    }

    #[test]
    fn test_target_allocation_negative_weight() {
        let mut weights = HashMap::new();
        weights.insert("BTC".into(), Decimal::new(-1, 1));
        let ta = TargetAllocation::new(weights);
        assert!(matches!(
            ta.validate(),
            Err(TargetAllocationError::NegativeWeight(_))
        ));
    }

    #[test]
    fn test_guardrails_not_violated() {
        let mut target_weights = HashMap::new();
        target_weights.insert("BTC".into(), Decimal::new(6, 1));
        let target = TargetAllocation::new(target_weights);

        let mut actual = HashMap::new();
        actual.insert("BTC".into(), Decimal::new(62, 2)); // 0.62 — within 5% of 0.6

        let guardrails = Guardrails::new(Some(Decimal::new(5, 2)), vec![]);
        assert!(!guardrails.is_violated(&target, &actual));
    }

    #[test]
    fn test_guardrails_violated() {
        let mut target_weights = HashMap::new();
        target_weights.insert("BTC".into(), Decimal::new(6, 1));
        let target = TargetAllocation::new(target_weights);

        let mut actual = HashMap::new();
        actual.insert("BTC".into(), Decimal::new(8, 1)); // 0.8 — 20% deviation

        let guardrails = Guardrails::new(Some(Decimal::new(5, 2)), vec![]);
        assert!(guardrails.is_violated(&target, &actual));
    }
}
