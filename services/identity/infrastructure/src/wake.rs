//! The worker wake transport. A wake failure degrades to the 60-second sweep, so
//! this adapter never retries.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::ports::WakeTransport;

/// POSTs the wake call to the worker's internal listener.
pub struct HttpWake {
    client: reqwest::Client,
    url: String,
    service_token: String,
}

impl HttpWake {
    /// Creates the transport.
    #[must_use]
    pub fn new(worker_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            url: format!("{}/internal/wake", worker_url.into().trim_end_matches('/')),
            service_token: service_token.into(),
        }
    }
}

#[async_trait]
impl WakeTransport for HttpWake {
    async fn wake(&self) -> Result<(), String> {
        let response = self
            .client
            .post(&self.url)
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| error.to_string())?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!("the worker answered {}", response.status()))
        }
    }
}
