//! Currency conversion at compute time (BR2.2, NFR2.1).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

/// A table of rates into the user's base currency, with the FX observation
/// time. A conversion is always an estimate and is labelled as one.
#[derive(Debug, Clone, PartialEq)]
pub struct FxRates {
    base: String,
    rates: BTreeMap<String, Decimal>,
    as_of: Option<DateTime<Utc>>,
    permissive: bool,
}

impl FxRates {
    /// Creates a table from observed rates. The estimate flag is true whenever
    /// a non-base currency is present.
    #[must_use]
    pub fn new(
        base: impl Into<String>,
        rates: BTreeMap<String, Decimal>,
        as_of: Option<DateTime<Utc>>,
    ) -> Self {
        let base = normalise_currency(&base.into());
        let estimated = rates.keys().any(|currency| currency != &base);
        Self {
            base,
            rates,
            as_of,
            permissive: !estimated,
        }
    }

    /// A base-only table: every base amount converts at one, and any unknown
    /// currency is refused.
    #[must_use]
    pub fn base_only(base: impl Into<String>) -> Self {
        Self {
            base: normalise_currency(&base.into()),
            rates: BTreeMap::new(),
            as_of: None,
            permissive: false,
        }
    }

    /// A permissive table used where only quantities matter (the holdings guard
    /// and the lot rebuild), so no FX is required.
    #[must_use]
    pub fn unconverted() -> Self {
        Self {
            base: "USD".to_string(),
            rates: BTreeMap::new(),
            as_of: None,
            permissive: true,
        }
    }

    /// The base currency.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The FX observation time, when a conversion was needed.
    #[must_use]
    pub fn as_of(&self) -> Option<DateTime<Utc>> {
        self.as_of
    }

    /// Whether any non-base amount was converted.
    #[must_use]
    pub fn is_estimated(&self) -> bool {
        !self.permissive && self.rates.keys().any(|currency| currency != &self.base)
    }

    /// The rate from `currency` into the base, when known.
    #[must_use]
    pub fn rate(&self, currency: &str) -> Option<Decimal> {
        let currency = normalise_currency(currency);
        if currency == self.base || self.permissive {
            return Some(Decimal::ONE);
        }
        self.rates.get(&currency).copied()
    }

    /// Converts an amount in `currency` into the base.
    #[must_use]
    pub fn convert(&self, amount: Decimal, currency: &str) -> Option<Decimal> {
        self.rate(currency).map(|rate| amount * rate)
    }
}

/// Uppercases and trims a currency code.
#[must_use]
pub fn normalise_currency(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}
