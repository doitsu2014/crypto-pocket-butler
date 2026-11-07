//! The Supabase Auth admin adapter: the `has_exchange_credentials` claim (BR2.4).

use std::time::Duration;

use account_application::ports::{AuthAdmin, ExternalError};
use account_domain::UserId;
use async_trait::async_trait;
use serde_json::json;

use crate::http_retry::{is_transient, pause_before_retry};

/// Calls the Supabase Auth admin API with the service-role key.
pub struct HttpAuthAdmin {
    client: reqwest::Client,
    base_url: String,
    service_role_key: String,
}

impl HttpAuthAdmin {
    /// Creates the adapter.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_role_key: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_role_key: service_role_key.into(),
        }
    }
}

#[async_trait]
impl AuthAdmin for HttpAuthAdmin {
    async fn set_has_exchange_credentials(
        &self,
        user_id: UserId,
        value: bool,
    ) -> Result<(), ExternalError> {
        let url = format!(
            "{}/admin/users/{}",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let body = json!({ "app_metadata": { "has_exchange_credentials": value } });
        for attempt in 0..3u32 {
            let response = self
                .client
                .put(&url)
                .bearer_auth(&self.service_role_key)
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
                        "supabase auth refused the claim update with {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    tracing::warn!(%error, "supabase auth claim update failed; retrying");
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "supabase auth is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(
            "supabase auth did not answer the claim update".to_string(),
        ))
    }
}
