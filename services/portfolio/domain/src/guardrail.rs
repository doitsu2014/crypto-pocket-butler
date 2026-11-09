//! Guardrails (BR3.4, BR7.1): a per-asset drift threshold in percentage points
//! with the crossing state used to notify exactly once per crossing.

use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::error::DomainError;
use crate::ids::{GuardrailId, PortfolioId, UserId};

/// The most decimal places stored for a threshold.
pub const MAX_THRESHOLD_SCALE: u32 = 2;

/// A guardrail to save with a target set. It becomes a [`Guardrail`] once the
/// matching target is known to exist (BR3.4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardrailDraft {
    /// The asset the guardrail watches.
    pub asset_id: String,
    /// The drift threshold in percentage points, greater than zero.
    pub threshold_points: Decimal,
}

impl GuardrailDraft {
    /// Validates a draft.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset and
    /// [`DomainError::Invalid`] when the threshold is not greater than zero.
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.asset_id.trim().is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        if self.threshold_points <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "guardrailPoints",
                reason: "must be greater than zero".to_string(),
            });
        }
        if self.threshold_points.scale() > MAX_THRESHOLD_SCALE {
            return Err(DomainError::Invalid {
                field: "guardrailPoints",
                reason: "has more than two decimal places".to_string(),
            });
        }
        Ok(())
    }
}

/// An optional per-asset drift threshold with its crossing state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guardrail {
    id: GuardrailId,
    portfolio_id: PortfolioId,
    asset_id: String,
    threshold_points: Decimal,
    is_outside: bool,
    last_crossing_key: Option<String>,
    crossed_at: Option<DateTime<Utc>>,
    updated_at: DateTime<Utc>,
}

impl Guardrail {
    /// Creates a guardrail in the inside state.
    ///
    /// # Errors
    /// Returns a domain refusal for an invalid asset or threshold.
    pub fn new(
        id: GuardrailId,
        portfolio_id: PortfolioId,
        asset_id: &str,
        threshold_points: Decimal,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let draft = GuardrailDraft {
            asset_id: asset_id.to_string(),
            threshold_points,
        };
        draft.validate()?;
        Ok(Self {
            id,
            portfolio_id,
            asset_id: draft.asset_id.trim().to_string(),
            threshold_points: threshold_points.round_dp_with_strategy(
                MAX_THRESHOLD_SCALE,
                RoundingStrategy::MidpointAwayFromZero,
            ),
            is_outside: false,
            last_crossing_key: None,
            crossed_at: None,
            updated_at: now,
        })
    }

    /// Rehydrates a stored guardrail.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: GuardrailId,
        portfolio_id: PortfolioId,
        asset_id: String,
        threshold_points: Decimal,
        is_outside: bool,
        last_crossing_key: Option<String>,
        crossed_at: Option<DateTime<Utc>>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            asset_id,
            threshold_points,
            is_outside,
            last_crossing_key,
            crossed_at,
            updated_at,
        }
    }

    /// The guardrail identifier.
    #[must_use]
    pub fn id(&self) -> GuardrailId {
        self.id
    }

    /// The portfolio.
    #[must_use]
    pub fn portfolio_id(&self) -> PortfolioId {
        self.portfolio_id
    }

    /// The watched asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The drift threshold in percentage points.
    #[must_use]
    pub fn threshold_points(&self) -> Decimal {
        self.threshold_points
    }

    /// Whether the drift currently sits outside the guardrail.
    #[must_use]
    pub fn is_outside(&self) -> bool {
        self.is_outside
    }

    /// The C9 idempotency key of the active crossing.
    #[must_use]
    pub fn last_crossing_key(&self) -> Option<&str> {
        self.last_crossing_key.as_deref()
    }

    /// When the active crossing began.
    #[must_use]
    pub fn crossed_at(&self) -> Option<DateTime<Utc>> {
        self.crossed_at
    }

    /// When the guardrail last changed.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Evaluates the guardrail against a drift in percentage points:
    /// `inside -> outside` crosses, staying outside retains the key, and
    /// returning inside clears the state (BR7.1).
    #[must_use]
    pub fn evaluate(
        &self,
        drift_points: Decimal,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> (Self, GuardrailTransition) {
        let outside = drift_points.abs() > self.threshold_points;
        let mut updated = self.clone();
        updated.updated_at = now;
        if outside && !self.is_outside {
            let key = crossing_key(user_id, self.portfolio_id, &self.asset_id, self.id, now);
            updated.is_outside = true;
            updated.crossed_at = Some(now);
            updated.last_crossing_key = Some(key.clone());
            (updated, GuardrailTransition::Crossed { key })
        } else if outside {
            (
                updated,
                GuardrailTransition::Retained {
                    key: self.last_crossing_key.clone().unwrap_or_default(),
                },
            )
        } else if self.is_outside {
            updated.is_outside = false;
            updated.crossed_at = None;
            updated.last_crossing_key = None;
            (updated, GuardrailTransition::Cleared)
        } else {
            (updated, GuardrailTransition::None)
        }
    }
}

/// The outcome of one guardrail evaluation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardrailTransition {
    /// Still inside; nothing to do.
    None,
    /// Crossed from inside; notify with this key.
    Crossed {
        /// The C9 idempotency key, keyed by the crossing instant.
        key: String,
    },
    /// Still outside; the active key is reused and nothing is notified.
    Retained {
        /// The active crossing key.
        key: String,
    },
    /// Returned inside; the state is cleared.
    Cleared,
}

/// The stable C9 idempotency key for one crossing: user, portfolio, asset,
/// guardrail, and crossing instant.
#[must_use]
pub fn crossing_key(
    user_id: UserId,
    portfolio_id: PortfolioId,
    asset_id: &str,
    guardrail_id: GuardrailId,
    crossed_at: DateTime<Utc>,
) -> String {
    format!(
        "drift:{}:{}:{}:{}:{}",
        user_id,
        portfolio_id,
        asset_id,
        guardrail_id,
        crossed_at.timestamp_millis()
    )
}
