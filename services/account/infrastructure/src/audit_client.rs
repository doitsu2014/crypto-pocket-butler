//! The C10 audit client. Credential, account, sync, and import events go through
//! this adapter; a failed write is surfaced to the use case.

use std::time::Duration;

use account_application::ports::{AuditClient, RepositoryError};
use account_domain::{AuditOutcome, AuditRecord};
use async_trait::async_trait;
use serde_json::json;
use uuid::Uuid;

/// Appends events to the Audit service over internal HTTP.
pub struct HttpAuditClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpAuditClient {
    /// Creates the client with the C10 budget of 1 s connect / 5 s total.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(5))
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
    let security = action.starts_with("account.credential")
        || action.starts_with("account.privacy")
        || action.starts_with("account.disconnected");
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

/// The outcome names the adapter accepts, exposed for a focused test.
#[must_use]
pub fn outcome_name(outcome: AuditOutcome) -> &'static str {
    outcome.as_str()
}
