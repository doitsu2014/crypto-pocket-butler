//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from application commands here; no DTO type reaches application or domain.

use std::collections::BTreeMap;

use analytics_application::{FlagCommand, FlagPatch, IngestCommand};
use analytics_domain::{FeatureFlag, MetricAggregate};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `POST /v1/events` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestRequest {
    /// The catalogue event name.
    pub event_name: String,
    /// When the event happened.
    pub occurred_at: DateTime<Utc>,
    /// The caller-supplied properties; only scalars survive the boundary.
    #[serde(default)]
    pub properties: BTreeMap<String, Value>,
}

impl IngestRequest {
    /// Maps the body plus the authenticated subject to an application command.
    #[must_use]
    pub fn into_command(self, subject: String, correlation_id: Option<String>) -> IngestCommand {
        let properties = self
            .properties
            .into_iter()
            .filter_map(|(key, value)| scalar(&value).map(|value| (key, value)))
            .collect();
        IngestCommand {
            event_name: self.event_name,
            occurred_at: self.occurred_at,
            properties,
            subject,
            correlation_id,
        }
    }
}

fn scalar(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        Value::Null | Value::Array(_) | Value::Object(_) => None,
    }
}

/// `POST /v1/events` response.
#[derive(Debug, Clone, Serialize)]
pub struct IngestResponse {
    /// Always `accepted`.
    pub status: &'static str,
}

impl IngestResponse {
    /// The accepted body.
    #[must_use]
    pub fn accepted() -> Self {
        Self { status: "accepted" }
    }
}

/// `GET /v1/flags/{key}` response.
#[derive(Debug, Clone, Serialize)]
pub struct FlagResponse {
    /// The requested key.
    pub key: String,
    /// Whether the flag is on for this caller.
    pub enabled: bool,
}

/// One flag in an admin response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlagDto {
    /// The unique key.
    pub key: String,
    /// The accountable owner.
    pub owner: String,
    /// The mandatory expiry instant.
    pub expires_at: DateTime<Utc>,
    /// The rollout percentage.
    pub rollout_percentage: u8,
    /// The derived status.
    pub status: String,
    /// When the flag was created.
    pub created_at: DateTime<Utc>,
    /// When the flag was last changed.
    pub updated_at: DateTime<Utc>,
}

impl FlagDto {
    /// Maps a flag, deriving `expired` from the clock.
    #[must_use]
    pub fn from_flag(flag: &FeatureFlag, now: DateTime<Utc>) -> Self {
        Self {
            key: flag.key().to_string(),
            owner: flag.owner().to_string(),
            expires_at: flag.expires_at(),
            rollout_percentage: flag.rollout_percentage(),
            status: flag.effective_status(now).as_str().to_string(),
            created_at: flag.created_at(),
            updated_at: flag.updated_at(),
        }
    }
}

/// `GET /v1/admin/flags` query string.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListFlagsParams {
    /// When true, only flags past their expiry are returned.
    #[serde(default)]
    pub expired: Option<bool>,
}

/// `POST /v1/admin/flags` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFlagRequest {
    /// The unique key.
    pub key: String,
    /// The accountable owner.
    pub owner: String,
    /// The mandatory expiry instant.
    pub expires_at: DateTime<Utc>,
    /// The rollout percentage.
    pub rollout_percentage: u16,
    /// `active` or `disabled`.
    #[serde(default)]
    pub status: Option<String>,
}

impl From<CreateFlagRequest> for FlagCommand {
    fn from(request: CreateFlagRequest) -> Self {
        Self {
            key: request.key,
            owner: request.owner,
            expires_at: request.expires_at,
            rollout_percentage: request.rollout_percentage,
            status: request.status,
        }
    }
}

/// `PATCH /v1/admin/flags/{key}` body.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFlagRequest {
    /// A replacement owner.
    #[serde(default)]
    pub owner: Option<String>,
    /// A replacement expiry instant.
    #[serde(default)]
    pub expires_at: Option<DateTime<Utc>>,
    /// A replacement rollout percentage.
    #[serde(default)]
    pub rollout_percentage: Option<u16>,
    /// A replacement active/disabled status.
    #[serde(default)]
    pub status: Option<String>,
}

impl From<UpdateFlagRequest> for FlagPatch {
    fn from(request: UpdateFlagRequest) -> Self {
        Self {
            owner: request.owner,
            expires_at: request.expires_at,
            rollout_percentage: request.rollout_percentage,
            status: request.status,
        }
    }
}

/// One aggregate in a response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateDto {
    /// The metric.
    pub metric: String,
    /// The period.
    pub period: String,
    /// The non-identifying cohort label.
    pub cohort: String,
    /// The first instant of the period.
    pub period_start: DateTime<Utc>,
    /// The ratio, between zero and one.
    pub value: Decimal,
    /// When the figure was computed.
    pub computed_at: DateTime<Utc>,
}

impl From<&MetricAggregate> for AggregateDto {
    fn from(aggregate: &MetricAggregate) -> Self {
        Self {
            metric: aggregate.metric().as_str().to_string(),
            period: aggregate.period().as_str().to_string(),
            cohort: aggregate.cohort().to_string(),
            period_start: aggregate.period_start(),
            value: aggregate.value(),
            computed_at: aggregate.computed_at(),
        }
    }
}

/// `GET /v1/analytics/aggregates` query string.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateQueryParams {
    /// A metric label.
    #[serde(default)]
    pub metric: Option<String>,
    /// `daily` or `weekly`.
    #[serde(default)]
    pub period: Option<String>,
    /// The lower bound on `periodStart`.
    #[serde(default)]
    pub from: Option<DateTime<Utc>>,
    /// The upper bound on `periodStart`.
    #[serde(default)]
    pub to: Option<DateTime<Utc>>,
    /// The page size.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// `GET /v1/analytics/aggregates` response.
#[derive(Debug, Clone, Serialize)]
pub struct AggregatePageResponse {
    /// The figures.
    pub aggregates: Vec<AggregateDto>,
}

/// `POST /v1/internal/privacy/erase` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EraseRequest {
    /// The raw Supabase Auth subject.
    pub auth_user_id: String,
}

/// `POST /v1/internal/run/analytics-aggregate` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregateRunResponse {
    /// Always `success`.
    pub status: &'static str,
    /// The closed day.
    pub period_start: DateTime<Utc>,
    /// The figures stored.
    pub stored: usize,
    /// The cohorts suppressed.
    pub suppressed: usize,
    /// Events read from the daily window.
    pub scanned_daily: usize,
    /// Events read from the day-7 cohort window.
    pub scanned_cohort_d7: usize,
    /// Events read from the day-30 cohort window.
    pub scanned_cohort_d30: usize,
}

/// `POST /v1/internal/run/analytics-retention` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionRunResponse {
    /// Always `success`.
    pub status: &'static str,
    /// Raw events removed.
    pub events_deleted: u64,
    /// Consent-check rows removed.
    pub consent_checks_deleted: u64,
}
