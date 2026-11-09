//! Transactions: the append-only movement record and its deterministic keys
//! (BR1.1, BR1.2, BR1.3, BR5.2).

use chrono::{DateTime, Duration, SecondsFormat, Timelike, Utc};
use rust_decimal::Decimal;
use uuid::Uuid;

use crate::constants::FUTURE_SKEW_SECONDS;
use crate::error::DomainError;
use crate::ids::{LedgerSubjectId, TransactionId};

/// The movement kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// An acquisition paid for in the base currency.
    Buy,
    /// A disposal.
    Sell,
    /// Assets arriving from an internal transfer.
    TransferIn,
    /// Assets leaving on an internal transfer.
    TransferOut,
    /// A network, venue, or account fee.
    Fee,
}

impl Kind {
    /// The stored spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "buy",
            Self::Sell => "sell",
            Self::TransferIn => "transfer_in",
            Self::TransferOut => "transfer_out",
            Self::Fee => "fee",
        }
    }

    /// Parses the stored spelling.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown kind.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "buy" => Ok(Self::Buy),
            "sell" => Ok(Self::Sell),
            "transfer_in" => Ok(Self::TransferIn),
            "transfer_out" => Ok(Self::TransferOut),
            "fee" => Ok(Self::Fee),
            _ => Err(DomainError::Invalid {
                field: "kind",
                reason: "must be one of buy, sell, transfer_in, transfer_out, fee".to_string(),
            }),
        }
    }

    /// Whether this kind creates a lot when it arrives priced.
    #[must_use]
    pub fn is_acquisition(self) -> bool {
        matches!(self, Self::Buy)
    }

    /// Whether this kind is a disposal.
    #[must_use]
    pub fn is_disposal(self) -> bool {
        matches!(self, Self::Sell)
    }

    /// Whether this kind is a transfer leg.
    #[must_use]
    pub fn is_transfer(self) -> bool {
        matches!(self, Self::TransferIn | Self::TransferOut)
    }
}

/// Where a transaction came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// A connected venue.
    Venue,
    /// An imported CSV row.
    Csv,
    /// A trade the user recorded.
    RecordedTrade,
}

impl Source {
    /// The stored spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Venue => "venue",
            Self::Csv => "csv",
            Self::RecordedTrade => "recorded_trade",
        }
    }

    /// Parses the stored spelling.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown source.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "venue" => Ok(Self::Venue),
            "csv" => Ok(Self::Csv),
            "recorded_trade" => Ok(Self::RecordedTrade),
            _ => Err(DomainError::Invalid {
                field: "source",
                reason: "must be one of venue, csv, recorded_trade".to_string(),
            }),
        }
    }
}

/// Where a unit price came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PriceSource {
    /// The venue reported it.
    Venue,
    /// A CSV row carried it.
    Csv,
    /// The user supplied it.
    User,
}

impl PriceSource {
    /// The stored spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Venue => "venue",
            Self::Csv => "csv",
            Self::User => "user",
        }
    }

    /// Parses the stored spelling.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown price source.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "venue" => Ok(Self::Venue),
            "csv" => Ok(Self::Csv),
            "user" => Ok(Self::User),
            _ => Err(DomainError::Invalid {
                field: "priceSource",
                reason: "must be one of venue, csv, user".to_string(),
            }),
        }
    }
}

/// One append-only ledger entry.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    id: TransactionId,
    subject_id: LedgerSubjectId,
    account_id: Option<Uuid>,
    asset_id: String,
    kind: Kind,
    quantity: Decimal,
    price: Option<Decimal>,
    price_source: Option<PriceSource>,
    currency: String,
    source: Source,
    external_ref: Option<String>,
    canonical_key: String,
    external_key: Option<String>,
    executed_at: DateTime<Utc>,
    recorded_at: DateTime<Utc>,
}

/// Everything a caller must supply to record a transaction.
#[derive(Debug, Clone, PartialEq)]
pub struct NewTransaction {
    /// The mutable subject the transaction belongs to.
    pub subject_id: LedgerSubjectId,
    /// The Supabase Auth subject, used only to build the canonical key.
    pub user_id: String,
    /// The connected account, absent for a user-recorded trade.
    pub account_id: Option<Uuid>,
    /// The asset identifier.
    pub asset_id: String,
    /// The movement kind.
    pub kind: Kind,
    /// The quantity moved; strictly positive.
    pub quantity: Decimal,
    /// The unit price, absent when unknown or on an unpaired transfer.
    pub price: Option<Decimal>,
    /// Where the price came from, when there is one.
    pub price_source: Option<PriceSource>,
    /// The ISO currency the unit price is quoted in.
    pub currency: String,
    /// The source system.
    pub source: Source,
    /// The source system's identifier for the row.
    pub external_ref: Option<String>,
    /// When the movement happened.
    pub executed_at: DateTime<Utc>,
    /// When it was recorded; the current time.
    pub recorded_at: DateTime<Utc>,
}

impl Transaction {
    /// Validates and records a transaction. A future timestamp beyond the skew
    /// tolerance is clamped to the record time (NFR1.7).
    ///
    /// # Errors
    /// Returns [`DomainError`] when a required field is blank, the quantity is
    /// not positive, or the price is negative.
    pub fn record(id: TransactionId, new: NewTransaction) -> Result<Self, DomainError> {
        if new.user_id.trim().is_empty() {
            return Err(DomainError::Required { field: "userId" });
        }
        if new.asset_id.trim().is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        if new.quantity <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "quantity",
                reason: "must be greater than zero".to_string(),
            });
        }
        if let Some(price) = new.price {
            if price < Decimal::ZERO {
                return Err(DomainError::Invalid {
                    field: "price",
                    reason: "must be zero or greater".to_string(),
                });
            }
        }
        if new.kind == Kind::Fee && new.price.is_none() {
            return Err(DomainError::Invalid {
                field: "price",
                reason: "a fee requires a unit price so its cost is not zero".to_string(),
            });
        }
        let currency = crate::fx::normalise_currency(&new.currency);
        if currency.is_empty() {
            return Err(DomainError::Required { field: "currency" });
        }
        let ceiling = new.recorded_at + Duration::seconds(FUTURE_SKEW_SECONDS);
        let executed_at = if new.executed_at > ceiling {
            new.recorded_at
        } else {
            new.executed_at
        };
        let external_ref = new
            .external_ref
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let canonical_key = canonical_key(
            &new.user_id,
            &new.asset_id,
            new.kind,
            new.quantity,
            new.price,
            executed_at,
        );
        let external_key = external_ref.as_ref().map(|reference| {
            format!(
                "{}|{}|{}",
                new.user_id.trim(),
                new.source.as_str(),
                reference
            )
        });
        Ok(Self {
            id,
            subject_id: new.subject_id,
            account_id: new.account_id,
            asset_id: new.asset_id.trim().to_string(),
            kind: new.kind,
            quantity: new.quantity,
            price: new.price,
            price_source: new.price_source,
            currency,
            source: new.source,
            external_ref,
            canonical_key,
            external_key,
            executed_at,
            recorded_at: new.recorded_at,
        })
    }

    /// Rehydrates a stored transaction without re-validating it.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: TransactionId,
        subject_id: LedgerSubjectId,
        account_id: Option<Uuid>,
        asset_id: String,
        kind: Kind,
        quantity: Decimal,
        price: Option<Decimal>,
        price_source: Option<PriceSource>,
        currency: String,
        source: Source,
        external_ref: Option<String>,
        canonical_key: String,
        external_key: Option<String>,
        executed_at: DateTime<Utc>,
        recorded_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            subject_id,
            account_id,
            asset_id,
            kind,
            quantity,
            price,
            price_source,
            currency,
            source,
            external_ref,
            canonical_key,
            external_key,
            executed_at,
            recorded_at,
        }
    }

    /// The transaction identifier.
    #[must_use]
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// The owning subject.
    #[must_use]
    pub fn subject_id(&self) -> LedgerSubjectId {
        self.subject_id
    }

    /// The connected account, when any.
    #[must_use]
    pub fn account_id(&self) -> Option<Uuid> {
        self.account_id
    }

    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The movement kind.
    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    /// The quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// The unit price, when known.
    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        self.price
    }

    /// Where the price came from.
    #[must_use]
    pub fn price_source(&self) -> Option<PriceSource> {
        self.price_source
    }

    /// The ISO currency the price is quoted in.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The source system.
    #[must_use]
    pub fn source(&self) -> Source {
        self.source
    }

    /// The source system's row identifier.
    #[must_use]
    pub fn external_ref(&self) -> Option<&str> {
        self.external_ref.as_deref()
    }

    /// The cross-source identity (BR1.2).
    #[must_use]
    pub fn canonical_key(&self) -> &str {
        &self.canonical_key
    }

    /// The secondary `(user, source, externalRef)` identity (BR1.2).
    #[must_use]
    pub fn external_key(&self) -> Option<&str> {
        self.external_key.as_deref()
    }

    /// When the movement happened.
    #[must_use]
    pub fn executed_at(&self) -> DateTime<Utc> {
        self.executed_at
    }

    /// When it was recorded.
    #[must_use]
    pub fn recorded_at(&self) -> DateTime<Utc> {
        self.recorded_at
    }

    /// Whether the price is missing, which the boundary must render as
    /// price-missing and never as zero (BR1.3).
    #[must_use]
    pub fn price_missing(&self) -> bool {
        self.price.is_none()
    }
}

/// The canonical cross-source identity:
/// `(userId, assetId, kind, quantity, price, executedAt rounded to the minute)`.
#[must_use]
pub fn canonical_key(
    user_id: &str,
    asset_id: &str,
    kind: Kind,
    quantity: Decimal,
    price: Option<Decimal>,
    executed_at: DateTime<Utc>,
) -> String {
    let minute = executed_at
        .with_second(0)
        .and_then(|value| value.with_nanosecond(0))
        .unwrap_or(executed_at)
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let price = price.map_or_else(|| "none".to_string(), canonical_decimal);
    format!(
        "{}|{}|{}|{}|{}|{}",
        user_id.trim(),
        asset_id.trim(),
        kind.as_str(),
        canonical_decimal(quantity),
        price,
        minute
    )
}

/// A decimal's canonical text: trailing zeros stripped, never binary floating
/// point.
#[must_use]
pub fn canonical_decimal(value: Decimal) -> String {
    value.normalize().to_string()
}
