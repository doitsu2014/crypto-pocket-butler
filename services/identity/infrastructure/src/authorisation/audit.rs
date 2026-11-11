//! The C9 event write for denials and role changes, with the field-set
//! redaction backstop (BR2.11, BR2.12).
//!
//! One attempt, 250 ms ceiling: a denial is not returned until this write is
//! accepted, so a retry would eat the C4 deadline. A value that matches a
//! token, wallet-address, or secret-name shape refuses the write instead of
//! sending it.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::authorisation::ports::{AuditSink, AuthorisationEvent};
use identity_application::ports::RepositoryError;
use serde_json::json;
use uuid::Uuid;

/// The bounded C9 writer.
pub struct AuthorisationAuditSink {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl AuthorisationAuditSink {
    /// Creates the sink. The 250 ms ceiling is the governing value from the
    /// security and reliability designs.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_millis(250))
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
impl AuditSink for AuthorisationAuditSink {
    async fn record(
        &self,
        event: &AuthorisationEvent,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        // Redaction is a backstop: the field set already has no room for a
        // credential. A matching value refuses the write.
        if let Some(field) = forbidden_field(event) {
            return Err(RepositoryError::Validation(format!(
                "the audit {field} field would carry a secret"
            )));
        }

        let mut context = event.context().clone();
        context.insert("correlationId".to_string(), correlation_id.to_string());
        let payload = json!({
            "actor": event.actor(),
            "action": event.action(),
            "target": event.target(),
            "occurredAt": event.occurred_at(),
            "context": context,
            "outcome": event.outcome().as_str(),
        });
        let url = format!("{}/v1/events", self.base_url);

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

/// The first field whose value would carry a secret, if any.
fn forbidden_field(event: &AuthorisationEvent) -> Option<&'static str> {
    for (field, value) in [
        ("actor", event.actor()),
        ("action", event.action()),
        ("target", event.target()),
    ] {
        if looks_like_secret(value) {
            return Some(field);
        }
    }
    for (key, value) in event.context() {
        if looks_like_secret_name(key) || looks_like_secret(value) {
            return Some("context");
        }
    }
    None
}

/// A value shaped like a credential: a JWT (`eyJ` prefix) or a wallet address
/// (`0x` + 40 hex).
fn looks_like_secret(value: &str) -> bool {
    let value = value.trim();
    if value.starts_with("eyJ") {
        return true;
    }
    if let Some(hex) = value.strip_prefix("0x") {
        if hex.len() == 40 && hex.chars().all(|character| character.is_ascii_hexdigit()) {
            return true;
        }
    }
    false
}

/// A context key that names a secret. The fixed field set never has one; this
/// catches a future field that adds one.
fn looks_like_secret_name(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    ["token", "secret", "password", "api_key", "apikey"]
        .iter()
        .any(|needle| lower.contains(needle))
}
