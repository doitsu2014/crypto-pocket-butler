//! The worker wake transport. A failure degrades to the periodic supervisor
//! tick.

use std::time::Duration;

use async_trait::async_trait;
use portfolio_application::ports::WakeTransport;

/// Posts to the worker's internal wake endpoint.
pub struct HttpWake {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpWake {
    /// Creates the client.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(500))
            .timeout(Duration::from_secs(2))
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
impl WakeTransport for HttpWake {
    async fn wake(&self) -> Result<(), String> {
        let url = format!("{}/internal/wake", self.base_url.trim_end_matches('/'));
        self.client
            .post(url)
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map(|response| {
                if response.status().is_success() {
                    Ok(())
                } else {
                    Err(format!("the worker answered {}", response.status()))
                }
            })
            .map_err(|error| error.to_string())?
    }
}
