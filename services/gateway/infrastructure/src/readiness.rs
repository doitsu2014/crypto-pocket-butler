//! Readiness by checking the owning service's readiness endpoint.

use std::time::Duration;

use async_trait::async_trait;
use gateway_application::ports::ReadinessProbe;

/// Checks the Asset Registry's `/readyz` endpoint.
pub struct HttpReadinessProbe {
    client: reqwest::Client,
    url: String,
}

impl HttpReadinessProbe {
    /// Creates the probe for the registry readiness URL.
    #[must_use]
    pub fn new(url: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(1))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            url: url.into(),
        }
    }
}

#[async_trait]
impl ReadinessProbe for HttpReadinessProbe {
    async fn check(&self) -> Result<(), String> {
        self.client
            .get(&self.url)
            .send()
            .await
            .map_err(|error| error.to_string())?
            .error_for_status()
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}
