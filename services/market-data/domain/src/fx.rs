//! FX readings and served rates (BR3.1–BR3.3).
//!
//! A rate is decimal and greater than zero. A direct pair is served only while
//! it is fresh; a missing direct pair may be derived through USD from two fresh
//! legs. A cross rate is computed per request and never stored as a source
//! reading.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::DomainError;
use crate::ids::FxRateId;
use crate::price::is_clock_skewed;
use crate::secrets::sanitise_message;

/// The longest source name accepted.
pub const MAX_SOURCE_NAME_CHARS: usize = 80;

/// The four-letter token symbols the deployment quotes prices in.
pub const TOKEN_CURRENCIES: [&str; 4] = ["USDT", "USDC", "BUSD", "TUSD"];

/// Normalises and validates a currency code.
///
/// A code is three ASCII letters (ISO 4217), or one of [`TOKEN_CURRENCIES`] for
/// a stablecoin quote. Both are uppercased.
///
/// # Errors
/// Returns [`DomainError::Required`] for a blank code and
/// [`DomainError::Invalid`] for anything outside those shapes.
pub fn normalise_currency(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field: "currency" });
    }
    let upper = value.to_ascii_uppercase();
    let three_letters = upper.len() == 3
        && upper
            .chars()
            .all(|character| character.is_ascii_alphabetic());
    if three_letters || TOKEN_CURRENCIES.contains(&upper.as_str()) {
        return Ok(upper);
    }
    Err(DomainError::Invalid {
        field: "currency",
        reason: "must be three letters or a supported token symbol".to_string(),
    })
}

/// One reading of an exchange rate between two currencies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxRate {
    id: FxRateId,
    base: String,
    quote: String,
    rate: Decimal,
    source: String,
    observed_at: DateTime<Utc>,
    received_at: DateTime<Utc>,
}

impl FxRate {
    /// Records one FX reading.
    ///
    /// # Errors
    /// Returns a domain refusal for a malformed currency, an identical pair, a
    /// non-positive rate, a blank source, or a clock-skewed timestamp.
    pub fn record(
        id: FxRateId,
        base: &str,
        quote: &str,
        rate: Decimal,
        source: &str,
        observed_at: DateTime<Utc>,
        received_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let base = normalise_currency(base)?;
        let quote = normalise_currency(quote)?;
        if base == quote {
            return Err(DomainError::Invalid {
                field: "base",
                reason: "must differ from the quote currency".to_string(),
            });
        }
        if rate <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "rate",
                reason: "must be greater than zero".to_string(),
            });
        }
        let source = source.trim();
        if source.is_empty() {
            return Err(DomainError::Required { field: "source" });
        }
        if source.chars().count() > MAX_SOURCE_NAME_CHARS {
            return Err(DomainError::TooLarge {
                field: "source",
                limit: "80 characters",
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
            base,
            quote,
            rate,
            source: source.to_string(),
            observed_at,
            received_at,
        })
    }

    /// Rehydrates a stored reading.
    ///
    /// # Errors
    /// Returns the same refusals as [`Self::record`].
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: FxRateId,
        base: String,
        quote: String,
        rate: Decimal,
        source: String,
        observed_at: DateTime<Utc>,
        received_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        Self::record(id, &base, &quote, rate, &source, observed_at, received_at)
    }

    /// The reading identifier.
    #[must_use]
    pub fn id(&self) -> FxRateId {
        self.id
    }

    /// The base currency.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The quote currency.
    #[must_use]
    pub fn quote(&self) -> &str {
        &self.quote
    }

    /// One base unit in quote units.
    #[must_use]
    pub fn rate(&self) -> Decimal {
        self.rate
    }

    /// The C4 source name.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
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

    /// The reading's age in whole seconds at `now`.
    #[must_use]
    pub fn age_seconds(&self, now: DateTime<Utc>) -> i64 {
        (now - self.observed_at).num_seconds()
    }

    /// A non-secret summary of why the reading was refused.
    #[must_use]
    pub fn refusal_summary(reason: &str) -> String {
        sanitise_message(reason, "fx reading rejected")
    }

    /// The opposite direction of the same reading: `quote → base` at `1 / rate`.
    /// The source and both timestamps are preserved, so provenance is unchanged.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] if the stored rate is not positive.
    pub fn inverted(&self) -> Result<Self, DomainError> {
        Self::record(
            self.id,
            &self.quote,
            &self.base,
            Decimal::ONE / self.rate,
            &self.source,
            self.observed_at,
            self.received_at,
        )
    }
}

/// The FX rate C6 serves: a direct reading or a per-request cross rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServedFxRate {
    base: String,
    quote: String,
    rate: Decimal,
    observed_at: DateTime<Utc>,
    source: String,
    derived_from: Vec<String>,
}

impl ServedFxRate {
    /// Serves a stored direct reading.
    #[must_use]
    pub fn from_direct(rate: &FxRate) -> Self {
        Self {
            base: rate.base.clone(),
            quote: rate.quote.clone(),
            rate: rate.rate,
            observed_at: rate.observed_at,
            source: rate.source.clone(),
            derived_from: Vec::new(),
        }
    }

    /// Derives `base → quote` through USD from two fresh legs.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the legs do not join through USD.
    pub fn derived(
        base: &str,
        quote: &str,
        first: &FxRate,
        second: &FxRate,
    ) -> Result<Self, DomainError> {
        let base = normalise_currency(base)?;
        let quote = normalise_currency(quote)?;
        if first.base != base || first.quote != "USD" {
            return Err(DomainError::Invalid {
                field: "derivedFrom",
                reason: "the first leg must run from the base currency to USD".to_string(),
            });
        }
        if second.base != "USD" || second.quote != quote {
            return Err(DomainError::Invalid {
                field: "derivedFrom",
                reason: "the second leg must run from USD to the quote currency".to_string(),
            });
        }
        Ok(Self {
            base,
            quote,
            rate: first.rate * second.rate,
            observed_at: first.observed_at.min(second.observed_at),
            source: "derived".to_string(),
            derived_from: vec![first.source.clone(), second.source.clone()],
        })
    }

    /// The base currency.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The quote currency.
    #[must_use]
    pub fn quote(&self) -> &str {
        &self.quote
    }

    /// The decimal rate.
    #[must_use]
    pub fn rate(&self) -> Decimal {
        self.rate
    }

    /// The conservative observation time: the older of the two legs.
    #[must_use]
    pub fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }

    /// The source name, `derived` for a cross rate.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Both leg sources, empty for a direct reading.
    #[must_use]
    pub fn derived_from(&self) -> &[String] {
        &self.derived_from
    }
}
