//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from domain and application types here; no DTO type reaches inward.

use chrono::{DateTime, Utc};
use market_data_application::RunOutcome;
use market_data_domain::{PriceQuote, RefreshRun, ServedFxRate};
use serde::{Deserialize, Serialize};

/// `GET /v1/fx` query string. A missing value is refused with 422 by the handler.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FxQuery {
    /// The base currency.
    #[serde(default)]
    pub base: Option<String>,
    /// The quote currency.
    #[serde(default)]
    pub quote: Option<String>,
}

/// One C6 price.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PriceDto {
    /// The canonical asset id.
    pub asset_id: String,
    /// The decimal price as a string; `null` when unpriced.
    pub price: Option<String>,
    /// The quote currency; `null` when unpriced.
    pub currency: Option<String>,
    /// The winning source; `null` when unpriced.
    pub source: Option<String>,
    /// The observation time; `null` when unpriced.
    pub observed_at: Option<DateTime<Utc>>,
    /// `live`, `delayed`, `stale`, or `unpriced`.
    pub freshness: String,
}

impl From<&PriceQuote> for PriceDto {
    fn from(quote: &PriceQuote) -> Self {
        Self {
            asset_id: quote.asset_id().to_string(),
            price: quote.price().map(|price| price.to_string()),
            currency: quote.currency().map(ToString::to_string),
            source: quote.source().map(ToString::to_string),
            observed_at: quote.observed_at(),
            freshness: quote.freshness().as_str().to_string(),
        }
    }
}

/// One C6 FX rate.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FxRateDto {
    /// The base currency.
    pub base: String,
    /// The quote currency.
    pub quote: String,
    /// The decimal rate as a string.
    pub rate: String,
    /// When the reading (or the older leg) was observed.
    pub observed_at: DateTime<Utc>,
    /// The source name, `derived` for a cross rate.
    pub source: String,
    /// Both leg sources when the pair is derived through USD.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<String>,
}

impl From<&ServedFxRate> for FxRateDto {
    fn from(rate: &ServedFxRate) -> Self {
        Self {
            base: rate.base().to_string(),
            quote: rate.quote().to_string(),
            rate: rate.rate().to_string(),
            observed_at: rate.observed_at(),
            source: rate.source().to_string(),
            derived_from: rate.derived_from().to_vec(),
        }
    }
}

/// The C11 run result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResultDto {
    /// The job name.
    pub job_name: String,
    /// `succeeded`, `failed`, or `skipped`.
    pub status: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// The run identifier, present when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// A short, non-secret detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl RunResultDto {
    /// Maps a terminal run.
    #[must_use]
    pub fn completed(run: &RefreshRun) -> Self {
        Self {
            job_name: run.job().job_name().to_string(),
            status: run.status().as_str().to_string(),
            started_at: run.started_at(),
            run_id: Some(run.id().to_string()),
            detail: run.error_message().map(ToString::to_string),
        }
    }

    /// Maps an already-active run (manual path, 202).
    #[must_use]
    pub fn active(run: &RefreshRun) -> Self {
        Self {
            job_name: run.job().job_name().to_string(),
            status: "skipped".to_string(),
            started_at: run.started_at(),
            run_id: Some(run.id().to_string()),
            detail: Some("a run of this job is already active".to_string()),
        }
    }

    /// Maps an application outcome.
    #[must_use]
    pub fn from_outcome(outcome: &RunOutcome) -> Self {
        match outcome {
            RunOutcome::Completed(run) => Self::completed(run),
            RunOutcome::AlreadyRunning(run) => Self::active(run),
        }
    }
}
