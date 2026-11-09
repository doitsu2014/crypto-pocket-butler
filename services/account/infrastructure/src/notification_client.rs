//! The C9 notification client. The alert type and idempotency key are supplied by
//! the use case; this adapter retries a transient failure within the bound
//! (BR7.1).

use std::time::Duration;

use account_application::ports::{ExternalError, NotificationClient};
use account_domain::UserId;
use async_trait::async_trait;
use serde_json::{json, Value};

use crate::http_retry::{is_transient, pause_before_retry};

/// Sends alerts to the Notification service over internal HTTP.
pub struct HttpNotificationClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpNotificationClient {
    /// Creates the client with the C9 budget of 1 s connect / 10 s total.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(10))
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
impl NotificationClient for HttpNotificationClient {
    async fn notify(
        &self,
        user_id: UserId,
        alert_type: &str,
        idempotency_key: &str,
        payload: Value,
    ) -> Result<(), ExternalError> {
        let url = format!("{}/v1/alerts", self.base_url.trim_end_matches('/'));
        let body = json!({
            "userId": user_id.to_string(),
            "type": alert_type,
            "idempotencyKey": idempotency_key,
            "payload": payload,
        });
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&self.service_token)
                .header("idempotency-key", idempotency_key)
                .json(&body)
                .send()
                .await;
            match response {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "the notification service answered {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    last_error = error.to_string();
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "the notification service is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "the notification service did not answer: {last_error}"
        )))
    }
}
