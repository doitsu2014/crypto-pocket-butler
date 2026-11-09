//! Brief composition: turn one C8 valuation read and one C7 performance read into
//! the sections of a brief.
//!
//! BR1.2: every figure carries the instant it was read (`as_of`) and the overall
//! data status. BR1.3: a figure that could not be computed stays `None` so the
//! reader shows an em dash, never a zero.

use std::cmp::Ordering;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::brief::{DataStatus, Period};

/// One priced holding read from C8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationLineRead {
    /// The canonical asset.
    pub asset_id: String,
    /// The display symbol, when C8 knows it.
    pub symbol: Option<String>,
    /// The holding's value in the brief currency, absent when unpriced.
    pub value: Option<Decimal>,
    /// The share of the portfolio, absent when the total is unknown.
    pub allocation_percent: Option<Decimal>,
    /// The saved target, absent when the user set none.
    pub target_percent: Option<Decimal>,
    /// Target minus allocation, absent when either is unknown.
    pub drift_points: Option<Decimal>,
    /// The freshness of this line.
    pub data_status: DataStatus,
    /// Where the price came from (C8 provenance).
    pub price_source: Option<String>,
    /// `live`, `delayed`, `stale`, or `unpriced`.
    pub price_freshness: String,
    /// When the quote was observed.
    pub price_observed_at: Option<DateTime<Utc>>,
    /// The FX source, when a conversion happened.
    pub fx_source: Option<String>,
    /// When the FX rate was observed.
    pub fx_observed_at: Option<DateTime<Utc>>,
}

/// The C8 valuation read a brief is built from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationRead {
    /// The portfolio total, absent when nothing could be priced.
    pub total_value: Option<Decimal>,
    /// The ISO currency every figure is in.
    pub currency: String,
    /// The valuation's overall status.
    pub status: DataStatus,
    /// The instant the figures were read.
    pub as_of: DateTime<Utc>,
    /// How many holdings had no price.
    pub unpriced_count: usize,
    /// The priced and unpriced lines.
    pub lines: Vec<ValuationLineRead>,
}

/// The C7 performance read a brief is built from.
#[derive(Debug, Clone, PartialEq)]
pub struct PnlRead {
    /// Realised profit and loss over the window, absent when it could not be
    /// computed.
    pub realized: Option<Decimal>,
    /// How much was invested, absent when it could not be computed.
    pub net_invested: Option<Decimal>,
    /// Unrealised profit and loss, absent when marks were unavailable.
    pub unrealized: Option<Decimal>,
    /// How much of the held value the figures cover, 0..1.
    pub coverage: Option<f64>,
    /// The ISO currency of the figure.
    pub currency: Option<String>,
    /// The FX instant used, when a conversion happened.
    pub fx_as_of: Option<DateTime<Utc>>,
    /// Whether the FX rate was estimated rather than observed.
    pub fx_estimated: bool,
    /// The instant the figures were read.
    pub as_of: DateTime<Utc>,
    /// The performance read's status.
    pub status: DataStatus,
}

/// The net-worth section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetWorthSection {
    /// The total, absent when nothing could be priced.
    pub total_value: Option<Decimal>,
    /// The ISO currency.
    pub currency: String,
    /// The instant the figures were read.
    pub as_of: DateTime<Utc>,
    /// The status of the figures.
    pub data_status: DataStatus,
    /// How many holdings had no price.
    pub unpriced_count: usize,
}

/// One asset row in the allocation section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllocationRow {
    /// The canonical asset.
    pub asset_id: String,
    /// The display symbol.
    pub symbol: String,
    /// The holding's value, absent when unpriced.
    pub value: Option<Decimal>,
    /// The share of the portfolio, absent when the total is unknown.
    pub allocation_percent: Option<Decimal>,
    /// The saved target, absent when none was set.
    pub target_percent: Option<Decimal>,
    /// Target minus allocation, absent when either is unknown.
    pub drift_points: Option<Decimal>,
    /// The freshness of this row.
    pub data_status: DataStatus,
    /// The instant this row's figure was read (BR1.2).
    pub as_of: DateTime<Utc>,
    /// Where the price came from (C8 provenance).
    pub price_source: Option<String>,
    /// `live`, `delayed`, `stale`, or `unpriced`.
    pub price_freshness: String,
    /// When the quote was observed.
    pub price_observed_at: Option<DateTime<Utc>>,
    /// The FX source, when a conversion happened.
    pub fx_source: Option<String>,
    /// When the FX rate was observed.
    pub fx_observed_at: Option<DateTime<Utc>>,
}

/// The performance section.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceSection {
    /// Realised profit and loss, absent when it could not be computed.
    pub realized: Option<Decimal>,
    /// How much was invested, absent when it could not be computed.
    pub net_invested: Option<Decimal>,
    /// Unrealised profit and loss, absent when marks were unavailable.
    pub unrealized: Option<Decimal>,
    /// How much of the held value the figures cover, 0..1.
    pub coverage: Option<f64>,
    /// The ISO currency of the figure.
    pub currency: Option<String>,
    /// The FX instant used, when a conversion happened.
    pub fx_as_of: Option<DateTime<Utc>>,
    /// Whether the FX rate was estimated rather than observed.
    pub fx_estimated: bool,
    /// The status of the figures.
    pub data_status: DataStatus,
    /// The instant the figures were read.
    pub as_of: DateTime<Utc>,
}

/// The composed brief body.
#[derive(Debug, Clone, PartialEq)]
pub struct BriefSections {
    /// The period the brief covers.
    pub period: Period,
    /// The net-worth section.
    pub net_worth: NetWorthSection,
    /// The allocation rows, largest holding first.
    pub allocation: Vec<AllocationRow>,
    /// The performance section.
    pub performance: PerformanceSection,
    /// Plain highlights, safe to show without formatting.
    pub highlights: Vec<String>,
    /// The weakest status across the sections (BR1.2).
    pub data_status: DataStatus,
}

/// Composes the brief sections. The overall status is the weakest of the two
/// reads, so a brief never claims better data than its weakest source (BR1.2);
/// an absent figure stays absent (BR1.3).
#[must_use]
pub fn compose_sections(period: Period, valuation: &ValuationRead, pnl: &PnlRead) -> BriefSections {
    let mut allocation: Vec<AllocationRow> = valuation
        .lines
        .iter()
        .map(|line| AllocationRow {
            asset_id: line.asset_id.clone(),
            symbol: line
                .symbol
                .clone()
                .filter(|symbol| !symbol.trim().is_empty())
                .unwrap_or_else(|| line.asset_id.clone()),
            value: line.value,
            allocation_percent: line.allocation_percent,
            target_percent: line.target_percent,
            drift_points: line.drift_points,
            data_status: line.data_status,
            as_of: valuation.as_of,
            price_source: line.price_source.clone(),
            price_freshness: line.price_freshness.clone(),
            price_observed_at: line.price_observed_at,
            fx_source: line.fx_source.clone(),
            fx_observed_at: line.fx_observed_at,
        })
        .collect();
    allocation.sort_by(|left, right| match (&left.value, &right.value) {
        (Some(left_value), Some(right_value)) => right_value
            .cmp(left_value)
            .then_with(|| left.asset_id.cmp(&right.asset_id)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.asset_id.cmp(&right.asset_id),
    });

    let data_status = valuation.status.weakest(pnl.status);
    let mut highlights = Vec::new();
    if valuation.total_value.is_none() {
        highlights.push("Your portfolio total could not be computed for this period.".to_string());
    }
    if valuation.unpriced_count > 0 {
        highlights.push(format!(
            "{} holding(s) had no price; their values are shown as —.",
            valuation.unpriced_count
        ));
    }
    if pnl.realized.is_none() {
        highlights.push("Realised performance could not be computed for this period.".to_string());
    } else if pnl.fx_estimated {
        highlights.push(
            "Performance uses an estimated FX rate; the figure is marked as estimated.".to_string(),
        );
    }
    let drifted = allocation
        .iter()
        .filter(|row| {
            row.drift_points
                .is_some_and(|drift| drift.abs() > Decimal::from(0))
        })
        .count();
    if drifted > 0 {
        highlights.push(format!(
            "{drifted} holding(s) are away from their saved target."
        ));
    }

    BriefSections {
        period,
        net_worth: NetWorthSection {
            total_value: valuation.total_value,
            currency: valuation.currency.clone(),
            as_of: valuation.as_of,
            data_status: valuation.status,
            unpriced_count: valuation.unpriced_count,
        },
        allocation,
        performance: PerformanceSection {
            realized: pnl.realized,
            net_invested: pnl.net_invested,
            unrealized: pnl.unrealized,
            coverage: pnl.coverage,
            currency: pnl.currency.clone(),
            fx_as_of: pnl.fx_as_of,
            fx_estimated: pnl.fx_estimated,
            data_status: pnl.status,
            as_of: pnl.as_of,
        },
        highlights,
        data_status,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn decimal(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("a valid decimal")
    }

    fn line(asset: &str, value: Option<&str>, allocation: Option<&str>) -> ValuationLineRead {
        ValuationLineRead {
            asset_id: asset.to_string(),
            symbol: Some(asset.to_uppercase()),
            value: value.map(decimal),
            allocation_percent: allocation.map(decimal),
            target_percent: Some(decimal("50")),
            drift_points: allocation.map(|share| decimal("50") - decimal(share)),
            data_status: if value.is_some() {
                DataStatus::Complete
            } else {
                DataStatus::Partial
            },
            price_source: Some("binance".to_string()),
            price_freshness: if value.is_some() {
                "live".to_string()
            } else {
                "unpriced".to_string()
            },
            price_observed_at: value.map(|_| at()),
            fx_source: None,
            fx_observed_at: None,
        }
    }

    fn valuation(lines: Vec<ValuationLineRead>, status: DataStatus) -> ValuationRead {
        let total =
            if status == DataStatus::Complete && lines.iter().all(|line| line.value.is_some()) {
                let mut running = Decimal::ZERO;
                for line in &lines {
                    if let Some(value) = line.value {
                        running += value;
                    }
                }
                Some(running)
            } else {
                None
            };
        ValuationRead {
            total_value: total,
            currency: "USD".to_string(),
            status,
            as_of: DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant"),
            unpriced_count: lines.iter().filter(|line| line.value.is_none()).count(),
            lines,
        }
    }

    fn at() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
    }

    fn pnl(realized: Option<&str>, status: DataStatus) -> PnlRead {
        PnlRead {
            realized: realized.map(decimal),
            net_invested: Some(decimal("900")),
            unrealized: Some(decimal("10")),
            coverage: Some(1.0),
            currency: Some("USD".to_string()),
            fx_as_of: None,
            fx_estimated: false,
            as_of: at(),
            status,
        }
    }

    #[test]
    fn a_complete_brief_carries_the_total_and_every_row() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![
                    line("btc", Some("600"), Some("60")),
                    line("eth", Some("400"), Some("40")),
                ],
                DataStatus::Complete,
            ),
            &pnl(Some("12.5"), DataStatus::Complete),
        );
        assert_eq!(sections.data_status, DataStatus::Complete);
        assert_eq!(sections.net_worth.total_value, Some(decimal("1000")));
        assert_eq!(sections.allocation.len(), 2);
        assert_eq!(sections.performance.realized, Some(decimal("12.5")));
    }

    #[test]
    fn an_unpriced_holding_leaves_its_value_absent_but_the_row_present() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![
                    line("btc", Some("600"), Some("100")),
                    line("eth", None, None),
                ],
                DataStatus::Partial,
            ),
            &pnl(Some("1"), DataStatus::Complete),
        );
        assert_eq!(sections.net_worth.total_value, None);
        assert_eq!(sections.allocation.len(), 2);
        let unpriced = sections
            .allocation
            .iter()
            .find(|row| row.asset_id == "eth")
            .expect("the row is present");
        assert_eq!(unpriced.value, None);
        assert_eq!(unpriced.allocation_percent, None);
        assert_eq!(unpriced.drift_points, None);
    }

    #[test]
    fn the_overall_status_is_the_weaker_of_the_two_reads() {
        let sections = compose_sections(
            Period::Weekly,
            &valuation(
                vec![line("btc", Some("10"), Some("100"))],
                DataStatus::Complete,
            ),
            &pnl(None, DataStatus::Unavailable),
        );
        assert_eq!(sections.data_status, DataStatus::Unavailable);
    }

    #[test]
    fn rows_are_ordered_by_value_and_unpriced_rows_come_last() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![
                    line("eth", Some("400"), Some("40")),
                    line("xyz", None, None),
                    line("btc", Some("600"), Some("60")),
                ],
                DataStatus::Partial,
            ),
            &pnl(Some("0"), DataStatus::Complete),
        );
        let order: Vec<&str> = sections
            .allocation
            .iter()
            .map(|row| row.asset_id.as_str())
            .collect();
        assert_eq!(order, vec!["btc", "eth", "xyz"]);
    }

    #[test]
    fn a_missing_symbol_falls_back_to_the_asset_id() {
        let mut priced = line("btc", Some("10"), Some("100"));
        priced.symbol = None;
        let sections = compose_sections(
            Period::Daily,
            &valuation(vec![priced], DataStatus::Complete),
            &pnl(Some("0"), DataStatus::Complete),
        );
        assert_eq!(sections.allocation[0].symbol, "btc");
    }

    #[test]
    fn a_blank_symbol_also_falls_back_to_the_asset_id() {
        let mut priced = line("btc", Some("10"), Some("100"));
        priced.symbol = Some("  ".to_string());
        let sections = compose_sections(
            Period::Daily,
            &valuation(vec![priced], DataStatus::Complete),
            &pnl(Some("0"), DataStatus::Complete),
        );
        assert_eq!(sections.allocation[0].symbol, "btc");
    }

    #[test]
    fn an_unavailable_valuation_produces_a_highlight_not_a_zero() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(Vec::new(), DataStatus::Unavailable),
            &pnl(None, DataStatus::Unavailable),
        );
        assert_eq!(sections.net_worth.total_value, None);
        assert!(sections
            .highlights
            .iter()
            .any(|line| line.contains("could not be computed")));
    }

    #[test]
    fn an_estimated_fx_rate_is_announced() {
        let mut performance = pnl(Some("10"), DataStatus::Complete);
        performance.fx_estimated = true;
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![line("btc", Some("10"), Some("100"))],
                DataStatus::Complete,
            ),
            &performance,
        );
        assert!(sections.performance.fx_estimated);
        assert!(sections
            .highlights
            .iter()
            .any(|line| line.contains("estimated")));
    }

    #[test]
    fn a_row_away_from_its_target_is_announced() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![line("btc", Some("60"), Some("60"))],
                DataStatus::Complete,
            ),
            &pnl(Some("0"), DataStatus::Complete),
        );
        assert!(sections
            .highlights
            .iter()
            .any(|line| line.contains("away from their saved target")));
    }

    #[test]
    fn a_complete_brief_with_no_drift_announces_no_holdings_away_from_target() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![line("btc", Some("50"), Some("50"))],
                DataStatus::Complete,
            ),
            &pnl(Some("0"), DataStatus::Complete),
        );
        assert!(!sections
            .highlights
            .iter()
            .any(|line| line.contains("away from their saved target")));
    }

    #[test]
    fn unpriced_holdings_are_counted_in_the_section() {
        let sections = compose_sections(
            Period::Daily,
            &valuation(
                vec![
                    line("btc", Some("60"), Some("100")),
                    line("eth", None, None),
                ],
                DataStatus::Partial,
            ),
            &pnl(Some("0"), DataStatus::Complete),
        );
        assert_eq!(sections.net_worth.unpriced_count, 1);
        assert!(sections
            .highlights
            .iter()
            .any(|line| line.contains("had no price")));
    }
}
