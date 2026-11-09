//! Frozen snapshot inputs, their digest, and reproduction (BR4.1–BR4.2).
//!
//! The input set is stored as JSON with decimal values as plain strings, so the
//! canonical bytes — and therefore the digest — are stable. Reproduction parses
//! the same inputs and recomputes the same lines and total.

use std::str::FromStr;

use chrono::{DateTime, Utc};
use portfolio_domain::{
    compute_valuation, AccountId, FxRate, Holding, Quote, TargetAllocation, TargetAllocationId,
    TradeOverlay, Valuation, ValuationLine,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One frozen holding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenHolding {
    /// The account, absent for a portfolio-wide overlay.
    pub account_id: Option<String>,
    /// The asset.
    pub asset_id: String,
    /// The quantity as a decimal string.
    pub quantity: String,
    /// When the holding was observed.
    pub observed_at: DateTime<Utc>,
}

/// One frozen quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenQuote {
    /// The asset.
    pub asset_id: String,
    /// The price, absent when unpriced.
    pub price: Option<String>,
    /// The price currency.
    pub currency: Option<String>,
    /// The quote source.
    pub source: Option<String>,
    /// When the quote was observed.
    pub observed_at: Option<DateTime<Utc>>,
    /// The freshness label.
    pub freshness: String,
}

/// One frozen FX rate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenFx {
    /// The base currency.
    pub base: String,
    /// The quote currency.
    pub quote: String,
    /// The rate as a decimal string.
    pub rate: String,
    /// When the rate was observed.
    pub observed_at: DateTime<Utc>,
    /// The rate source.
    pub source: String,
    /// Both leg sources when derived.
    #[serde(default)]
    pub derived_from: Vec<String>,
}

/// One frozen target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenTarget {
    /// The asset.
    pub asset_id: String,
    /// The target percentage as a decimal string.
    pub target_percent: String,
}

/// The complete frozen input set.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenInputs {
    /// The base currency.
    pub base_currency: String,
    /// The holdings used.
    pub holdings: Vec<FrozenHolding>,
    /// The quotes used.
    pub quotes: Vec<FrozenQuote>,
    /// The FX rates used.
    pub fx: Vec<FrozenFx>,
    /// The targets used.
    pub targets: Vec<FrozenTarget>,
}

/// The stored form of one computed line.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredLine {
    /// The asset.
    pub asset_id: String,
    /// The quantity.
    pub quantity: String,
    /// The value, absent when unpriced.
    pub value: Option<String>,
    /// The allocation percentage.
    pub allocation: Option<f64>,
    /// The target percentage.
    pub target_percent: Option<f64>,
    /// The drift in percentage points.
    pub drift_points: Option<f64>,
    /// The unit price.
    pub price: Option<String>,
    /// The quote source.
    pub price_source: Option<String>,
    /// The quote freshness.
    pub price_freshness: String,
    /// When the quote was observed.
    pub price_observed_at: Option<DateTime<Utc>>,
    /// The effective FX rate.
    pub fx_rate: Option<String>,
    /// The FX source.
    pub fx_source: Option<String>,
    /// Both FX leg sources.
    pub fx_derived_from: Vec<String>,
    /// When the FX rate was observed.
    pub fx_observed_at: Option<DateTime<Utc>>,
    /// The data status.
    pub data_status: String,
}

/// The stored form of a valuation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredValuation {
    /// The status label.
    pub status: String,
    /// The priced total.
    pub total_value: Option<String>,
    /// The base currency.
    pub currency: String,
    /// The priced coverage.
    pub priced_coverage: String,
    /// The unpriced count.
    pub unpriced_count: u32,
    /// The excluded count.
    pub excluded_count: u32,
    /// The lines.
    pub lines: Vec<StoredLine>,
}

impl FrozenInputs {
    /// Serialises the input set canonically.
    #[must_use]
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }

    /// The SHA-256 digest over the canonical input bytes.
    #[must_use]
    pub fn digest(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.to_json().as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Parses a stored input set.
    ///
    /// # Errors
    /// Returns a message when the JSON is malformed.
    pub fn from_json(raw: &str) -> Result<Self, String> {
        serde_json::from_str(raw).map_err(|error| format!("snapshot inputs are malformed: {error}"))
    }

    /// Rebuilds the domain inputs and recomputes the valuation exactly.
    ///
    /// # Errors
    /// Returns a message when a stored decimal or label is malformed.
    pub fn reproduce(&self) -> Result<Valuation, String> {
        let holdings = self
            .holdings
            .iter()
            .map(|holding| {
                let account_id = holding
                    .account_id
                    .as_deref()
                    .and_then(|raw| AccountId::parse(raw).ok());
                Holding::new(
                    account_id,
                    &holding.asset_id,
                    parse(&holding.quantity, "quantity")?,
                    holding.observed_at,
                )
                .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let quotes = self
            .quotes
            .iter()
            .map(|quote| {
                let price = quote
                    .price
                    .as_deref()
                    .map(|raw| parse(raw, "price"))
                    .transpose()?;
                Quote::new(
                    &quote.asset_id,
                    price,
                    quote.currency.clone(),
                    quote.source.clone(),
                    quote.observed_at,
                    portfolio_domain::Freshness::parse(&quote.freshness)
                        .map_err(|error| error.to_string())?,
                )
                .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let fx = self
            .fx
            .iter()
            .map(|rate| {
                FxRate::new(
                    &rate.base,
                    &rate.quote,
                    parse(&rate.rate, "fxRate")?,
                    rate.observed_at,
                    &rate.source,
                    rate.derived_from.clone(),
                )
                .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        let targets = self
            .targets
            .iter()
            .map(|target| {
                TargetAllocation::new(
                    TargetAllocationId::generate(),
                    portfolio_domain::PortfolioId::generate(),
                    &target.asset_id,
                    parse(&target.target_percent, "targetPercent")?,
                    DateTime::from_timestamp(0, 0).unwrap_or_else(Utc::now),
                )
                .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(compute_valuation(
            &self.base_currency,
            &holdings,
            &quotes,
            &fx,
            &targets,
            true,
        ))
    }
}

fn parse(raw: &str, field: &str) -> Result<Decimal, String> {
    Decimal::from_str(raw.trim()).map_err(|_| format!("stored {field} is not a decimal"))
}

/// Applies trade overlays that were recorded after a snapshot to the snapshot's
/// frozen holdings, so a read served from that snapshot still reflects a later
/// recorded trade (AC5.4.1) without calling C5 or C6.
#[must_use]
pub fn apply_overlay_deltas(inputs: &FrozenInputs, overlays: &[&TradeOverlay]) -> FrozenInputs {
    let mut frozen = inputs.clone();
    for overlay in overlays {
        let asset = overlay.asset_id();
        if let Some(holding) = frozen
            .holdings
            .iter_mut()
            .find(|holding| holding.asset_id == asset)
        {
            let current = Decimal::from_str(&holding.quantity).unwrap_or_default();
            holding.quantity = (current + overlay.quantity_delta())
                .max(Decimal::ZERO)
                .to_string();
        } else {
            frozen.holdings.push(FrozenHolding {
                account_id: overlay.account_id().map(|id| id.to_string()),
                asset_id: asset.to_string(),
                quantity: overlay.quantity_delta().max(Decimal::ZERO).to_string(),
                observed_at: overlay.applied_at(),
            });
        }
    }
    frozen
        .holdings
        .sort_by(|left, right| left.asset_id.cmp(&right.asset_id));
    frozen
}

/// Freezes the exact input set used by a valuation into its stored form, with a
/// deterministic order so the digest is stable.
#[must_use]
pub fn freeze_inputs(
    base_currency: &str,
    holdings: &[Holding],
    quotes: &[Quote],
    fx: &[FxRate],
    targets: &[TargetAllocation],
) -> FrozenInputs {
    let mut frozen_holdings: Vec<FrozenHolding> = holdings
        .iter()
        .map(|holding| FrozenHolding {
            account_id: holding.account_id().map(|id| id.to_string()),
            asset_id: holding.asset_id().to_string(),
            quantity: holding.quantity().to_string(),
            observed_at: holding.observed_at(),
        })
        .collect();
    frozen_holdings.sort_by(|left, right| left.asset_id.cmp(&right.asset_id));

    let mut frozen_quotes: Vec<FrozenQuote> = quotes
        .iter()
        .map(|quote| FrozenQuote {
            asset_id: quote.asset_id().to_string(),
            price: quote.price().map(|price| price.to_string()),
            currency: quote.currency().map(ToString::to_string),
            source: quote.source().map(ToString::to_string),
            observed_at: quote.observed_at(),
            freshness: quote.freshness().as_str().to_string(),
        })
        .collect();
    frozen_quotes.sort_by(|left, right| left.asset_id.cmp(&right.asset_id));

    let mut frozen_fx: Vec<FrozenFx> = fx
        .iter()
        .map(|rate| FrozenFx {
            base: rate.base().to_string(),
            quote: rate.quote().to_string(),
            rate: rate.rate().to_string(),
            observed_at: rate.observed_at(),
            source: rate.source().to_string(),
            derived_from: rate.derived_from().to_vec(),
        })
        .collect();
    frozen_fx.sort_by(|left, right| (&left.base, &left.quote).cmp(&(&right.base, &right.quote)));

    let mut frozen_targets: Vec<FrozenTarget> = targets
        .iter()
        .map(|target| FrozenTarget {
            asset_id: target.asset_id().to_string(),
            target_percent: target.target_percent().to_string(),
        })
        .collect();
    frozen_targets.sort_by(|left, right| left.asset_id.cmp(&right.asset_id));

    FrozenInputs {
        base_currency: base_currency.trim().to_ascii_uppercase(),
        holdings: frozen_holdings,
        quotes: frozen_quotes,
        fx: frozen_fx,
        targets: frozen_targets,
    }
}

/// Projects a domain valuation into its stored form.
#[must_use]
pub fn stored_valuation(valuation: &Valuation) -> StoredValuation {
    StoredValuation {
        status: valuation.status().as_str().to_string(),
        total_value: valuation.total_value().map(|value| value.to_string()),
        currency: valuation.currency().to_string(),
        priced_coverage: valuation.priced_coverage().to_string(),
        unpriced_count: valuation.unpriced_count(),
        excluded_count: valuation.excluded_count(),
        lines: valuation.lines().iter().map(stored_line).collect(),
    }
}

fn stored_line(line: &ValuationLine) -> StoredLine {
    StoredLine {
        asset_id: line.asset_id().to_string(),
        quantity: line.quantity().to_string(),
        value: line.value().map(|value| value.to_string()),
        allocation: line.allocation().and_then(decimal_to_f64),
        target_percent: line.target_percent().and_then(decimal_to_f64),
        drift_points: line.drift_points().and_then(decimal_to_f64),
        price: line.price().map(|price| price.to_string()),
        price_source: line.price_source().map(ToString::to_string),
        price_freshness: line.price_freshness().as_str().to_string(),
        price_observed_at: line.price_observed_at(),
        fx_rate: line.fx_rate().map(|rate| rate.to_string()),
        fx_source: line.fx_source().map(ToString::to_string),
        fx_derived_from: line.fx_derived_from().to_vec(),
        fx_observed_at: line.fx_observed_at(),
        data_status: line.data_status().as_str().to_string(),
    }
}

fn decimal_to_f64(value: Decimal) -> Option<f64> {
    use rust_decimal::prelude::ToPrimitive;
    value.to_f64()
}

/// Serialises a valuation into its stored JSON.
#[must_use]
pub fn lines_json(valuation: &Valuation) -> String {
    serde_json::to_string(&stored_valuation(valuation)).unwrap_or_else(|_| "{}".to_string())
}

/// Parses stored lines.
///
/// # Errors
/// Returns a message when the JSON is malformed.
pub fn parse_lines(raw: &str) -> Result<StoredValuation, String> {
    serde_json::from_str(raw).map_err(|error| format!("snapshot lines are malformed: {error}"))
}

/// Verifies a snapshot: the digest matches its frozen inputs and reproduction
/// yields the stored lines and total (BR4.2).
///
/// # Errors
/// Returns the reason when the digest or the reproduced valuation differs.
pub fn verify_snapshot(
    inputs_json: &str,
    digest: &str,
    lines_json: &str,
) -> Result<Valuation, String> {
    let inputs = FrozenInputs::from_json(inputs_json)?;
    if inputs.digest() != digest {
        return Err("the stored digest does not match the frozen inputs".to_string());
    }
    let valuation = inputs.reproduce()?;
    let stored = parse_lines(lines_json)?;
    let reproduced = stored_valuation(&valuation);
    if stored != reproduced {
        return Err("the stored lines do not reproduce from the frozen inputs".to_string());
    }
    Ok(valuation)
}
