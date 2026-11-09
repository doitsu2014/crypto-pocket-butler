//! The Identity base-currency preference read.
//!
//! Identity currently exposes the preference write and the SPA-facing read; an
//! internal read for services is not yet on C3. The adapter therefore treats a
//! non-success answer as "no preference" and the caller falls back to USD.

use std::time::Duration;

use async_trait::async_trait;
use portfolio_application::ports::{ExternalError, UserPreferenceClient};
use portfolio_domain::UserId;
use serde::Deserialize;

/// Reads the user's base currency from Identity.
pub struct HttpIdentityPreferences {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpIdentityPreferences {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(200))
            .timeout(Duration::from_millis(300))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PreferencesDto {
    base_currency: Option<String>,
}

#[async_trait]
impl UserPreferenceClient for HttpIdentityPreferences {
    async fn base_currency(&self, user_id: UserId) -> Result<Option<String>, ExternalError> {
        let url = format!(
            "{}/v1/users/{}/preferences",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.service_token)
            .send()
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                tracing::warn!(%error, "the Identity preference read failed; using the default currency");
                return Ok(None);
            }
        };
        if !response.status().is_success() {
            tracing::warn!(
                status = %response.status(),
                "the Identity preference read was refused; using the default currency"
            );
            return Ok(None);
        }
        match response.json::<PreferencesDto>().await {
            Ok(preferences) => Ok(preferences.base_currency),
            Err(error) => {
                tracing::warn!(%error, "the Identity preference body was malformed; using the default currency");
                Ok(None)
            }
        }
    }
}
