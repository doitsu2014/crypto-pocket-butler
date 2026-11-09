//! The C10 Audit client. A failed append is returned to the caller so the use
//! case can log it and increment the audit-failure metric; it is never dropped.

use std::time::Duration;

use async_trait::async_trait;
use market_data_application::ports::{AuditClient, RepositoryError};
use market_data_domain::AuditRecord;
use serde_json::json;

/// Appends events to the Audit service.
pub struct HttpAuditClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpAuditClient {
    /// Creates the client with the C10 budget.
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
}

#[async_trait]
impl AuditClient for HttpAuditClient {
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        let mut context = record.context().clone();
        if let Some(actor) = record.actor_service() {
            context.insert("actorService".to_string(), actor.to_string());
        }
        let body = json!({
            "actorKind": record.actor_kind(),
            "action": record.action(),
            "target": record.target(),
            "occurredAt": record.occurred_at(),
            "context": context,
            "outcome": record.outcome().as_str(),
            "sensitivity": "security",
        });
        let url = format!("{}/v1/events", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.service_token)
            .header("idempotency-key", format!("market-data:{correlation_id}"))
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                RepositoryError::Unavailable(format!("the audit service is unreachable: {error}"))
            })?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(RepositoryError::Unavailable(format!(
                "the audit service answered {}",
                response.status().as_u16()
            )))
        }
    }
}
