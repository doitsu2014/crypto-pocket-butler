//! The C7 performance read.
//!
//! Ledger returns the user's figures in the base currency with the flags that say
//! whether anything was degraded. A withheld figure stays absent; a degraded
//! response becomes `Partial`, never `Complete` (BR1.2).

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_application::ports::{ExternalError, PerformanceClient};
use reporting_domain::{DataStatus, PnlRead, UserId};
use rust_decimal::Decimal;
use serde::Deserialize;

/// Reads realised performance from Ledger.
pub struct HttpPerformance {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpPerformance {
    /// Creates the client with a 400 ms budget.
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PnlDto {
    realized: Option<String>,
    #[serde(default)]
    net_invested: Option<String>,
    #[serde(default)]
    unrealized: Option<String>,
    #[serde(default)]
    coverage: Option<f64>,
    currency: String,
    fx_as_of: Option<DateTime<Utc>>,
    fx_estimated: bool,
    #[serde(default)]
    as_of: Option<DateTime<Utc>>,
    #[serde(default)]
    marks_unavailable: bool,
    #[serde(default)]
    holdings_unavailable: bool,
    #[serde(default)]
    fx_unavailable: bool,
}

/// Maps the C7 flags onto the brief's data status: a withheld realisation is
/// unavailable, a degraded one is partial, a clean one complete.
#[must_use]
fn status_of(dto: &PnlDto) -> DataStatus {
    let degraded = dto.marks_unavailable || dto.holdings_unavailable || dto.fx_unavailable;
    match (dto.realized.is_some(), degraded) {
        (true, false) => DataStatus::Complete,
        (true, true) => DataStatus::Partial,
        (false, _) => DataStatus::Unavailable,
    }
}

#[async_trait]
impl PerformanceClient for HttpPerformance {
    async fn performance(
        &self,
        user_id: UserId,
        _from: DateTime<Utc>,
        _to: DateTime<Utc>,
    ) -> Result<PnlRead, ExternalError> {
        // C7 computes the realised figure to date; the brief's window is carried
        // in the brief itself, not in the call.
        let url = format!(
            "{}/v1/users/{}/pnl",
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
                ExternalError::Unavailable(format!("the performance read failed: {error}"))
            })?;
        if !response.status().is_success() {
            return Err(ExternalError::Refused(format!(
                "the performance read answered {}",
                response.status()
            )));
        }
        let dto = response.json::<PnlDto>().await.map_err(|error| {
            ExternalError::Unavailable(format!("the performance body was malformed: {error}"))
        })?;
        let status = status_of(&dto);
        Ok(PnlRead {
            realized: dto
                .realized
                .as_deref()
                .and_then(|raw| raw.trim().parse::<Decimal>().ok()),
            net_invested: dto
                .net_invested
                .as_deref()
                .and_then(|raw| raw.trim().parse::<Decimal>().ok()),
            unrealized: dto
                .unrealized
                .as_deref()
                .and_then(|raw| raw.trim().parse::<Decimal>().ok()),
            coverage: dto.coverage,
            currency: Some(dto.currency),
            fx_as_of: dto.fx_as_of,
            fx_estimated: dto.fx_estimated,
            as_of: dto.as_of.unwrap_or_else(Utc::now),
            status,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{status_of, PnlDto};
    use reporting_domain::DataStatus;

    fn dto(realized: Option<&str>, degraded: bool) -> PnlDto {
        PnlDto {
            realized: realized.map(ToString::to_string),
            net_invested: Some("100".to_string()),
            unrealized: None,
            coverage: Some(0.8),
            currency: "USD".to_string(),
            fx_as_of: None,
            fx_estimated: false,
            as_of: None,
            marks_unavailable: degraded,
            holdings_unavailable: false,
            fx_unavailable: false,
        }
    }

    #[test]
    fn a_clean_figure_is_complete() {
        assert_eq!(status_of(&dto(Some("1.5"), false)), DataStatus::Complete);
    }

    #[test]
    fn a_degraded_figure_is_partial() {
        assert_eq!(status_of(&dto(Some("1.5"), true)), DataStatus::Partial);
    }

    #[test]
    fn a_withheld_figure_is_unavailable() {
        assert_eq!(status_of(&dto(None, false)), DataStatus::Unavailable);
        assert_eq!(status_of(&dto(None, true)), DataStatus::Unavailable);
    }
}
