//! Price observations and served quotes (BR1.1–BR1.5, BR2.1–BR2.2, BR4.4).
//!
//! A reading is accepted only when its price is greater than zero and its
//! `observed_at` is not more than the clock tolerance ahead of `received_at`.
//! Everything else is stored as a rejected observation for diagnosis and is
//! never selected.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::str::FromStr;

use crate::constants::{CLOCK_TOLERANCE_SECONDS, MAX_REJECTION_REASON_CHARS};
use crate::error::DomainError;
use crate::freshness::Freshness;
use crate::ids::ObservationId;
use crate::secrets::looks_like_secret;
use crate::tier::validate_asset_id;

/// The most decimal places stored for a price.
pub const MAX_PRICE_SCALE: u32 = 18;
/// The longest source name accepted.
pub const MAX_SOURCE_NAME_CHARS: usize = 80;

/// Parses a plain decimal price string.
///
/// # Errors
/// Returns [`DomainError::Required`] for an empty value and
/// [`DomainError::Invalid`] for scientific notation, a non-decimal value, or
/// more than [`MAX_PRICE_SCALE`] decimal places.
pub fn parse_price(raw: &str) -> Result<Decimal, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "price" });
    }
    if value.contains('e') || value.contains('E') {
        return Err(DomainError::Invalid {
            field: "price",
            reason: "must be a plain decimal string".to_string(),
        });
    }
    let price = Decimal::from_str(value).map_err(|_| DomainError::Invalid {
        field: "price",
        reason: "must be a decimal number".to_string(),
    })?;
    if price.scale() > MAX_PRICE_SCALE {
        return Err(DomainError::Invalid {
            field: "price",
            reason: "has more than 18 decimal places".to_string(),
        });
    }
    Ok(price)
}

/// Whether a source timestamp is beyond the clock tolerance.
#[must_use]
pub fn is_clock_skewed(observed_at: DateTime<Utc>, received_at: DateTime<Utc>) -> bool {
    observed_at > received_at + chrono::Duration::seconds(CLOCK_TOLERANCE_SECONDS)
}

/// The state an observation is stored in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationStatus {
    /// The reading may be selected.
    Accepted,
    /// The reading is kept for diagnosis and never selected.
    Rejected,
}

impl ObservationStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Rejected => "rejected",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be accepted or rejected".to_string(),
            }),
        }
    }
}

/// One reading of an asset price from one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceObservation {
    id: ObservationId,
    asset_id: String,
    source_name: String,
    price: Decimal,
    currency: String,
    observed_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
    status: ObservationStatus,
    rejection_reason: Option<String>,
}

impl PriceObservation {
    /// Records an accepted reading.
    ///
    /// # Errors
    /// Returns a domain refusal for a blank identity field, a non-positive
    /// price, a malformed currency, or a clock-skewed timestamp.
    #[allow(clippy::too_many_arguments)]
    pub fn accept(
        id: ObservationId,
        asset_id: &str,
        source_name: &str,
        price: Decimal,
        currency: &str,
        observed_at: DateTime<Utc>,
        received_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let (asset_id, source_name, currency) = validate_identity(asset_id, source_name, currency)?;
        if price <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "price",
                reason: "must be greater than zero".to_string(),
            });
        }
        if is_clock_skewed(observed_at, received_at) {
            return Err(DomainError::Invalid {
                field: "observedAt",
                reason: "is beyond the clock tolerance".to_string(),
            });
        }
        Ok(Self {
            id,
            asset_id,
            source_name,
            price,
            currency,
            observed_at,
            received_at,
            status: ObservationStatus::Accepted,
            rejection_reason: None,
        })
    }

    /// Records a rejected reading. The reading is kept for diagnosis and is
    /// never selected (BR4.4).
    ///
    /// # Errors
    /// Returns a domain refusal for a blank identity field or reason.
    #[allow(clippy::too_many_arguments)]
    pub fn reject(
        id: ObservationId,
        asset_id: &str,
        source_name: &str,
        price: Decimal,
        currency: &str,
        observed_at: DateTime<Utc>,
        received_at: DateTime<Utc>,
        reason: &str,
    ) -> Result<Self, DomainError> {
        let (asset_id, source_name, currency) = validate_identity(asset_id, source_name, currency)?;
        Ok(Self {
            id,
            asset_id,
            source_name,
            price,
            currency,
            observed_at,
            received_at,
            status: ObservationStatus::Rejected,
            rejection_reason: Some(bounded_reason(reason)?),
        })
    }

    /// Evaluates one provider reading, returning an accepted observation or a
    /// rejected one. A data problem never becomes an error the run aborts on.
    ///
    /// # Errors
    /// Returns a domain refusal only when the identity fields are invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate(
        id: ObservationId,
        asset_id: &str,
        source_name: &str,
        price_raw: &str,
        currency: &str,
        observed_at: DateTime<Utc>,
        received_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let price = match parse_price(price_raw) {
            Ok(price) => price,
            Err(_) => {
                return Self::reject(
                    id,
                    asset_id,
                    source_name,
                    Decimal::ZERO,
                    currency,
                    observed_at,
                    received_at,
                    "malformed price",
                );
            }
        };
        if price <= Decimal::ZERO {
            return Self::reject(
                id,
                asset_id,
                source_name,
                price,
                currency,
                observed_at,
                received_at,
                "non-positive price",
            );
        }
        if is_clock_skewed(observed_at, received_at) {
            return Self::reject(
                id,
                asset_id,
                source_name,
                price,
                currency,
                observed_at,
                received_at,
                "clock skew beyond tolerance",
            );
        }
        Self::accept(
            id,
            asset_id,
            source_name,
            price,
            currency,
            observed_at,
            received_at,
        )
    }

    /// Rehydrates a stored observation.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the stored shape is inconsistent.
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: ObservationId,
        asset_id: String,
        source_name: String,
        price: Decimal,
        currency: String,
        observed_at: DateTime<Utc>,
        received_at: DateTime<Utc>,
        status: ObservationStatus,
        rejection_reason: Option<String>,
    ) -> Result<Self, DomainError> {
        let (asset_id, source_name, currency) =
            validate_identity(&asset_id, &source_name, &currency)?;
        if status == ObservationStatus::Accepted && price <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "price",
                reason: "an accepted observation must be positive".to_string(),
            });
        }
        if status == ObservationStatus::Rejected && rejection_reason.is_none() {
            return Err(DomainError::Invalid {
                field: "rejectionReason",
                reason: "a rejected observation must carry a reason".to_string(),
            });
        }
        Ok(Self {
            id,
            asset_id,
            source_name,
            price,
            currency,
            observed_at,
            received_at,
            status,
            rejection_reason,
        })
    }

    /// The observation identifier.
    #[must_use]
    pub fn id(&self) -> ObservationId {
        self.id
    }

    /// The canonical asset id.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The C4 source name.
    #[must_use]
    pub fn source_name(&self) -> &str {
        &self.source_name
    }

    /// The decimal price.
    #[must_use]
    pub fn price(&self) -> Decimal {
        self.price
    }

    /// The normalised quote currency.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The source's own timestamp.
    #[must_use]
    pub fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }

    /// When the service received the reading.
    #[must_use]
    pub fn received_at(&self) -> DateTime<Utc> {
        self.received_at
    }

    /// The stored state.
    #[must_use]
    pub fn status(&self) -> ObservationStatus {
        self.status
    }

    /// Why the reading was rejected.
    #[must_use]
    pub fn rejection_reason(&self) -> Option<&str> {
        self.rejection_reason.as_deref()
    }

    /// Whether the reading may be selected.
    #[must_use]
    pub fn is_accepted(&self) -> bool {
        self.status == ObservationStatus::Accepted
    }

    /// The observation's age in whole seconds at `now`.
    #[must_use]
    pub fn age_seconds(&self, now: DateTime<Utc>) -> i64 {
        (now - self.observed_at).num_seconds()
    }
}

fn validate_identity(
    asset_id: &str,
    source_name: &str,
    currency: &str,
) -> Result<(String, String, String), DomainError> {
    let asset_id = validate_asset_id(asset_id)?;
    let source_name = source_name.trim();
    if source_name.is_empty() {
        return Err(DomainError::Required {
            field: "sourceName",
        });
    }
    if source_name.chars().count() > MAX_SOURCE_NAME_CHARS {
        return Err(DomainError::TooLarge {
            field: "sourceName",
            limit: "80 characters",
        });
    }
    let currency = crate::fx::normalise_currency(currency)?;
    Ok((asset_id, source_name.to_string(), currency))
}

fn bounded_reason(reason: &str) -> Result<String, DomainError> {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Required {
            field: "rejectionReason",
        });
    }
    if looks_like_secret(trimmed) {
        return Ok("reading rejected".to_string());
    }
    Ok(trimmed.chars().take(MAX_REJECTION_REASON_CHARS).collect())
}

/// The served price for one asset, selected across sources (BR1.1–BR1.3,
/// BR2.1–BR2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceQuote {
    asset_id: String,
    price: Option<Decimal>,
    currency: Option<String>,
    source: Option<String>,
    observed_at: Option<DateTime<Utc>>,
    freshness: Freshness,
}

impl PriceQuote {
    /// Builds a priced quote from an accepted observation.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the observation is rejected or the
    /// freshness is unpriced.
    pub fn priced(
        asset_id: &str,
        observation: &PriceObservation,
        freshness: Freshness,
    ) -> Result<Self, DomainError> {
        let asset_id = validate_asset_id(asset_id)?;
        if !observation.is_accepted() {
            return Err(DomainError::Invalid {
                field: "observation",
                reason: "a rejected observation is never served".to_string(),
            });
        }
        if !freshness.is_priced() {
            return Err(DomainError::Invalid {
                field: "freshness",
                reason: "a priced quote cannot be unpriced".to_string(),
            });
        }
        Ok(Self {
            asset_id,
            price: Some(observation.price()),
            currency: Some(observation.currency().to_string()),
            source: Some(observation.source_name().to_string()),
            observed_at: Some(observation.observed_at()),
            freshness,
        })
    }

    /// Builds the unpriced quote for an asset no observation can be served for.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset id.
    pub fn unpriced(asset_id: &str) -> Result<Self, DomainError> {
        Ok(Self {
            asset_id: validate_asset_id(asset_id)?,
            price: None,
            currency: None,
            source: None,
            observed_at: None,
            freshness: Freshness::Unpriced,
        })
    }

    /// The canonical asset id.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The decimal price, absent when unpriced.
    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        self.price
    }

    /// The quote currency, absent when unpriced.
    #[must_use]
    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    /// The winning source, absent when unpriced.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// The observation time, absent when unpriced.
    #[must_use]
    pub fn observed_at(&self) -> Option<DateTime<Utc>> {
        self.observed_at
    }

    /// The freshness label.
    #[must_use]
    pub fn freshness(&self) -> Freshness {
        self.freshness
    }
}
