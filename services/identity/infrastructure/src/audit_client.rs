//! The C10 audit client. Every access, role, support, privacy, and export event
//! goes through this adapter; a failed write is surfaced to the use case, which
//! decides whether the action is refused.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::ports::{AuditClient, RepositoryError};
use identity_domain::{AuditOutcome, AuditRecord};
use serde_json::json;
use uuid::Uuid;

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
