//! The C4 Asset Registry client.
//!
//! The read path uses a 150 ms budget with no retry (NFR1.11): a slow C4 must
//! degrade the response immediately, not hold it open. A refresh run uses the
//! run budget with bounded retries and a failed call fails the run.

use std::time::Duration;

use async_trait::async_trait;
use market_data_application::ports::{AssetRegistry, CanonicalAsset, ExternalError, PriceSource};
use serde::Deserialize;

/// How long the read path waits for C4.
pub const READ_PATH_TIMEOUT: Duration = Duration::from_millis(150);
/// The total budget of one run-path call.
pub const RUN_PATH_TIMEOUT: Duration = Duration::from_secs(10);
/// The run path's bounded attempt count.
pub const RUN_PATH_ATTEMPTS: u32 = 3;

/// Reads the Asset Registry over internal HTTP.
pub struct HttpAssetRegistry {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

#[derive(Debug, Deserialize)]
struct SourceView {
    name: String,
    priority: u32,
    #[serde(default = "default_currency")]
    #[serde(rename = "quoteCurrency", alias = "quote_currency")]
    quote_currency: String,
    #[serde(default = "default_rate_limit")]
    #[serde(rename = "rateLimitPerMinute", alias = "rate_limit_per_minute")]
    rate_limit_per_minute: u32,
    #[serde(default)]
    #[serde(rename = "delayedFeed", alias = "delayed_feed")]
    delayed_feed: bool,
    #[serde(default = "default_true")]
    enabled: bool,
}

#[derive(Debug, Deserialize)]
struct AssetView {
    id: String,
    #[serde(default)]
    rank: u32,
    #[serde(default = "default_true")]
    canonical: bool,
}

fn default_currency() -> String {
    // C4's contract requires quoteCurrency. The default keeps Market Data
    // interoperable with a registry that has not yet published the field.
    "USD".to_string()
}

fn default_rate_limit() -> u32 {
    60
}

fn default_true() -> bool {
    true
}

impl HttpAssetRegistry {
    /// Creates the client with the C4 read/run budgets.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(RUN_PATH_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            base_url: base_url.into(),
            service_token: service_token.into(),
        }
    }

    async fn read(&self, url: &str, operation: &str) -> Result<reqwest::Response, ExternalError> {
        match tokio::time::timeout(
            READ_PATH_TIMEOUT,
            self.client.get(url).bearer_auth(&self.service_token).send(),
        )
        .await
        {
            Ok(Ok(response)) if response.status().is_success() => Ok(response),
            Ok(Ok(response)) => Err(ExternalError::Refused(format!(
                "the asset registry answered {} for {operation}",
                response.status()
            ))),
            Ok(Err(error)) => Err(ExternalError::Unavailable(format!(
                "the asset registry is unreachable for {operation}: {error}"
            ))),
            Err(_) => Err(ExternalError::Unavailable(format!(
                "the asset registry did not answer {operation} within 150 ms"
            ))),
        }
    }

    async fn run(&self, url: &str, operation: &str) -> Result<reqwest::Response, ExternalError> {
        let mut last = String::new();
        for attempt in 0..RUN_PATH_ATTEMPTS {
            match self
                .client
                .get(url)
                .bearer_auth(&self.service_token)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) if response.status().as_u16() == 404 => return Ok(response),
                Ok(response) if attempt + 1 < RUN_PATH_ATTEMPTS => {
                    last = format!("the asset registry answered {}", response.status());
                    tokio::time::sleep(retry_pause(attempt)).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "the asset registry answered {} for {operation}",
                        response.status()
                    )));
                }
                Err(error) if attempt + 1 < RUN_PATH_ATTEMPTS => {
                    last = error.to_string();
                    tokio::time::sleep(retry_pause(attempt)).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "the asset registry is unreachable for {operation}: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "the asset registry did not answer {operation}: {last}"
        )))
    }
}

fn retry_pause(attempt: u32) -> Duration {
    Duration::from_millis(100 * u64::from(1_u32 << attempt.min(4)))
}

#[async_trait]
impl AssetRegistry for HttpAssetRegistry {
    async fn sources(&self) -> Result<Vec<PriceSource>, ExternalError> {
        let url = format!("{}/v1/price-sources", self.base_url.trim_end_matches('/'));
        let response = self.read(&url, "price sources").await?;
        let views = response.json::<Vec<SourceView>>().await.map_err(|error| {
            ExternalError::Refused(format!("the price-source shape is invalid: {error}"))
        })?;
        views
            .into_iter()
            .map(|view| {
                PriceSource::new(
                    &view.name,
                    view.priority,
                    &view.quote_currency,
                    view.rate_limit_per_minute,
                    view.delayed_feed,
                    view.enabled,
                )
                .map_err(ExternalError::Refused)
            })
            .collect()
    }

    async fn canonical_assets(&self) -> Result<Vec<CanonicalAsset>, ExternalError> {
        let url = format!(
            "{}/v1/assets?limit=10000",
            self.base_url.trim_end_matches('/')
        );
        let response = self.run(&url, "canonical assets").await?;
        let views = response.json::<Vec<AssetView>>().await.map_err(|error| {
            ExternalError::Refused(format!("the asset shape is invalid: {error}"))
        })?;
        Ok(views
            .into_iter()
            .filter(|view| view.canonical && view.rank >= 1)
            .map(|view| CanonicalAsset {
                id: view.id,
                rank: view.rank,
            })
            .collect())
    }
}

/// Percent-encodes one query value.
#[must_use]
pub fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
