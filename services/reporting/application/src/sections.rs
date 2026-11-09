//! The JSON projection of a composed brief.
//!
//! The domain composes typed sections; this module is the one place that decides
//! the wire and storage shape. A missing figure is `null` so the reader renders
//! an em dash, never a zero (BR1.3).

use reporting_domain::compose::BriefSections;
use rust_decimal::Decimal;
use serde_json::{json, Map, Value};

fn decimal(value: Decimal) -> Value {
    Value::String(value.to_string())
}

fn percent(value: Decimal) -> Value {
    Value::String(value.round_dp(2).to_string())
}

/// Projects the composed sections onto the C1 brief body.
#[must_use]
pub fn sections_json(sections: &BriefSections) -> Value {
    let allocation: Vec<Value> = sections
        .allocation
        .iter()
        .map(|row| {
            json!({
                "assetId": row.asset_id,
                "symbol": row.symbol,
                "value": row.value.map(decimal),
                "allocationPercent": row.allocation_percent.map(percent),
                "targetPercent": row.target_percent.map(percent),
                "driftPoints": row.drift_points.map(percent),
                "dataStatus": row.data_status.as_str(),
                "asOf": row.as_of,
                "priceSource": row.price_source,
                "priceFreshness": row.price_freshness,
                "priceObservedAt": row.price_observed_at,
                "fxSource": row.fx_source,
                "fxObservedAt": row.fx_observed_at,
            })
        })
        .collect();
    json!({
        "period": sections.period.as_str(),
        "netWorth": {
            "totalValue": sections.net_worth.total_value.map(decimal),
            "currency": sections.net_worth.currency,
            "asOf": sections.net_worth.as_of,
            "dataStatus": sections.net_worth.data_status.as_str(),
            "unpricedCount": sections.net_worth.unpriced_count,
        },
        "allocation": allocation,
        "performance": {
            "realized": sections.performance.realized.map(decimal),
            "netInvested": sections.performance.net_invested.map(decimal),
            "unrealized": sections.performance.unrealized.map(decimal),
            "coverage": sections.performance.coverage,
            "currency": sections.performance.currency,
            "fxAsOf": sections.performance.fx_as_of,
            "fxEstimated": sections.performance.fx_estimated,
            "dataStatus": sections.performance.data_status.as_str(),
            "asOf": sections.performance.as_of,
        },
        "highlights": sections.highlights,
        "dataStatus": sections.data_status.as_str(),
    })
}

/// The public payload of the email alert that announces a brief (BR2.2). It
/// carries identifiers and the headline status only; no figure is sent through
/// Notification.
#[must_use]
pub fn email_payload(
    brief_id: &str,
    period: &str,
    as_of: &str,
    data_status: &str,
    has_unpriced: bool,
) -> Value {
    let mut payload = Map::new();
    payload.insert("briefId".to_string(), json!(brief_id));
    payload.insert("period".to_string(), json!(period));
    payload.insert("asOf".to_string(), json!(as_of));
    payload.insert("dataStatus".to_string(), json!(data_status));
    payload.insert("hasUnpriced".to_string(), json!(has_unpriced));
    Value::Object(payload)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::DateTime;
    use reporting_domain::compose::{AllocationRow, NetWorthSection, PerformanceSection};
    use reporting_domain::{DataStatus, Period};
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn decimal_of(raw: &str) -> Decimal {
        Decimal::from_str(raw).expect("a valid decimal")
    }

    pub(crate) fn composed() -> BriefSections {
        sections(None)
    }

    fn sections(total: Option<Decimal>) -> BriefSections {
        BriefSections {
            period: Period::Daily,
            net_worth: NetWorthSection {
                total_value: total,
                currency: "USD".to_string(),
                as_of: DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant"),
                data_status: DataStatus::Partial,
                unpriced_count: 1,
            },
            allocation: vec![AllocationRow {
                asset_id: "btc".to_string(),
                symbol: "BTC".to_string(),
                value: Some(decimal_of("600")),
                allocation_percent: Some(decimal_of("100")),
                target_percent: Some(decimal_of("50")),
                drift_points: Some(decimal_of("50")),
                data_status: DataStatus::Complete,
                as_of: DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant"),
                price_source: Some("binance".to_string()),
                price_freshness: "live".to_string(),
                price_observed_at: Some(
                    DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant"),
                ),
                fx_source: Some("ecb".to_string()),
                fx_observed_at: None,
            }],
            performance: PerformanceSection {
                realized: None,
                net_invested: Some(decimal_of("500")),
                unrealized: None,
                coverage: Some(0.5),
                currency: Some("USD".to_string()),
                fx_as_of: None,
                fx_estimated: false,
                data_status: DataStatus::Unavailable,
                as_of: DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant"),
            },
            highlights: vec!["A holding had no price.".to_string()],
            data_status: DataStatus::Partial,
        }
    }

    #[test]
    fn a_missing_total_is_null_rather_than_zero() {
        let body = sections_json(&sections(None));
        assert!(body["netWorth"]["totalValue"].is_null());
        assert_eq!(body["dataStatus"], "partial");
    }

    #[test]
    fn a_present_total_is_a_decimal_string() {
        let body = sections_json(&sections(Some(decimal_of("1000"))));
        assert_eq!(body["netWorth"]["totalValue"], "1000");
    }

    #[test]
    fn allocation_rows_carry_both_freshness_and_targets() {
        let body = sections_json(&sections(Some(decimal_of("600"))));
        let row = &body["allocation"][0];
        assert_eq!(row["assetId"], "btc");
        assert_eq!(row["symbol"], "BTC");
        assert_eq!(row["allocationPercent"], "100");
        assert_eq!(row["targetPercent"], "50");
        assert_eq!(row["driftPoints"], "50");
        assert_eq!(row["dataStatus"], "complete");
    }

    #[test]
    fn a_missing_realised_figure_is_null() {
        let body = sections_json(&sections(Some(decimal_of("600"))));
        assert!(body["performance"]["realized"].is_null());
        assert_eq!(body["performance"]["fxEstimated"], false);
    }

    #[test]
    fn percentages_are_rounded_to_two_places() {
        let mut composed = sections(Some(decimal_of("600")));
        composed.allocation[0].allocation_percent = Some(decimal_of("33.333333"));
        let body = sections_json(&composed);
        assert_eq!(body["allocation"][0]["allocationPercent"], "33.33");
    }

    #[test]
    fn the_email_payload_names_the_status_and_no_figure() {
        let payload = email_payload("brief-1", "daily", "2026-11-08T07:00:00Z", "partial", true);
        assert_eq!(payload["briefId"], "brief-1");
        assert_eq!(payload["period"], "daily");
        assert_eq!(payload["dataStatus"], "partial");
        assert_eq!(payload["hasUnpriced"], true);
        assert!(payload.get("totalValue").is_none());
    }
}

#[cfg(test)]
mod provenance_tests {
    use super::sections_json;
    use crate::sections::tests::composed;

    #[test]
    fn an_allocation_row_carries_its_provenance_and_as_of() {
        let body = sections_json(&composed());
        let row = &body["allocation"][0];
        assert_eq!(row["priceSource"], "binance");
        assert_eq!(row["priceFreshness"], "live");
        assert!(row["priceObservedAt"].is_string());
        assert!(row["asOf"].is_string());
    }

    #[test]
    fn the_performance_section_carries_coverage_and_the_remaining_figures() {
        let body = sections_json(&composed());
        assert_eq!(body["performance"]["coverage"], 0.5);
        assert_eq!(body["performance"]["netInvested"], "500");
        assert!(body["performance"]["unrealized"].is_null());
    }
}
