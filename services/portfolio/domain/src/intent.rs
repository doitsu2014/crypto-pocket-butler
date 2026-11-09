//! Order intents (BR5.1–BR5.3): advisory buy or sell intentions with a
//! projected allocation and drift, and an explicit shortfall when a sell
//! exceeds the holdings. No external order call exists.

use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::error::DomainError;
use crate::ids::{OrderIntentId, PortfolioId};
use crate::valuation::Valuation;

/// The number of decimal places kept for a projected allocation.
const PROJECTION_SCALE: u32 = 1;

fn round_one(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(PROJECTION_SCALE, RoundingStrategy::MidpointAwayFromZero)
}

/// The side of an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderSide {
    /// Buy.
    Buy,
    /// Sell.
    Sell,
}

impl OrderSide {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
        }
    }

    /// Parses a side label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown side.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            _ => Err(DomainError::Invalid {
                field: "side",
                reason: "must be buy or sell".to_string(),
            }),
        }
    }
}

/// The status of an intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntentStatus {
    /// Recorded and open.
    Pending,
    /// Cancelled; still visible.
    Cancelled,
}

impl IntentStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Cancelled => "cancelled",
        }
    }
}

/// A projected post-trade allocation and drift, with any shortfall.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntentProjection {
    /// The projected allocation as a percentage.
    pub projected_allocation: Decimal,
    /// The projected drift in percentage points, when targets exist.
    pub projected_drift_points: Option<Decimal>,
    /// How far a sell exceeds the known quantity, when it does.
    pub shortfall: Option<Decimal>,
}

/// Projects the post-trade allocation and drift (BR5.2). A sell beyond the known
/// holdings states the shortfall and projects only the available quantity.
///
/// # Errors
/// Returns [`DomainError::Invalid`] for a non-positive quantity and
/// [`DomainError::Denied`] when the portfolio or the asset has no usable price.
pub fn project_intent(
    valuation: &Valuation,
    asset_id: &str,
    side: OrderSide,
    quantity: Decimal,
    unit_price_in_base: Option<Decimal>,
) -> Result<IntentProjection, DomainError> {
    if quantity <= Decimal::ZERO {
        return Err(DomainError::Invalid {
            field: "quantity",
            reason: "must be greater than zero".to_string(),
        });
    }
    let total = valuation.total_value().ok_or_else(|| DomainError::Denied {
        reason: "the portfolio has no priced value to project against".to_string(),
    })?;
    if total <= Decimal::ZERO {
        return Err(DomainError::Denied {
            reason: "the portfolio has no priced value to project against".to_string(),
        });
    }
    let line = valuation.line(asset_id);
    let current_value = line.and_then(|line| line.value()).unwrap_or(Decimal::ZERO);
    let current_quantity = line.map_or(Decimal::ZERO, |line| line.quantity());
    let unit_price = if current_quantity > Decimal::ZERO {
        current_value / current_quantity
    } else {
        unit_price_in_base.ok_or_else(|| DomainError::Denied {
            reason: "the asset has no price to project against".to_string(),
        })?
    };
    if unit_price <= Decimal::ZERO {
        return Err(DomainError::Denied {
            reason: "the asset has no price to project against".to_string(),
        });
    }

    let (new_asset_value, new_total, shortfall) = match side {
        OrderSide::Buy => {
            let delta = quantity * unit_price;
            (current_value + delta, total + delta, None)
        }
        OrderSide::Sell => {
            let shortfall = if quantity > current_quantity {
                Some(quantity - current_quantity)
            } else {
                None
            };
            let sold = quantity.min(current_quantity);
            let delta = sold * unit_price;
            (
                (current_value - delta).max(Decimal::ZERO),
                (total - delta).max(Decimal::ZERO),
                shortfall,
            )
        }
    };

    let projected_allocation = if new_total > Decimal::ZERO {
        round_one(new_asset_value / new_total * Decimal::ONE_HUNDRED)
    } else {
        Decimal::ZERO
    };
    let projected_drift_points = line
        .and_then(|line| line.target_percent())
        .map(|target| round_one(projected_allocation - target));
    Ok(IntentProjection {
        projected_allocation,
        projected_drift_points,
        shortfall,
    })
}

/// A user's advisory buy or sell intention.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderIntent {
    id: OrderIntentId,
    portfolio_id: PortfolioId,
    asset_id: String,
    side: OrderSide,
    quantity: Decimal,
    projected_allocation: Decimal,
    projected_drift_points: Option<Decimal>,
    shortfall: Option<Decimal>,
    status: IntentStatus,
    created_at: DateTime<Utc>,
    cancelled_at: Option<DateTime<Utc>>,
}

impl OrderIntent {
    /// Records a pending intent from its projection.
    ///
    /// # Errors
    /// Returns a domain refusal for a blank asset or a non-positive quantity.
    pub fn new(
        id: OrderIntentId,
        portfolio_id: PortfolioId,
        asset_id: &str,
        side: OrderSide,
        quantity: Decimal,
        projection: &IntentProjection,
        now: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        if quantity <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "quantity",
                reason: "must be greater than zero".to_string(),
            });
        }
        Ok(Self {
            id,
            portfolio_id,
            asset_id: asset_id.to_string(),
            side,
            quantity,
            projected_allocation: projection.projected_allocation,
            projected_drift_points: projection.projected_drift_points,
            shortfall: projection.shortfall,
            status: IntentStatus::Pending,
            created_at: now,
            cancelled_at: None,
        })
    }

    /// Rehydrates a stored intent.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: OrderIntentId,
        portfolio_id: PortfolioId,
        asset_id: String,
        side: OrderSide,
        quantity: Decimal,
        projected_allocation: Decimal,
        projected_drift_points: Option<Decimal>,
        shortfall: Option<Decimal>,
        status: IntentStatus,
        created_at: DateTime<Utc>,
        cancelled_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            asset_id,
            side,
            quantity,
            projected_allocation,
            projected_drift_points,
            shortfall,
            status,
            created_at,
            cancelled_at,
        }
    }

    /// The intent identifier.
    #[must_use]
    pub fn id(&self) -> OrderIntentId {
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

    /// The side.
    #[must_use]
    pub fn side(&self) -> OrderSide {
        self.side
    }

    /// The requested quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// The projected allocation.
    #[must_use]
    pub fn projected_allocation(&self) -> Decimal {
        self.projected_allocation
    }

    /// The projected drift.
    #[must_use]
    pub fn projected_drift_points(&self) -> Option<Decimal> {
        self.projected_drift_points
    }

    /// The shortfall for a sell beyond holdings.
    #[must_use]
    pub fn shortfall(&self) -> Option<Decimal> {
        self.shortfall
    }

    /// The status.
    #[must_use]
    pub fn status(&self) -> IntentStatus {
        self.status
    }

    /// When the intent was recorded.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// When the intent was cancelled.
    #[must_use]
    pub fn cancelled_at(&self) -> Option<DateTime<Utc>> {
        self.cancelled_at
    }

    /// Cancels a pending intent; a cancelled intent stays visible (BR5.3).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when it is already cancelled.
    pub fn cancel(&mut self, now: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status == IntentStatus::Cancelled {
            return Err(DomainError::InvalidTransition {
                entity: "order intent",
                from: "cancelled",
                to: "cancelled",
            });
        }
        self.status = IntentStatus::Cancelled;
        self.cancelled_at = Some(now);
        Ok(())
    }
}
