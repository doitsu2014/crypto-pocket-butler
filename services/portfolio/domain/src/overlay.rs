//! A recorded trade applied to the current holdings until a venue sync reports
//! it (BR6.1). The valuation adds each overlay delta to the C5 holding for the
//! asset, so a trade is reflected immediately and never counted twice.

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{AccountId, PortfolioId, RecordedTradeId, TradeOverlayId};

/// One recorded trade's holding delta.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TradeOverlay {
    id: TradeOverlayId,
    recorded_trade_id: RecordedTradeId,
    portfolio_id: PortfolioId,
    account_id: Option<AccountId>,
    asset_id: String,
    quantity_delta: rust_decimal::Decimal,
    applied_at: DateTime<Utc>,
}

impl TradeOverlay {
    /// Creates an overlay. A zero delta is refused because it would change
    /// nothing.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset and
    /// [`DomainError::Invalid`] for a zero delta.
    pub fn new(
        id: TradeOverlayId,
        recorded_trade_id: RecordedTradeId,
        portfolio_id: PortfolioId,
        account_id: Option<AccountId>,
        asset_id: &str,
        quantity_delta: rust_decimal::Decimal,
        applied_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        if quantity_delta.is_zero() {
            return Err(DomainError::Invalid {
                field: "quantityDelta",
                reason: "must not be zero".to_string(),
            });
        }
        Ok(Self {
            id,
            recorded_trade_id,
            portfolio_id,
            account_id,
            asset_id: asset_id.to_string(),
            quantity_delta,
            applied_at,
        })
    }

    /// Rehydrates a stored overlay.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: TradeOverlayId,
        recorded_trade_id: RecordedTradeId,
        portfolio_id: PortfolioId,
        account_id: Option<AccountId>,
        asset_id: String,
        quantity_delta: rust_decimal::Decimal,
        applied_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            recorded_trade_id,
            portfolio_id,
            account_id,
            asset_id,
            quantity_delta,
            applied_at,
        }
    }

    /// The overlay identifier.
    #[must_use]
    pub fn id(&self) -> TradeOverlayId {
        self.id
    }

    /// The recorded trade it belongs to.
    #[must_use]
    pub fn recorded_trade_id(&self) -> RecordedTradeId {
        self.recorded_trade_id
    }

    /// The portfolio.
    #[must_use]
    pub fn portfolio_id(&self) -> PortfolioId {
        self.portfolio_id
    }

    /// The account, absent when the trade is portfolio-only.
    #[must_use]
    pub fn account_id(&self) -> Option<AccountId> {
        self.account_id
    }

    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The quantity delta: positive for a buy, negative for a sell.
    #[must_use]
    pub fn quantity_delta(&self) -> rust_decimal::Decimal {
        self.quantity_delta
    }

    /// When the overlay was applied.
    #[must_use]
    pub fn applied_at(&self) -> DateTime<Utc> {
        self.applied_at
    }
}
