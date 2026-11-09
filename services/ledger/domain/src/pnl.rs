//! P&L with coverage, market marks, and base-currency conversion
//! (BR2.2, BR3.1, BR3.2).

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::engine::ReplayOutcome;
use crate::error::DomainError;
use crate::fx::FxRates;

/// One asset the user currently holds, from Account's C5 `getHoldings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldAsset {
    /// The asset.
    pub asset_id: String,
    /// The quantity held across every account.
    pub quantity: Decimal,
}

/// How fresh a C6 price is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceFreshness {
    /// A live feed.
    Live,
    /// A delayed feed.
    Delayed,
    /// A stale feed.
    Stale,
    /// No price at all.
    Unpriced,
}

impl PriceFreshness {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Delayed => "delayed",
            Self::Stale => "stale",
            Self::Unpriced => "unpriced",
        }
    }

    /// Parses the wire spelling; an unknown value is treated as unpriced.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw.trim() {
            "live" => Self::Live,
            "delayed" => Self::Delayed,
            "stale" => Self::Stale,
            _ => Self::Unpriced,
        }
    }
}

/// One C6 price observation.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceMark {
    /// The unit price; absent when unpriced.
    pub price: Option<Decimal>,
    /// The currency the price is quoted in.
    pub currency: Option<String>,
    /// When the price was observed.
    pub as_of: Option<DateTime<Utc>>,
    /// The freshness label.
    pub freshness: PriceFreshness,
}

impl PriceMark {
    /// An unpriced observation.
    #[must_use]
    pub fn unpriced() -> Self {
        Self {
            price: None,
            currency: None,
            as_of: None,
            freshness: PriceFreshness::Unpriced,
        }
    }
}

/// The coverage label that always accompanies a P&L answer (BR3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoverageLabel {
    /// Every held asset by value has acquisition history and a price.
    Complete,
    /// Some holdings lack history or a price; the figures exclude them.
    Partial,
    /// No holding has history; no figures are returned.
    Unavailable,
}

impl CoverageLabel {
    /// The wire spelling.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

/// Per-asset figures, computed transiently for the C7 response (BR2.2).
#[derive(Debug, Clone, PartialEq)]
pub struct AssetPnl {
    /// The asset.
    pub asset_id: String,
    /// Cost basis of the open lots, in the base currency.
    pub net_invested: Option<Decimal>,
    /// Realised P&L from covered sells, in the base currency.
    pub realized: Option<Decimal>,
    /// Current value less remaining cost, in the base currency.
    pub unrealized: Option<Decimal>,
    /// Realised plus unrealised.
    pub total_return: Option<Decimal>,
    /// The C6 mark in the base currency; absent when unpriced.
    pub mark_price: Option<Decimal>,
    /// When the mark was observed.
    pub price_as_of: Option<DateTime<Utc>>,
    /// The mark's freshness label.
    pub price_freshness: String,
    /// Whether the asset's history and price are complete.
    pub covered: bool,
}

/// The P&L answer, total and per asset, always carrying its coverage and
/// currency.
#[derive(Debug, Clone, PartialEq)]
pub struct PnlReport {
    /// When the figures were computed.
    pub as_of: DateTime<Utc>,
    /// The user's base currency; every figure is in it.
    pub currency: String,
    /// When the FX rate was observed.
    pub fx_as_of: Option<DateTime<Utc>>,
    /// Whether any non-base amount was converted.
    pub fx_estimated: bool,
    /// Cost basis of open lots; absent when unavailable.
    pub net_invested: Option<Decimal>,
    /// Realised P&L; absent when unavailable.
    pub realized: Option<Decimal>,
    /// Unrealised P&L; absent when unavailable.
    pub unrealized: Option<Decimal>,
    /// Realised plus unrealised, separate from net invested (BR2.2).
    pub total_return: Option<Decimal>,
    /// Consumed plus remaining acquisition cost, never called net invested.
    pub total_acquisition_cost: Option<Decimal>,
    /// Whether C6 marks were unavailable, so holdings are labelled unpriced.
    pub marks_unavailable: bool,
    /// Whether C5 holdings were unavailable, so coverage fell back to the ledger.
    pub holdings_unavailable: bool,
    /// Whether a non-base amount could not be converted through C6 FX.
    pub fx_unavailable: bool,
    /// Share of held value with acquisition history and a price, 0 to 1.
    pub coverage: Decimal,
    /// The coverage label.
    pub label: CoverageLabel,
    /// The uncovered or unpriced assets, named and excluded from the figures.
    pub coverage_gap: Vec<String>,
    /// Per-asset figures.
    pub assets: Vec<AssetPnl>,
}

/// Computes a P&L report from a replay, C5 holdings, and C6 marks.
///
/// Held quantities come from Account's C5 holdings; when C5 has not yet seen a
/// recorded trade, the ledger's own lot quantity is used instead. An asset is
/// covered when it has no shortfall and its lot quantity covers the held
/// quantity. Coverage is by value (held quantity × C6 mark), and an unpriced
/// asset is named in the gap and never valued at zero.
///
/// # Errors
/// Returns [`DomainError`] when a price or lot currency cannot be converted to
/// the base currency.
pub fn compute_pnl(
    as_of: DateTime<Utc>,
    outcome: &ReplayOutcome,
    holdings: &[HeldAsset],
    prices: &BTreeMap<String, PriceMark>,
    fx: &FxRates,
) -> Result<PnlReport, DomainError> {
    let held_by_asset: BTreeMap<String, Decimal> = aggregate_holdings(holdings);
    let asset_ids: BTreeSet<String> = outcome
        .lots
        .iter()
        .map(|lot| lot.asset_id().to_string())
        .chain(outcome.shortfall_by_asset.keys().cloned())
        .chain(
            held_by_asset
                .iter()
                .filter(|(_, quantity)| **quantity > Decimal::ZERO)
                .map(|(asset, _)| asset.clone()),
        )
        .collect();

    let mut assets = Vec::with_capacity(asset_ids.len());
    let mut coverage_gap = Vec::new();
    let mut covered_value = Decimal::ZERO;
    let mut total_value = Decimal::ZERO;
    let mut covered_net_invested = Decimal::ZERO;
    let mut covered_realized = Decimal::ZERO;
    let mut covered_unrealized = Decimal::ZERO;

    for asset_id in asset_ids {
        let lots = outcome.open_lots_for(&asset_id);
        let lot_quantity: Decimal = lots.iter().map(|lot| lot.remaining()).sum();
        let mut remaining_cost = Decimal::ZERO;
        for lot in &lots {
            remaining_cost += convert_cost(lot.remaining() * lot.unit_cost(), lot.currency(), fx)?;
        }
        let shortfall = outcome
            .shortfall_by_asset
            .get(&asset_id)
            .copied()
            .unwrap_or(Decimal::ZERO);
        let held_from_c5 = held_by_asset
            .get(&asset_id)
            .copied()
            .unwrap_or(Decimal::ZERO);
        let held = if held_from_c5 > Decimal::ZERO {
            held_from_c5
        } else {
            lot_quantity
        };
        let mark = prices.get(&asset_id);
        let mark_base = match mark {
            Some(price_mark) if price_mark.freshness != PriceFreshness::Unpriced => {
                match price_mark.price {
                    Some(price) => Some(convert_cost(
                        price,
                        price_mark.currency.as_deref().unwrap_or_else(|| fx.base()),
                        fx,
                    )?),
                    None => None,
                }
            }
            _ => None,
        };
        let has_history = shortfall == Decimal::ZERO && lot_quantity >= held;
        let covered = has_history && mark_base.is_some();
        let realized = outcome
            .realized_by_asset
            .get(&asset_id)
            .copied()
            .unwrap_or(Decimal::ZERO);
        // When C5 reports a smaller holding than the ledger's lots (a recorded
        // trade C5 has not synced yet), the open-lot cost is scaled to the held
        // quantity so net invested describes what is actually held.
        let held_cost = if lot_quantity > Decimal::ZERO && held < lot_quantity {
            remaining_cost * held / lot_quantity
        } else {
            remaining_cost
        };
        let unrealized = if covered {
            mark_base.unwrap_or(Decimal::ZERO) * held - held_cost
        } else {
            Decimal::ZERO
        };

        if let Some(mark_base) = mark_base {
            total_value += mark_base * held;
            if covered {
                covered_value += mark_base * held;
            }
        }
        if covered {
            covered_net_invested += held_cost;
            covered_realized += realized;
            covered_unrealized += unrealized;
        } else {
            coverage_gap.push(asset_id.clone());
        }

        assets.push(AssetPnl {
            asset_id,
            net_invested: covered.then_some(held_cost),
            realized: covered.then_some(realized),
            unrealized: covered.then_some(unrealized),
            total_return: covered.then_some(realized + unrealized),
            mark_price: mark_base,
            price_as_of: mark.and_then(|mark| mark.as_of),
            price_freshness: mark
                .map(|mark| mark.freshness.as_str().to_string())
                .unwrap_or_else(|| PriceFreshness::Unpriced.as_str().to_string()),
            covered,
        });
    }

    coverage_gap.sort();
    coverage_gap.dedup();
    let coverage = if total_value > Decimal::ZERO {
        (covered_value / total_value).round_dp(4)
    } else if coverage_gap.is_empty() {
        Decimal::ONE
    } else {
        Decimal::ZERO
    };
    let label = if coverage_gap.is_empty() {
        CoverageLabel::Complete
    } else if covered_value <= Decimal::ZERO || coverage <= Decimal::ZERO {
        CoverageLabel::Unavailable
    } else {
        CoverageLabel::Partial
    };

    let (net_invested, realized, unrealized, total_return, total_acquisition_cost) = match label {
        CoverageLabel::Complete | CoverageLabel::Partial => (
            Some(covered_net_invested),
            Some(covered_realized),
            Some(covered_unrealized),
            Some(covered_realized + covered_unrealized),
            Some(outcome.total_acquisition_cost()),
        ),
        CoverageLabel::Unavailable => (None, None, None, None, None),
    };

    Ok(PnlReport {
        as_of,
        currency: fx.base().to_string(),
        fx_as_of: fx.as_of(),
        fx_estimated: fx.is_estimated(),
        net_invested,
        realized,
        unrealized,
        total_return,
        total_acquisition_cost,
        marks_unavailable: false,
        holdings_unavailable: false,
        fx_unavailable: false,
        coverage,
        label,
        coverage_gap,
        assets,
    })
}

fn aggregate_holdings(holdings: &[HeldAsset]) -> BTreeMap<String, Decimal> {
    let mut aggregated: BTreeMap<String, Decimal> = BTreeMap::new();
    for holding in holdings {
        *aggregated
            .entry(holding.asset_id.clone())
            .or_insert(Decimal::ZERO) += holding.quantity;
    }
    aggregated
}

fn convert_cost(amount: Decimal, currency: &str, fx: &FxRates) -> Result<Decimal, DomainError> {
    fx.convert(amount, currency)
        .ok_or_else(|| DomainError::Invalid {
            field: "currency",
            reason: format!("no FX rate from {currency} to {}", fx.base()),
        })
}
