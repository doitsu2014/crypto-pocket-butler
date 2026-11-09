//! HTTP clients for C3 (verified contact at delivery time), the transactional
//! email provider, and C10 (audit).
//!
//! The verified address is read per delivery and never stored, logged, or
//! traced: none of these adapters record a request or response body.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_application::ports::{
    AuditClient, AuditRecord, ContactDirectory, EmailMessage, EmailSender, ExternalError,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The C3 contact budget on the delivery path: one attempt, 150 ms.
pub const CONTACT_TIMEOUT: Duration = Duration::from_millis(150);
/// The provider connect deadline.
pub const PROVIDER_CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// The provider per-call deadline.
pub const PROVIDER_TIMEOUT: Duration = Duration::from_secs(5);
/// The audit connect deadline.
pub const AUDIT_CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
/// The audit per-call deadline.
pub const AUDIT_TIMEOUT: Duration = Duration::from_secs(5);

/// Reads the verified contact from Identity's C3.
pub struct HttpContactDirectory {
    client: reqwest::Client,
    base_url: String,
    token: String,
}

impl HttpContactDirectory {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(CONTACT_TIMEOUT)
            .connect_timeout(CONTACT_TIMEOUT)
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
struct ContactBody {
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    verified: Option<bool>,
}

#[async_trait]
impl ContactDirectory for HttpContactDirectory {
    async fn verified_contact(&self, user_id: &str) -> Result<Option<String>, ExternalError> {
        let url = format!(
            "{}/v1/users/{}/contact",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C3 getContact: {error}")))?;
        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            // No contact record is a permanent address error (BR4.2).
            return Ok(None);
        }
        if !status.is_success() {
            if status.is_client_error() {
                return Err(ExternalError::Refused(format!(
                    "C3 getContact returned {status}"
                )));
            }
            return Err(ExternalError::Unavailable(format!(
                "C3 getContact returned {status}"
            )));
        }
        let body = response
            .json::<ContactBody>()
            .await
            .map_err(|error| ExternalError::Unavailable(format!("C3 contact body: {error}")))?;
        if body.verified == Some(false) {
            return Ok(None);
        }
        match body.email.map(|email| email.trim().to_string()) {
            Some(email) if !email.is_empty() => Ok(Some(email)),
            _ => Ok(None),
        }
    }
}

/// Sends through the transactional email provider over HTTPS.
pub struct HttpEmailSender {
    client: reqwest::Client,
    base_url: String,
    key: String,
    from: String,
}

impl HttpEmailSender {
    /// Creates the client.
    #[must_use]
    pub fn new(
        base_url: impl Into<String>,
        key: impl Into<String>,
        from: impl Into<String>,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(PROVIDER_CONNECT_TIMEOUT)
            .timeout(PROVIDER_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            key: key.into(),
            from: from.into(),
        }
    }
}

#[derive(Serialize)]
struct EmailBody<'a> {
    from: &'a str,
    to: &'a str,
    subject: &'a str,
    text: &'a str,
}

#[async_trait]
impl EmailSender for HttpEmailSender {
    async fn send(&self, message: &EmailMessage) -> Result<(), ExternalError> {
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let body = EmailBody {
            from: &self.from,
            to: &message.to,
            subject: &message.subject,
            text: &message.body,
        };
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.key)
            .json(&body)
            .send()
            .await
            // The error text is the transport's; it never contains the message.
            .map_err(|error| ExternalError::Unavailable(format!("email provider call: {error}")))?;
        let status = response.status();
        if status.is_success() {
            return Ok(());
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
            return Err(ExternalError::Unavailable(format!(
                "email provider returned {status}"
            )));
        }
        Err(ExternalError::Refused(format!(
            "email provider returned {status}"
        )))
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
