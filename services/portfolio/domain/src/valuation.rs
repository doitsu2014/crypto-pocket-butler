//! Backend valuation (BR2.1–BR2.3, BR3.1): quantity × price, converted to the
//! base currency, with per-line provenance, unpriced lines kept and counted, and
//! drift in percentage points over the priced total.
//!
//! All arithmetic uses [`rust_decimal::Decimal`]; binary floating point is never
//! used.

use std::collections::BTreeMap;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};

use crate::error::DomainError;
use crate::ids::AccountId;
use crate::overlay::TradeOverlay;
use crate::target::TargetAllocation;

/// The most decimal places accepted for a quantity, price, or rate.
pub const MAX_DECIMAL_SCALE: u32 = 18;

/// Parses a plain decimal string.
///
/// # Errors
/// Returns [`DomainError::Required`] for an empty value and
/// [`DomainError::Invalid`] for scientific notation, a non-number, or more than
/// [`MAX_DECIMAL_SCALE`] decimal places.
pub fn parse_decimal(field: &'static str, raw: &str) -> Result<Decimal, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required { field });
    }
    if value.contains('e') || value.contains('E') {
        return Err(DomainError::Invalid {
            field,
            reason: "must be a plain decimal string".to_string(),
        });
    }
    let decimal = Decimal::from_str(value).map_err(|_| DomainError::Invalid {
        field,
        reason: "must be a decimal number".to_string(),
    })?;
    if decimal.scale() > MAX_DECIMAL_SCALE {
        return Err(DomainError::Invalid {
            field,
            reason: "has more than 18 decimal places".to_string(),
        });
    }
    Ok(decimal)
}

fn round_one(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(1, RoundingStrategy::MidpointAwayFromZero)
}

fn round_four(value: Decimal) -> Decimal {
    value.round_dp_with_strategy(4, RoundingStrategy::MidpointAwayFromZero)
}

/// A quote's freshness (C6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// A live quote.
    Live,
    /// A delayed quote.
    Delayed,
    /// A stale quote.
    Stale,
    /// No price at all.
    Unpriced,
}

impl Freshness {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Delayed => "delayed",
            Self::Stale => "stale",
            Self::Unpriced => "unpriced",
        }
    }

    /// Parses a C6 freshness label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown label.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "live" => Ok(Self::Live),
            "delayed" => Ok(Self::Delayed),
            "stale" => Ok(Self::Stale),
            "unpriced" => Ok(Self::Unpriced),
            _ => Err(DomainError::Invalid {
                field: "priceFreshness",
                reason: "is not a known freshness label".to_string(),
            }),
        }
    }
}

/// One C6 price with its provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Quote {
    asset_id: String,
    price: Option<Decimal>,
    currency: Option<String>,
    source: Option<String>,
    observed_at: Option<DateTime<Utc>>,
    freshness: Freshness,
}

impl Quote {
    /// Creates a quote. A `price` is present only when the freshness is not
    /// [`Freshness::Unpriced`].
    ///
    /// # Errors
    /// Returns a domain refusal for a blank asset or a negative price.
    pub fn new(
        asset_id: &str,
        price: Option<Decimal>,
        currency: Option<String>,
        source: Option<String>,
        observed_at: Option<DateTime<Utc>>,
        freshness: Freshness,
    ) -> Result<Self, DomainError> {
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        if price.is_some_and(|price| price.is_sign_negative()) {
            return Err(DomainError::Invalid {
                field: "price",
                reason: "must not be negative".to_string(),
            });
        }
        Ok(Self {
            asset_id: asset_id.to_string(),
            price,
            currency: currency.map(|currency| currency.trim().to_ascii_uppercase()),
            source,
            observed_at,
            freshness,
        })
    }

    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The unit price, absent when unpriced.
    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        self.price
    }

    /// The currency the price is quoted in.
    #[must_use]
    pub fn currency(&self) -> Option<&str> {
        self.currency.as_deref()
    }

    /// The quote source.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// When the quote was observed.
    #[must_use]
    pub fn observed_at(&self) -> Option<DateTime<Utc>> {
        self.observed_at
    }

    /// The freshness label.
    #[must_use]
    pub fn freshness(&self) -> Freshness {
        self.freshness
    }

    fn usable_price(&self) -> Option<Decimal> {
        match self.freshness {
            Freshness::Unpriced => None,
            _ => self.price,
        }
    }
}

/// One C6 FX rate with its provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FxRate {
    base: String,
    quote: String,
    rate: Decimal,
    observed_at: DateTime<Utc>,
    source: String,
    derived_from: Vec<String>,
}

impl FxRate {
    /// Creates a rate. The rate must be greater than zero.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for a non-positive rate or a blank pair.
    pub fn new(
        base: &str,
        quote: &str,
        rate: Decimal,
        observed_at: DateTime<Utc>,
        source: &str,
        derived_from: Vec<String>,
    ) -> Result<Self, DomainError> {
        let base = base.trim().to_ascii_uppercase();
        let quote = quote.trim().to_ascii_uppercase();
        if base.is_empty() || quote.is_empty() {
            return Err(DomainError::Required { field: "fx" });
        }
        if rate <= Decimal::ZERO {
            return Err(DomainError::Invalid {
                field: "fxRate",
                reason: "must be greater than zero".to_string(),
            });
        }
        Ok(Self {
            base,
            quote,
            rate,
            observed_at,
            source: source.trim().to_string(),
            derived_from,
        })
    }

    /// The base currency of the pair.
    #[must_use]
    pub fn base(&self) -> &str {
        &self.base
    }

    /// The quote currency of the pair.
    #[must_use]
    pub fn quote(&self) -> &str {
        &self.quote
    }

    /// The rate.
    #[must_use]
    pub fn rate(&self) -> Decimal {
        self.rate
    }

    /// When the rate was observed.
    #[must_use]
    pub fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }

    /// The rate source.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Both leg sources when the pair was derived through a third currency.
    #[must_use]
    pub fn derived_from(&self) -> &[String] {
        &self.derived_from
    }
}

/// One holding read from C5. The account is absent only for an overlay that
/// applies to the whole portfolio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    account_id: Option<AccountId>,
    asset_id: String,
    quantity: Decimal,
    observed_at: DateTime<Utc>,
}

impl Holding {
    /// Creates a holding.
    ///
    /// # Errors
    /// Returns [`DomainError::Required`] for a blank asset and
    /// [`DomainError::Invalid`] for a negative quantity.
    pub fn new(
        account_id: Option<AccountId>,
        asset_id: &str,
        quantity: Decimal,
        observed_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        let asset_id = asset_id.trim();
        if asset_id.is_empty() {
            return Err(DomainError::Required { field: "assetId" });
        }
        if quantity.is_sign_negative() {
            return Err(DomainError::Invalid {
                field: "quantity",
                reason: "must not be negative".to_string(),
            });
        }
        Ok(Self {
            account_id,
            asset_id: asset_id.to_string(),
            quantity,
            observed_at,
        })
    }

    /// The account, absent for a portfolio-wide overlay.
    #[must_use]
    pub fn account_id(&self) -> Option<AccountId> {
        self.account_id
    }

    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// When the holding was observed.
    #[must_use]
    pub fn observed_at(&self) -> DateTime<Utc> {
        self.observed_at
    }
}

/// The data status of one valuation line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataStatus {
    /// The line is priced.
    Priced,
    /// The line has no usable price and is excluded from the total.
    Unpriced,
    /// C5 holdings were unavailable; no line was fabricated.
    HoldingsUnavailable,
}

impl DataStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Priced => "priced",
            Self::Unpriced => "unpriced",
            Self::HoldingsUnavailable => "holdings_unavailable",
        }
    }
}

/// The overall status of a valuation read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValuationStatus {
    /// Every line is priced.
    Complete,
    /// Some or all data is missing; the lines carry their own labels.
    Partial,
    /// Nothing could be valued.
    Unavailable,
}

impl ValuationStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One asset's contribution to the valuation, with its provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationLine {
    asset_id: String,
    quantity: Decimal,
    value: Option<Decimal>,
    allocation: Option<Decimal>,
    target_percent: Option<Decimal>,
    drift_points: Option<Decimal>,
    price: Option<Decimal>,
    price_source: Option<String>,
    price_freshness: Freshness,
    price_observed_at: Option<DateTime<Utc>>,
    fx_rate: Option<Decimal>,
    fx_source: Option<String>,
    fx_derived_from: Vec<String>,
    fx_observed_at: Option<DateTime<Utc>>,
    data_status: DataStatus,
}

impl ValuationLine {
    /// The asset.
    #[must_use]
    pub fn asset_id(&self) -> &str {
        &self.asset_id
    }

    /// The aggregated quantity.
    #[must_use]
    pub fn quantity(&self) -> Decimal {
        self.quantity
    }

    /// The value in the base currency, absent when unpriced.
    #[must_use]
    pub fn value(&self) -> Option<Decimal> {
        self.value
    }

    /// The allocation as a percentage of the priced total.
    #[must_use]
    pub fn allocation(&self) -> Option<Decimal> {
        self.allocation
    }

    /// The target percentage, absent when no target exists.
    #[must_use]
    pub fn target_percent(&self) -> Option<Decimal> {
        self.target_percent
    }

    /// The drift in percentage points: actual minus target.
    #[must_use]
    pub fn drift_points(&self) -> Option<Decimal> {
        self.drift_points
    }

    /// The unit price.
    #[must_use]
    pub fn price(&self) -> Option<Decimal> {
        self.price
    }

    /// The quote source.
    #[must_use]
    pub fn price_source(&self) -> Option<&str> {
        self.price_source.as_deref()
    }

    /// The quote freshness.
    #[must_use]
    pub fn price_freshness(&self) -> Freshness {
        self.price_freshness
    }

    /// When the quote was observed.
    #[must_use]
    pub fn price_observed_at(&self) -> Option<DateTime<Utc>> {
        self.price_observed_at
    }

    /// The effective FX rate used.
    #[must_use]
    pub fn fx_rate(&self) -> Option<Decimal> {
        self.fx_rate
    }

    /// The FX source.
    #[must_use]
    pub fn fx_source(&self) -> Option<&str> {
        self.fx_source.as_deref()
    }

    /// Both leg sources when the FX pair was derived.
    #[must_use]
    pub fn fx_derived_from(&self) -> &[String] {
        &self.fx_derived_from
    }

    /// When the FX rate was observed.
    #[must_use]
    pub fn fx_observed_at(&self) -> Option<DateTime<Utc>> {
        self.fx_observed_at
    }

    /// The line's data status.
    #[must_use]
    pub fn data_status(&self) -> DataStatus {
        self.data_status
    }
}

/// A computed valuation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Valuation {
    status: ValuationStatus,
    holdings_unavailable: bool,
    total_value: Option<Decimal>,
    currency: String,
    priced_coverage: Decimal,
    unpriced_count: u32,
    excluded_count: u32,
    lines: Vec<ValuationLine>,
}

impl Valuation {
    /// The overall status.
    #[must_use]
    pub fn status(&self) -> ValuationStatus {
        self.status
    }

    /// Whether C5 holdings were unavailable.
    #[must_use]
    pub fn holdings_unavailable(&self) -> bool {
        self.holdings_unavailable
    }

    /// The priced total, absent when nothing could be valued.
    #[must_use]
    pub fn total_value(&self) -> Option<Decimal> {
        self.total_value
    }

    /// The base currency.
    #[must_use]
    pub fn currency(&self) -> &str {
        &self.currency
    }

    /// The fraction of asset lines that are priced (0–1).
    #[must_use]
    pub fn priced_coverage(&self) -> Decimal {
        self.priced_coverage
    }

    /// How many assets were excluded for having no usable price.
    #[must_use]
    pub fn unpriced_count(&self) -> u32 {
        self.unpriced_count
    }

    /// How many holdings are kept as excluded labelled lines.
    #[must_use]
    pub fn excluded_count(&self) -> u32 {
        self.excluded_count
    }

    /// The per-asset lines, sorted by asset id.
    #[must_use]
    pub fn lines(&self) -> &[ValuationLine] {
        &self.lines
    }

    /// The line for one asset, when present.
    #[must_use]
    pub fn line(&self, asset_id: &str) -> Option<&ValuationLine> {
        self.lines.iter().find(|line| line.asset_id == asset_id)
    }

    fn unavailable(currency: &str) -> Self {
        Self {
            status: ValuationStatus::Partial,
            holdings_unavailable: true,
            total_value: None,
            currency: currency.to_string(),
            priced_coverage: Decimal::ZERO,
            unpriced_count: 0,
            excluded_count: 0,
            lines: Vec::new(),
        }
    }
}

/// Applies each recorded trade overlay to the current holdings so allocation and
/// drift reflect a recorded trade immediately (BR6.1). A sell never drives a
/// holding below zero.
#[must_use]
pub fn apply_overlays(
    holdings: &[Holding],
    overlays: &[TradeOverlay],
    at: DateTime<Utc>,
) -> Vec<Holding> {
    let mut quantities: BTreeMap<String, (Decimal, Option<AccountId>, DateTime<Utc>)> =
        BTreeMap::new();
    for holding in holdings {
        let entry = quantities.entry(holding.asset_id.clone()).or_insert((
            Decimal::ZERO,
            holding.account_id,
            holding.observed_at,
        ));
        entry.0 += holding.quantity;
    }
    for overlay in overlays {
        let entry = quantities.entry(overlay.asset_id().to_string()).or_insert((
            Decimal::ZERO,
            overlay.account_id(),
            at,
        ));
        entry.0 = (entry.0 + overlay.quantity_delta()).max(Decimal::ZERO);
        entry.2 = overlay.applied_at();
    }
    quantities
        .into_iter()
        .filter_map(|(asset_id, (quantity, account_id, observed_at))| {
            Holding::new(account_id, &asset_id, quantity, observed_at).ok()
        })
        .collect()
}

/// Computes a valuation (BR2.1–BR2.3, BR3.1). An unpriced line is excluded from
/// the priced total, kept with its quantity, and counted.
#[must_use]
pub fn compute_valuation(
    base_currency: &str,
    holdings: &[Holding],
    quotes: &[Quote],
    fx_rates: &[FxRate],
    targets: &[TargetAllocation],
    holdings_available: bool,
) -> Valuation {
    let currency = base_currency.trim().to_ascii_uppercase();
    if !holdings_available {
        return Valuation::unavailable(&currency);
    }

    let mut quantities: BTreeMap<String, Decimal> = BTreeMap::new();
    for holding in holdings {
        *quantities.entry(holding.asset_id.clone()).or_default() += holding.quantity;
    }

    let quote_by_asset: BTreeMap<&str, &Quote> = quotes
        .iter()
        .map(|quote| (quote.asset_id.as_str(), quote))
        .collect();
    let target_by_asset: BTreeMap<&str, Decimal> = targets
        .iter()
        .map(|target| (target.asset_id(), target.target_percent()))
        .collect();

    struct Draft {
        asset_id: String,
        quantity: Decimal,
        value: Option<Decimal>,
        quote: Option<Quote>,
        fx_rate: Option<Decimal>,
        fx_source: Option<String>,
        fx_derived_from: Vec<String>,
        fx_observed_at: Option<DateTime<Utc>>,
        target: Option<Decimal>,
    }

    let mut drafts: Vec<Draft> = Vec::with_capacity(quantities.len());
    let mut priced_total = Decimal::ZERO;
    for (asset_id, quantity) in quantities {
        let quote = quote_by_asset
            .get(asset_id.as_str())
            .map(|quote| (*quote).clone());
        let mut draft = Draft {
            asset_id: asset_id.clone(),
            quantity,
            value: None,
            quote: quote.clone(),
            fx_rate: None,
            fx_source: None,
            fx_derived_from: Vec::new(),
            fx_observed_at: None,
            target: target_by_asset.get(asset_id.as_str()).copied(),
        };
        if let Some(quote) = &quote {
            if let Some(price) = quote.usable_price() {
                // The quote currency comes from C6's Price. A missing currency is
                // never assumed to be the base: without a currency there is no
                // provable conversion, so the line stays unpriced.
                if let Some(quoted_in) = quote.currency.as_deref() {
                    if quoted_in == currency {
                        draft.value = Some(quantity * price);
                    } else if let Some((rate, inverted)) = find_rate(&currency, quoted_in, fx_rates)
                    {
                        let effective = if inverted {
                            Decimal::ONE / rate.rate
                        } else {
                            rate.rate
                        };
                        draft.value = Some(quantity * price * effective);
                        draft.fx_rate = Some(effective);
                        draft.fx_source = Some(rate.source.clone());
                        draft.fx_derived_from = rate.derived_from.clone();
                        draft.fx_observed_at = Some(rate.observed_at);
                    }
                }
            }
        }
        if let Some(value) = draft.value {
            priced_total += value;
        }
        drafts.push(draft);
    }

    let line_count = drafts.len() as u32;
    let priced_count = drafts.iter().filter(|draft| draft.value.is_some()).count() as u32;
    let unpriced_count = line_count - priced_count;

    let lines: Vec<ValuationLine> = drafts
        .into_iter()
        .map(|draft| {
            let allocation = draft.value.and_then(|value| {
                if priced_total > Decimal::ZERO {
                    Some(round_one(value / priced_total * Decimal::ONE_HUNDRED))
                } else {
                    None
                }
            });
            let drift_points = match (allocation, draft.target) {
                (Some(allocation), Some(target)) => Some(round_one(allocation - target)),
                _ => None,
            };
            let data_status = if draft.value.is_some() {
                DataStatus::Priced
            } else {
                DataStatus::Unpriced
            };
            let price_freshness = draft
                .quote
                .as_ref()
                .map_or(Freshness::Unpriced, Quote::freshness);
            ValuationLine {
                asset_id: draft.asset_id,
                quantity: draft.quantity,
                value: draft.value,
                allocation,
                target_percent: draft.target,
                drift_points,
                price: draft.quote.as_ref().and_then(Quote::price),
                price_source: draft.quote.as_ref().and_then(|quote| quote.source.clone()),
                price_freshness,
                price_observed_at: draft.quote.as_ref().and_then(Quote::observed_at),
                fx_rate: draft.fx_rate,
                fx_source: draft.fx_source,
                fx_derived_from: draft.fx_derived_from,
                fx_observed_at: draft.fx_observed_at,
                data_status,
            }
        })
        .collect();

    let priced_coverage = if line_count == 0 {
        Decimal::ZERO
    } else {
        round_four(Decimal::from(priced_count) / Decimal::from(line_count))
    };
    Valuation {
        status: if unpriced_count > 0 {
            ValuationStatus::Partial
        } else {
            ValuationStatus::Complete
        },
        holdings_unavailable: false,
        total_value: Some(priced_total),
        currency,
        priced_coverage,
        unpriced_count,
        excluded_count: unpriced_count,
        lines,
    }
}

fn find_rate<'a>(base: &str, quote: &str, fx_rates: &'a [FxRate]) -> Option<(&'a FxRate, bool)> {
    if let Some(rate) = fx_rates
        .iter()
        .find(|rate| rate.base == quote && rate.quote == base)
    {
        return Some((rate, false));
    }
    fx_rates
        .iter()
        .find(|rate| rate.base == base && rate.quote == quote)
        .map(|rate| (rate, true))
}
