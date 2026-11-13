//! The C10 audit client. Every access, role, support, privacy, and export event
//! goes through this adapter; a failed write is surfaced to the use case, which
//! decides whether the action is refused.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::jurisdiction::AuditSink;
use identity_application::ports::{AuditClient, RepositoryError};
use identity_domain::{AuditOutcome, AuditRecord};
use serde_json::json;
use uuid::Uuid;

use crate::http_retry::is_transient;

/// Appends events to the Audit service over internal HTTP.
pub struct HttpAuditClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpAuditClient {
    /// Creates the client. The support-read budget is 200 ms end to end.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_millis(200))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }

    fn payload(record: &AuditRecord, correlation_id: &str) -> serde_json::Value {
        audit_payload(record, correlation_id)
    }
}

/// Builds the C9 event body from a domain record and the correlation id.
fn audit_payload(record: &AuditRecord, correlation_id: &str) -> serde_json::Value {
    let mut context = record.context().clone();
    context.insert("correlationId".to_string(), correlation_id.to_string());
    json!({
        "actorId": record.actor_id().map(|id| id.to_string()),
        "actorKind": if record.actor_id().is_some() { "user" } else { "system" },
        "action": record.action(),
        "target": record.target(),
        "occurredAt": record.occurred_at(),
        "context": context,
        "outcome": record.outcome().as_str(),
        "sensitivity": sensitivity(record.action()),
    })
}

fn sensitivity(action: &str) -> &'static str {
    let security = action.starts_with("identity.support")
        || action.starts_with("identity.deletion")
        || action.starts_with("identity.export")
        || action.starts_with("identity.principal")
        || action.starts_with("identity.role");
    if security {
        "security"
    } else {
        "routine"
    }
}

#[async_trait]
impl AuditClient for HttpAuditClient {
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        let url = format!("{}/v1/events", self.base_url.trim_end_matches('/'));
        let payload = Self::payload(record, correlation_id);
        // The support-read budget is 200 ms end to end, so this path makes one
        // attempt and refuses on exhaustion rather than retrying past the budget.
        match self
            .client
            .post(&url)
            .bearer_auth(&self.service_token)
            .header("idempotency-key", Uuid::new_v4().to_string())
            .json(&payload)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => Ok(()),
            Ok(response) => Err(RepositoryError::Unavailable(format!(
                "the audit service answered {}",
                response.status()
            ))),
            Err(error) => Err(RepositoryError::Unavailable(format!(
                "the audit service is unreachable: {error}"
            ))),
        }
    }
}

/// The outcome names the HTTP adapter accepts; exposed for a focused test.
#[must_use]
pub fn outcome_name(outcome: AuditOutcome) -> &'static str {
    outcome.as_str()
}

/// The jurisdiction gate's C9 writer.
///
/// The performance design gives the audit write 300 ms and one retry; the
/// idempotency key is the decision id, so the retry appends once (BR1.7, NFR2.4).
pub struct HttpDecisionAudit {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpDecisionAudit {
    /// Creates the adapter with the 300 ms per-attempt budget.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_millis(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into().trim_end_matches('/').to_string(),
            service_token: service_token.into(),
        }
    }
}

#[async_trait]
impl AuditSink for HttpDecisionAudit {
    async fn record(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
        idempotency_key: &str,
    ) -> Result<(), RepositoryError> {
        let url = format!("{}/v1/events", self.base_url);
        let payload = audit_payload(record, correlation_id);

        let mut last_error =
            RepositoryError::Unavailable("the audit service did not accept the event".to_string());
        for attempt in 0..=1 {
            let result = self
                .client
                .post(&url)
                .bearer_auth(&self.service_token)
                .header("idempotency-key", idempotency_key)
                .json(&payload)
                .send()
                .await;
            match result {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if is_transient(response.status().as_u16()) => {
                    last_error = RepositoryError::Unavailable(format!(
                        "the audit service answered {}",
                        response.status()
                    ));
                }
                Ok(response) => {
                    return Err(RepositoryError::Unavailable(format!(
                        "the audit service answered {}",
                        response.status()
                    )));
                }
                Err(error) => {
                    last_error = RepositoryError::Unavailable(format!(
                        "the audit service is unreachable: {error}"
                    ));
                }
            }
            if attempt == 0 {
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
        Err(last_error)
    }
}
