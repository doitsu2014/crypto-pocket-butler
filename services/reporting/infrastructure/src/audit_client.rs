//! The C10 audit client. Brief failures and privacy actions go through this
//! adapter with the bounded 1/2/5 s retry.

use std::time::Duration;

use async_trait::async_trait;
use reporting_application::ports::{AuditClient, RepositoryError};
use reporting_domain::AuditRecord;
use serde_json::json;
use uuid::Uuid;

use crate::http_retry::{is_transient, pause_before_retry};

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
    if action.starts_with("reporting.privacy") {
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
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.service_token)
                .header("idempotency-key", Uuid::new_v4().to_string())
                .json(&payload)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(RepositoryError::Unavailable(format!(
                        "the audit service answered {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    last_error = error.to_string();
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(RepositoryError::Unavailable(format!(
                        "the audit service is unreachable: {error}"
                    )));
                }
            }
        }
        Err(RepositoryError::Unavailable(format!(
            "the audit service did not answer: {last_error}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::sensitivity;

    #[test]
    fn privacy_actions_are_security_sensitive() {
        assert_eq!(sensitivity("reporting.privacy.export"), "security");
        assert_eq!(sensitivity("reporting.privacy.anonymise"), "security");
    }

    #[test]
    fn routine_actions_are_routine() {
        assert_eq!(sensitivity("reporting.brief.failed"), "routine");
    }
}
