//! The HTTP boundary's request and response bodies. DTOs never leak inward.

use chrono::{DateTime, Utc};
use reporting_application::{BriefView, RunReport};
use reporting_domain::{Brief, BriefDelivery};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `GET /v1/users/{userId}/briefs` query string.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListQuery {
    /// `daily` or `weekly`; absent lists both.
    #[serde(default)]
    pub period: Option<String>,
    /// The page size.
    #[serde(default)]
    pub limit: Option<i64>,
    /// The `updatedAt` cursor from the previous page.
    #[serde(default)]
    pub cursor: Option<DateTime<Utc>>,
}

/// One brief without its sections, for the list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefSummaryDto {
    /// The brief identifier.
    pub id: String,
    /// `daily` or `weekly`.
    pub period: String,
    /// The first instant of the window.
    pub period_start: DateTime<Utc>,
    /// The window's exclusive end.
    pub period_end: DateTime<Utc>,
    /// `pending`, `generated`, or `failed`.
    pub status: String,
    /// The explicit failure reason, when the brief failed.
    pub failure_reason: Option<String>,
    /// The instant the figures were read.
    pub as_of: Option<DateTime<Utc>>,
    /// `complete`, `partial`, or `unavailable`.
    pub data_status: Option<String>,
    /// The instant the brief was generated, when it was.
    pub generated_at: Option<DateTime<Utc>>,
    /// When the record was created.
    pub created_at: DateTime<Utc>,
    /// When the record last changed.
    pub updated_at: DateTime<Utc>,
}

impl From<&Brief> for BriefSummaryDto {
    fn from(brief: &Brief) -> Self {
        Self {
            id: brief.id().to_string(),
            period: brief.period().as_str().to_string(),
            period_start: brief.period_start(),
            period_end: brief.period_end(),
            status: brief.status().as_str().to_string(),
            failure_reason: brief.failure_reason().map(ToString::to_string),
            as_of: brief.as_of(),
            data_status: brief
                .data_status()
                .map(|status| status.as_str().to_string()),
            generated_at: brief.as_of(),
            created_at: brief.created_at(),
            updated_at: brief.updated_at(),
        }
    }
}

/// One delivery attempt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryDto {
    /// `in_app` or `email`.
    pub channel: String,
    /// `submitted`, `suppressed`, or `failed`.
    pub status: String,
    /// The C9 alert identifier, when Notification accepted it.
    pub alert_id: Option<String>,
    /// When the attempt was made.
    pub submitted_at: DateTime<Utc>,
}

impl From<&BriefDelivery> for DeliveryDto {
    fn from(delivery: &BriefDelivery) -> Self {
        Self {
            channel: delivery.channel().as_str().to_string(),
            status: delivery.status().as_str().to_string(),
            alert_id: delivery.alert_id().map(ToString::to_string),
            submitted_at: delivery.submitted_at(),
        }
    }
}

/// `GET /v1/briefs/{briefId}` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BriefDto {
    /// The brief record.
    #[serde(flatten)]
    pub brief: BriefSummaryDto,
    /// The composed sections; absent until generated.
    pub sections: Option<Value>,
    /// The delivery attempts, oldest first.
    pub deliveries: Vec<DeliveryDto>,
}

impl From<BriefView> for BriefDto {
    fn from(view: BriefView) -> Self {
        Self {
            brief: BriefSummaryDto::from(&view.brief),
            sections: view.sections,
            deliveries: view.deliveries.iter().map(DeliveryDto::from).collect(),
        }
    }
}

/// `POST /v1/internal/run/brief-generation` body.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    /// The owner instance recorded on the lease.
    #[serde(default)]
    pub owner: Option<String>,
}

/// The run report returned to Job Orchestration.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResultDto {
    /// How many (user, period) pairs were due.
    pub due: usize,
    /// How many briefs were produced.
    pub generated: usize,
    /// How many briefs could not be produced.
    pub failed: usize,
    /// How many due windows already had a brief.
    pub skipped: usize,
    /// How many email alerts Notification accepted.
    pub email_submitted: usize,
    /// How many email alerts could not be delivered.
    pub email_failed: usize,
    /// How long the run took.
    pub duration_seconds: f64,
}

impl From<RunReport> for RunResultDto {
    fn from(report: RunReport) -> Self {
        Self {
            due: report.due,
            generated: report.generated,
            failed: report.failed,
            skipped: report.skipped,
            email_submitted: report.email_submitted,
            email_failed: report.email_failed,
            duration_seconds: report.duration_seconds,
        }
    }
}

/// `GET /v1/internal/privacy/export` query string.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportQuery {
    /// The user to export.
    pub user_id: String,
}

/// `POST /v1/internal/privacy/anonymise` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseRequest {
    /// The user to anonymise.
    pub user_id: String,
    /// The caller's idempotency key.
    pub idempotency_key: String,
}

/// The anonymisation response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseResponse {
    /// The random reference that replaced the subject.
    pub anonymised_ref: String,
    /// How many briefs changed in this call.
    pub briefs_changed: u64,
    /// When the anonymisation completed.
    pub completed_at: DateTime<Utc>,
    /// Whether the caller's key had already been applied.
    pub already_applied: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use reporting_domain::{BriefId, DataStatus, Period, UserId};

    fn generated() -> Brief {
        let start = DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant");
        let mut brief = Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Daily,
            start,
            start + Duration::hours(24),
            start,
        )
        .expect("a valid window");
        brief
            .mark_generated(start, DataStatus::Partial, start)
            .expect("a pending brief can be generated");
        brief
    }

    #[test]
    fn a_summary_carries_the_status_and_the_as_of() {
        let summary = BriefSummaryDto::from(&generated());
        assert_eq!(summary.status, "generated");
        assert_eq!(summary.data_status.as_deref(), Some("partial"));
        assert!(summary.as_of.is_some());
        assert_eq!(summary.failure_reason, None);
    }

    #[test]
    fn a_failed_summary_names_its_reason() {
        let start = DateTime::from_timestamp(1_800_000_000, 0).expect("a valid instant");
        let mut brief = Brief::new(
            BriefId::generate(),
            UserId::generate(),
            Period::Weekly,
            start,
            start + Duration::days(7),
            start,
        )
        .expect("a valid window");
        brief
            .mark_failed("upstream unavailable", start)
            .expect("the failure is recorded");
        let summary = BriefSummaryDto::from(&brief);
        assert_eq!(summary.status, "failed");
        assert_eq!(
            summary.failure_reason.as_deref(),
            Some("upstream unavailable")
        );
        assert_eq!(summary.data_status, None);
    }

    #[test]
    fn a_run_report_projects_every_count() {
        let dto = RunResultDto::from(RunReport {
            due: 3,
            generated: 2,
            failed: 1,
            skipped: 0,
            email_submitted: 2,
            email_failed: 0,
            duration_seconds: 1.5,
        });
        assert_eq!(dto.due, 3);
        assert_eq!(dto.generated, 2);
        assert_eq!(dto.failed, 1);
    }
}
