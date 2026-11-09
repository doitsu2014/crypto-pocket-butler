//! Request and response DTOs. They live only at the HTTP boundary and map to and
//! from application commands here; no DTO type reaches application or domain.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use notification_application::{AlertOutcome, FeedPage, PreferenceChange, SubmitCommand};
use notification_domain::{Alert, NotificationType, ResolvedPreference};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `POST /v1/alerts` body (C9).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRequest {
    /// The receiving user.
    pub user_id: String,
    /// The notification type.
    pub r#type: String,
    /// The caller's dedupe key.
    pub idempotency_key: String,
    /// The caller's payload; only allow-listed scalars survive.
    #[serde(default)]
    pub payload: BTreeMap<String, Value>,
    /// The channel the emitter asked for: `in_app`, `email`, or `both` (C9).
    #[serde(default)]
    pub channel: Option<String>,
}

impl AlertRequest {
    /// Maps the body plus the request correlation id to an application command.
    #[must_use]
    pub fn into_command(self, correlation_id: Option<String>) -> SubmitCommand {
        SubmitCommand {
            user_id: self.user_id,
            kind: self.r#type,
            idempotency_key: self.idempotency_key,
            payload: self
                .payload
                .into_iter()
                .filter_map(|(key, value)| scalar(&value).map(|value| (key, value)))
                .collect(),
            correlation_id,
            channel: self.channel,
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

/// `POST /v1/alerts` response (C9).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertAccepted {
    /// The stored or existing alert.
    pub alert_id: uuid::Uuid,
    /// `queued` or `deduplicated`.
    pub status: &'static str,
}

/// `GET /v1/users/{userId}/notifications` query string.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedQuery {
    /// The `nextCursor` from the previous page.
    #[serde(default)]
    pub cursor: Option<String>,
    /// The page size.
    #[serde(default)]
    pub limit: Option<u32>,
}

/// One alert in the feed.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedAlertDto {
    /// The alert id.
    pub id: uuid::Uuid,
    /// The type label.
    pub r#type: String,
    /// The allow-listed, sanitised payload.
    pub payload: BTreeMap<String, String>,
    /// The status label.
    pub status: String,
    /// Field names the content guard dropped.
    pub sanitised_fields: Vec<String>,
    /// When the alert was stored.
    pub created_at: DateTime<Utc>,
}

impl From<&Alert> for FeedAlertDto {
    fn from(alert: &Alert) -> Self {
        Self {
            id: alert.id().as_uuid(),
            r#type: alert.kind().as_str().to_string(),
            payload: alert.payload().clone(),
            status: alert.status().as_str().to_string(),
            sanitised_fields: alert.sanitised_fields().to_vec(),
            created_at: alert.created_at(),
        }
    }
}

/// `GET /v1/users/{userId}/notifications` response.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedResponse {
    /// The alerts, newest first.
    pub alerts: Vec<FeedAlertDto>,
    /// Present when another page may exist.
    pub next_cursor: Option<String>,
}

impl From<FeedPage> for FeedResponse {
    fn from(page: FeedPage) -> Self {
        Self {
            alerts: page.alerts.iter().map(FeedAlertDto::from).collect(),
            next_cursor: page.next_cursor,
        }
    }
}

/// One type's two switches, as the settings screen sends and receives them.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferenceRow {
    /// The type label.
    pub r#type: String,
    /// The in-app switch.
    pub in_app: bool,
    /// The email switch.
    pub email: bool,
    /// Whether email ignores the switch (BR3.1); response only.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mandatory_email: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

/// `PUT /v1/users/{userId}/notifications/preferences` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesRequest {
    /// One row per type.
    pub preferences: Vec<PreferenceRow>,
}

impl PreferencesRequest {
    /// Expands each row into one change per channel.
    #[must_use]
    pub fn into_changes(self) -> Vec<PreferenceChange> {
        let mut changes = Vec::with_capacity(self.preferences.len() * 2);
        for row in self.preferences {
            changes.push(PreferenceChange {
                kind: row.r#type.clone(),
                channel: "in_app".to_string(),
                enabled: row.in_app,
            });
            changes.push(PreferenceChange {
                kind: row.r#type,
                channel: "email".to_string(),
                enabled: row.email,
            });
        }
        changes
    }
}

/// A preferences response.
#[derive(Debug, Clone, Serialize)]
pub struct PreferencesResponse {
    /// Every type, with both switches.
    pub preferences: Vec<PreferenceRow>,
}

impl From<Vec<ResolvedPreference>> for PreferencesResponse {
    fn from(resolved: Vec<ResolvedPreference>) -> Self {
        let mut rows: Vec<PreferenceRow> = Vec::with_capacity(NotificationType::ALL.len());
        for entry in resolved {
            let index = match rows
                .iter()
                .position(|row| row.r#type == entry.kind.as_str())
            {
                Some(index) => index,
                None => {
                    rows.push(PreferenceRow {
                        r#type: entry.kind.as_str().to_string(),
                        in_app: false,
                        email: false,
                        mandatory_email: false,
                    });
                    rows.len() - 1
                }
            };
            let row = &mut rows[index];
            match entry.channel {
                notification_domain::Channel::InApp => row.in_app = entry.enabled,
                notification_domain::Channel::Email => {
                    row.email = entry.enabled;
                    row.mandatory_email = entry.mandatory;
                }
            }
        }
        Self { preferences: rows }
    }
}

/// `POST /v1/internal/privacy/anonymise` body (C9).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymiseRequest {
    /// The user Identity is erasing.
    pub user_id: String,
    /// The caller's idempotency key.
    pub idempotency_key: String,
}

/// A C11 run result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunResponse {
    /// The job name.
    pub job_name: String,
    /// `succeeded`, `failed`, or `skipped`.
    pub status: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// The lease owner, when a run produced one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    /// A human-readable summary carrying counts only.
    pub detail: String,
}

impl RunResponse {
    /// Builds a succeeded result.
    #[must_use]
    pub fn succeeded(
        job_name: &str,
        started_at: DateTime<Utc>,
        run_id: Option<String>,
        detail: String,
    ) -> Self {
        Self {
            job_name: job_name.to_string(),
            status: "succeeded".to_string(),
            started_at,
            run_id,
            detail,
        }
    }
}

/// The delivery run's detail line.
#[must_use]
pub fn delivery_detail(outcomes: &[AlertOutcome]) -> String {
    let delivered = outcomes.iter().filter(|o| o.status == "delivered").count();
    let partial = outcomes
        .iter()
        .filter(|o| o.status == "partially_delivered")
        .count();
    let failed = outcomes.iter().filter(|o| o.status == "failed").count();
    let queued = outcomes.iter().filter(|o| o.status == "queued").count();
    format!(
        "{delivered} delivered, {partial} partially delivered, {failed} failed, {queued} queued"
    )
}
