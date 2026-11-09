//! Recorded trades (BR6.1–BR6.2): a user-confirmed executed trade that becomes
//! a ledger entry, with a user-supplied price labelled as such.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::DomainError;
use crate::ids::{PortfolioId, RecordedTradeId, UserId};
use crate::intent::OrderSide;

/// Where a recorded trade's price came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceSource {
    /// The venue's price.
    Venue,
    /// A price the user supplied because the venue has none.
    User,
}

impl PriceSource {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Venue => "venue",
            Self::User => "user",
        }
    }

    /// Parses a price-source label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown source.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "venue" => Ok(Self::Venue),
            "user" => Ok(Self::User),
            _ => Err(DomainError::Invalid {
                field: "priceSource",
                reason: "must be venue or user".to_string(),
            }),
        }
    }
}

/// A user-confirmed executed trade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedTrade {
    id: RecordedTradeId,
    portfolio_id: PortfolioId,
    user_id: UserId,
    asset_id: String,
    side: OrderSide,
    quantity: Decimal,
    price: Decimal,
    price_source: PriceSource,
    executed_at: DateTime<Utc>,
    created_at: DateTime<Utc>,
    idempotency_key: String,
    ledger_transaction_id: Option<String>,
}

impl RecordedTrade {
    /// Creates a recorded trade. A user-supplied price is labelled `user`.
    ///
    /// # Errors
    /// Returns a domain refusal for a blank asset, a non-positive quantity, a
    /// negative price, or a blank idempotency key.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: RecordedTradeId,
        portfolio_id: PortfolioId,
        user_id: UserId,
        asset_id: &str,
        side: OrderSide,
        quantity: Decimal,
        price: Decimal,
        price_source: PriceSource,
        executed_at: DateTime<Utc>,
        now: DateTime<Utc>,
        idempotency_key: &str,
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
        if price.is_sign_negative() {
            return Err(DomainError::Invalid {
                field: "price",
                reason: "must be zero or greater".to_string(),
            });
        }
        let idempotency_key = idempotency_key.trim();
        if idempotency_key.is_empty() {
            return Err(DomainError::Required {
                field: "idempotencyKey",
            });
        }
        Ok(Self {
            id,
            portfolio_id,
            user_id,
            asset_id: asset_id.to_string(),
            side,
            quantity,
            price,
            price_source,
            executed_at,
            created_at: now,
            idempotency_key: idempotency_key.to_string(),
            ledger_transaction_id: None,
        })
    }

    /// Rehydrates a stored trade.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: RecordedTradeId,
        portfolio_id: PortfolioId,
        user_id: UserId,
        asset_id: String,
        side: OrderSide,
        quantity: Decimal,
        price: Decimal,
        price_source: PriceSource,
        executed_at: DateTime<Utc>,
        created_at: DateTime<Utc>,
        idempotency_key: String,
        ledger_transaction_id: Option<String>,
    ) -> Self {
        Self {
            id,
            portfolio_id,
            user_id,
            asset_id,
            side,
            quantity,
            price,
            price_source,
            executed_at,
            created_at,
            idempotency_key,
            ledger_transaction_id,
        }
    }

    /// Attaches the Ledger transaction id.
    #[must_use]
    pub fn with_ledger_transaction(mut self, transaction_id: Option<String>) -> Self {
        self.ledger_transaction_id = transaction_id;
        self
    }

    /// The trade identifier.
    #[must_use]
    pub fn id(&self) -> RecordedTradeId {
        self.id
    }

    /// The portfolio.
    #[must_use]
    pub fn portfolio_id(&self) -> PortfolioId {
        self.portfolio_id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
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

    /// The quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// The unit price.
    #[must_use]
    pub fn price(&self) -> Decimal {
        self.price
    }

    /// Where the price came from.
    #[must_use]
    pub fn price_source(&self) -> PriceSource {
        self.price_source
    }

    /// When the trade executed.
    #[must_use]
    pub fn executed_at(&self) -> DateTime<Utc> {
        self.executed_at
    }

    /// When the trade was recorded.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// The client idempotency key.
    #[must_use]
    pub fn idempotency_key(&self) -> &str {
        &self.idempotency_key
    }

    /// The Ledger transaction id, when the write succeeded.
    #[must_use]
    pub fn ledger_transaction_id(&self) -> Option<&str> {
        self.ledger_transaction_id.as_deref()
    }

    /// The quantity delta this trade applies to the holdings.
    #[must_use]
    pub fn quantity_delta(&self) -> Decimal {
        match self.side {
            OrderSide::Buy => self.quantity,
            OrderSide::Sell => -self.quantity,
        }
    }
}

/// The canonical key used to match a recorded trade with the venue transaction a
/// C5 sync later reports: asset, side, quantity, and execution instant.
#[must_use]
pub fn canonical_trade_key(
    asset_id: &str,
    side: OrderSide,
    quantity: Decimal,
    executed_at: DateTime<Utc>,
) -> String {
    format!(
        "{}:{}:{}:{}",
        asset_id.trim(),
        side.as_str(),
        quantity.normalize(),
        executed_at.timestamp_millis()
    )
}
