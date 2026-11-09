//! Target allocations (BR3.1–BR3.3): a per-asset percentage and the preview of
//! a set that need not sum to 100.

use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::error::DomainError;
use crate::ids::{PortfolioId, TargetAllocationId};

/// The upper bound of a target percentage.
pub const MAX_TARGET_PERCENT: Decimal = Decimal::ONE_HUNDRED;

/// The most decimal places stored for a target or threshold.
pub const MAX_PERCENT_SCALE: u32 = 2;

/// Trims a decimal to two places with midpoint rounding away from zero.
#[must_use]
pub fn round_percent(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(MAX_PERCENT_SCALE, RoundingStrategy::MidpointAwayFromZero)
}

fn validate_percent(
    field: &'static str,
    value: Decimal,
    max: Decimal,
) -> Result<Decimal, DomainError> {
    if value.is_sign_negative() || value > max {
        return Err(DomainError::Invalid {
            field,
            reason: format!("must be between 0 and {max}"),
        });
    }
    if value.scale() > MAX_PERCENT_SCALE {
        return Err(DomainError::Invalid {
            field,
            reason: "has more than two decimal places".to_string(),
        });
    }
    Ok(round_percent(value))
}

/// Validates a target's asset and percentage without allocating identifiers.
///
/// # Errors
/// Returns [`DomainError::Required`] for a blank asset and
/// [`DomainError::Invalid`] for an out-of-range percentage.
pub fn validate_target(asset_id: &str, target_percent: Decimal) -> Result<(), DomainError> {
    if asset_id.trim().is_empty() {
        return Err(DomainError::Required { field: "assetId" });
    }
    validate_percent("targetPercent", target_percent, MAX_TARGET_PERCENT).map(|_| ())
}

/// The target percentage for one asset in one portfolio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetAllocation {
    id: TargetAllocationId,
    portfolio_id: PortfolioId,
    asset_id: String,
    target_percent: Decimal,
    updated_at: DateTime<Utc>,
}

impl TargetAllocation {
    /// Creates a target. The percentage is between 0 and 100 with at most two
    /// decimals.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset and
    /// [`DomainError::Invalid`] for an out-of-range percentage.
    pub fn new(
        id: TargetAllocationId,
        portfolio_id: PortfolioId,
        asset_id: &str,
        target_percent: Decimal,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        Ok(Self {
            id,
            portfolio_id,
            asset_id: asset_id.to_string(),
            target_percent: validate_percent("targetPercent", target_percent, MAX_TARGET_PERCENT)?,
            updated_at: now,
        })
    }

    /// Rehydrates a stored target.
    #[must_use]
    pub fn from_stored(
        id: TargetAllocationId,
        portfolio_id: PortfolioId,
        asset_id: String,
        target_percent: Decimal,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            asset_id,
            target_percent,
            updated_at,
        }
    }

    /// The target identifier.
    #[must_use]
    pub fn id(&self) -> TargetAllocationId {
        self.id
    }

    /// The portfolio.
    #[must_use]
    pub fn portfolio_id(&self) -> PortfolioId {
        self.portfolio_id
    }

    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The target percentage.
    #[must_use]
    pub fn target_percent(&self) -> Decimal {
        self.target_percent
    }

    /// When the target last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }
}

/// The preview of a target set (BR3.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetPreview {
    /// The sum of the target percentages.
    pub sum: Decimal,
    /// How far the sum is from 100: positive when under-allocated.
    pub difference: Decimal,
}

/// Refuses a guardrail whose asset has no matching target (BR3.4).
///
/// # Errors
/// Returns [`DomainError::Invalid`] when the asset is not in the saved target
/// set.
pub fn require_target_for_guardrail(
    guardrail_asset: &str,
    target_assets: &[String],
) -> Result<(), DomainError> {
    if target_assets.iter().any(|asset| asset == guardrail_asset) {
        Ok(())
    } else {
        Err(DomainError::Invalid {
            field: "guardrailPoints",
            reason: "a guardrail requires a target for the same asset".to_string(),
        })
    }
}

/// Sums a target set and reports the difference from 100. A set need not total
/// 100; the difference is shown before saving.
#[must_use]
pub fn preview_targets(targets: &[TargetAllocation]) -> TargetPreview {
    let sum = targets.iter().fold(Decimal::ZERO, |total, target| {
        total + target.target_percent()
    });
    TargetPreview {
        sum: round_percent(sum),
        difference: round_percent(MAX_TARGET_PERCENT - sum),
    }
}
