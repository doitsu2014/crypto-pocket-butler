//! The C8 valuation read.
//!
//! Reporting composes a user-level valuation from the user's portfolios: it lists
//! them, reads each one's valuation with its lines, and folds them into one
//! `ValuationRead`. A portfolio's total is only summed when every portfolio has
//! one; a holding with no price stays unpriced and never becomes zero (BR1.3).

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_application::ports::{ExternalError, ValuationClient};
use reporting_domain::{DataStatus, UserId, ValuationLineRead, ValuationRead};
use rust_decimal::Decimal;
use serde::Deserialize;

/// How many portfolio valuation reads run at once.
pub const PORTFOLIO_CONCURRENCY: usize = 8;

/// Reads valuations from Portfolio.
pub struct HttpValuation {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpValuation {
    /// Creates the client with a 400 ms per-call budget.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(200))
            .timeout(Duration::from_millis(400))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }

    async fn portfolios(&self, user_id: UserId) -> Result<Vec<String>, ExternalError> {
        let url = format!(
            "{}/v1/users/{}/portfolios",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("the portfolio list read failed: {error}"))
            })?;
        if !response.status().is_success() {
            return Err(ExternalError::Refused(format!(
                "the portfolio list read answered {}",
                response.status()
            )));
        }
        let rows = response
            .json::<Vec<PortfolioDto>>()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("the portfolio list was malformed: {error}"))
            })?;
        Ok(rows.into_iter().map(|row| row.id).collect())
    }

    async fn valuation(&self, portfolio_id: &str) -> Result<PortfolioValuationDto, ExternalError> {
        let url = format!(
            "{}/v1/portfolios/{portfolio_id}/valuation",
            self.base_url.trim_end_matches('/')
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("the valuation read failed: {error}"))
            })?;
        if !response.status().is_success() {
            return Err(ExternalError::Refused(format!(
                "the valuation read answered {}",
                response.status()
            )));
        }
        response
            .json::<PortfolioValuationDto>()
            .await
            .map_err(|error| {
                ExternalError::Unavailable(format!("the valuation body was malformed: {error}"))
            })
    }
}

#[derive(Debug, Deserialize)]
struct PortfolioDto {
    id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PortfolioValuationDto {
    as_of: DateTime<Utc>,
    status: String,
    total_value: Option<String>,
    currency: String,
    unpriced_count: u64,
    lines: Vec<ValuationLineDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ValuationLineDto {
    asset_id: String,
    value: Option<String>,
    allocation: Option<Decimal>,
    target_percent: Option<Decimal>,
    drift_points: Option<Decimal>,
    data_status: String,
    #[serde(default)]
    price_source: Option<String>,
    #[serde(default)]
    price_freshness: Option<String>,
    #[serde(default)]
    price_observed_at: Option<DateTime<Utc>>,
    #[serde(default)]
    fx_source: Option<String>,
    #[serde(default)]
    fx_observed_at: Option<DateTime<Utc>>,
}

fn parse_decimal(raw: &str) -> Option<Decimal> {
    raw.trim().parse::<Decimal>().ok()
}

fn status(raw: &str) -> DataStatus {
    DataStatus::parse(raw).unwrap_or(DataStatus::Unavailable)
}

fn line_unpriced(raw: &str) -> bool {
    raw.trim() == "unpriced"
}

fn line_status(raw: &str) -> DataStatus {
    match raw.trim() {
        "priced" => DataStatus::Complete,
        "unpriced" => DataStatus::Partial,
        _ => DataStatus::Unavailable,
    }
}

#[async_trait]
impl ValuationClient for HttpValuation {
    async fn valuation(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> Result<ValuationRead, ExternalError> {
        let portfolios = self.portfolios(user_id).await?;
        if portfolios.is_empty() {
            return Ok(ValuationRead {
                total_value: None,
                currency: "USD".to_string(),
                status: DataStatus::Unavailable,
                as_of: at,
                unpriced_count: 0,
                lines: Vec::new(),
            });
        }
        let mut reads: Vec<PortfolioValuationDto> = Vec::with_capacity(portfolios.len());
        for portfolio_id in &portfolios {
            reads.push(self.valuation(portfolio_id).await?);
        }
        Ok(fold(reads))
    }
}

/// Folds the per-portfolio reads into one user-level valuation.
#[must_use]
fn fold(reads: Vec<PortfolioValuationDto>) -> ValuationRead {
    let mut currency: Option<String> = None;
    let mut status = DataStatus::Complete;
    let mut as_of: Option<DateTime<Utc>> = None;
    let mut total: Option<Decimal> = Some(Decimal::ZERO);
    let mut unpriced = 0_usize;
    let mut mixed_currency = false;
    let mut lines: Vec<ValuationLineRead> = Vec::new();

    for read in reads {
        currency.get_or_insert_with(|| read.currency.clone());
        if currency.as_deref() != Some(read.currency.as_str()) {
            mixed_currency = true;
        }
        status = status.weakest(status_of(&read.status));
        as_of = Some(match as_of {
            Some(earliest) => earliest.min(read.as_of),
            None => read.as_of,
        });
        match read.total_value.as_deref().and_then(parse_decimal) {
            Some(value) => {
                total = total.map(|running| running + value);
            }
            None => total = None,
        }
        unpriced += usize::try_from(read.unpriced_count).unwrap_or(usize::MAX);
        for line in read.lines {
            lines.push(ValuationLineRead {
                asset_id: line.asset_id,
                symbol: None,
                value: line.value.as_deref().and_then(parse_decimal),
                allocation_percent: line.allocation,
                target_percent: line.target_percent,
                drift_points: line.drift_points,
                data_status: line_status(&line.data_status),
                price_source: line.price_source,
                price_freshness: line.price_freshness.unwrap_or_else(|| {
                    if line_unpriced(&line.data_status) {
                        "unpriced".to_string()
                    } else {
                        "delayed".to_string()
                    }
                }),
                price_observed_at: line.price_observed_at,
                fx_source: line.fx_source,
                fx_observed_at: line.fx_observed_at,
            });
        }
    }
    if mixed_currency || status == DataStatus::Unavailable {
        total = None;
    }
    ValuationRead {
        total_value: total,
        currency: currency.unwrap_or_else(|| "USD".to_string()),
        status,
        as_of: as_of.unwrap_or_else(Utc::now),
        unpriced_count: unpriced,
        lines,
    }
}

fn status_of(raw: &str) -> DataStatus {
    status(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn at() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant")
    }

    fn read(
        currency: &str,
        status: &str,
        total: Option<&str>,
        unpriced: u64,
        lines: Vec<ValuationLineDto>,
    ) -> PortfolioValuationDto {
        PortfolioValuationDto {
            as_of: at(),
            status: status.to_string(),
            total_value: total.map(ToString::to_string),
            currency: currency.to_string(),
            unpriced_count: unpriced,
            lines,
        }
    }

    fn line(asset: &str, value: Option<&str>, status: &str) -> ValuationLineDto {
        ValuationLineDto {
            asset_id: asset.to_string(),
            value: value.map(ToString::to_string),
            allocation: None,
            target_percent: None,
            drift_points: None,
            data_status: status.to_string(),
            price_source: None,
            price_freshness: None,
            price_observed_at: None,
            fx_source: None,
            fx_observed_at: None,
        }
    }

    #[test]
    fn two_complete_portfolios_sum_their_totals() {
        let folded = fold(vec![
            read(
                "USD",
                "complete",
                Some("100"),
                0,
                vec![line("btc", Some("100"), "priced")],
            ),
            read(
                "USD",
                "complete",
                Some("250"),
                0,
                vec![line("eth", Some("250"), "priced")],
            ),
        ]);
        assert_eq!(folded.total_value, Some(Decimal::new(350, 0)));
        assert_eq!(folded.status, DataStatus::Complete);
        assert_eq!(folded.lines.len(), 2);
    }

    #[test]
    fn a_missing_total_leaves_the_user_total_absent() {
        let folded = fold(vec![read(
            "USD",
            "partial",
            None,
            1,
            vec![line("btc", None, "unpriced")],
        )]);
        assert_eq!(folded.total_value, None);
        assert_eq!(folded.status, DataStatus::Partial);
        assert_eq!(folded.unpriced_count, 1);
    }

    #[test]
    fn the_weakest_status_wins() {
        let folded = fold(vec![
            read("USD", "complete", Some("10"), 0, vec![]),
            read("USD", "unavailable", None, 0, vec![]),
        ]);
        assert_eq!(folded.status, DataStatus::Unavailable);
        assert_eq!(folded.total_value, None);
    }

    #[test]
    fn mixed_currencies_never_produce_a_total() {
        let folded = fold(vec![
            read("USD", "complete", Some("10"), 0, vec![]),
            read("EUR", "complete", Some("10"), 0, vec![]),
        ]);
        assert_eq!(folded.total_value, None);
    }

    #[test]
    fn the_as_of_is_the_earliest_read() {
        let mut later = read("USD", "complete", Some("10"), 0, vec![]);
        later.as_of = at() + Duration::hours(1);
        let folded = fold(vec![read("USD", "complete", Some("10"), 0, vec![]), later]);
        assert_eq!(folded.as_of, at());
    }

    #[test]
    fn an_unpriced_line_keeps_its_row_with_no_value() {
        let folded = fold(vec![read(
            "USD",
            "partial",
            Some("10"),
            1,
            vec![line("eth", None, "unpriced")],
        )]);
        assert_eq!(folded.lines.len(), 1);
        assert_eq!(folded.lines[0].value, None);
        assert_eq!(folded.lines[0].data_status, DataStatus::Partial);
    }

    #[test]
    fn a_malformed_decimal_is_treated_as_absent() {
        let folded = fold(vec![read(
            "USD",
            "partial",
            Some("not-a-number"),
            0,
            vec![],
        )]);
        assert_eq!(folded.total_value, None);
    }
}
