//! HTTP clients for C3 consent and C10 audit.
//!
//! Consent is one attempt with an 80 ms total timeout; a failure is surfaced so
//! ingest fails closed with 503. Audit uses a 1 s connect and 5 s total timeout;
//! its failure is logged by the caller and never blocks ingest.

use std::collections::BTreeMap;
use std::time::Duration;

use analytics_application::ports::{AuditClient, AuditRecord, ConsentChecker, ExternalError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The production consent budget.
pub const CONSENT_TIMEOUT: Duration = Duration::from_millis(80);
/// The production audit totals.
pub const AUDIT_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
pub const AUDIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Checks C3 `getConsent`.
pub struct HttpConsentChecker {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpConsentChecker {
    /// Creates the checker.
    ///
    /// A timeout before the client is built falls back to a default client; the
    /// per-request timeout below still applies.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(CONSENT_TIMEOUT)
            .connect_timeout(CONSENT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            token: token.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConsentBody {
    analytics: bool,
}

#[async_trait]
impl ConsentChecker for HttpConsentChecker {
    async fn analytics_granted(&self, user_id: &str) -> Result<bool, ExternalError> {
        let url = format!(
            "{}/v1/consent/{}",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C3 getConsent: {error}")))?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            // No record is no grant (BR1.3).
            return Ok(false);
        }
        if !status.is_success() {
            if status.is_client_error() {
                return Err(ExternalError::Refused(format!(
                    "C3 getConsent returned {status}"
                )));
            }
            return Err(ExternalError::Unavailable(format!(
                "C3 getConsent returned {status}"
            )));
        }
        let body = response
            .json::<ConsentBody>()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C3 consent body: {error}")))?;
        Ok(body.analytics)
    }
}

/// Appends C10 audit records.
pub struct HttpAuditClient {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpAuditClient {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(AUDIT_CONNECT_TIMEOUT)
            .timeout(AUDIT_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            token: token.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditBody<'a> {
    actor_id: Option<&'a str>,
    actor_kind: &'a str,
    action: &'a str,
    target: &'a str,
    occurred_at: DateTime<Utc>,
    context: BTreeMap<String, String>,
    outcome: &'a str,
    sensitivity: &'a str,
}

#[async_trait]
impl AuditClient for HttpAuditClient {
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError> {
        let url = format!("{}/v1/events", self.base_url.trim_end_matches('/'));
        let body = AuditBody {
            actor_id: record.actor_id.as_deref(),
            actor_kind: &record.actor_kind,
            action: &record.action,
            target: &record.target,
            occurred_at: record.occurred_at,
            context: record.context.clone(),
            outcome: &record.outcome,
            sensitivity: &record.sensitivity,
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .header("Idempotency-Key", Uuid::new_v4().to_string())
            .json(&body)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C10 append: {error}")))?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        if status.is_client_error() {
            return Err(ExternalError::Refused(format!(
                "C10 append returned {status}"
            )));
        }
        Err(ExternalError::Unavailable(format!(
            "C10 append returned {status}"
        )))
    }
}
