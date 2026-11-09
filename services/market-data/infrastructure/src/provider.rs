//! The provider HTTP client.
//!
//! Each configured source is reached at its base URL. The client performs one
//! bounded call (1 s connect / 2 s read / 5 s total) and classifies a failure so
//! the application can decide whether to retry (BR4.2). A provider key is sent
//! only as a bearer header and is never logged.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_data_application::ports::{PriceProvider, PriceSource, ProviderError, ProviderQuote};
use market_data_domain::ErrorCategory;
use serde::Deserialize;

use crate::registry_client::urlencode;

/// The connect budget of one provider call.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
/// The total budget of one provider call.
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

/// Calls provider REST bases over HTTPS.
pub struct HttpPriceProvider {
    client: reqwest::Client,
    bases: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ReadingView {
    price: String,
    #[serde(default = "default_currency")]
    currency: String,
    #[serde(rename = "observedAt", alias = "observed_at")]
    observed_at: DateTime<Utc>,
}

fn default_currency() -> String {
    "USD".to_string()
}

impl HttpPriceProvider {
    /// Creates the client over `name → base URL` pairs.
    #[must_use]
    pub fn new(bases: BTreeMap<String, String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(TOTAL_TIMEOUT)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { client, bases }
    }

    fn base(&self, source: &PriceSource) -> Result<&str, ProviderError> {
        self.bases
            .get(&source.name)
            .map(String::as_str)
            .ok_or_else(|| {
                ProviderError::permanent(
                    ErrorCategory::Data,
                    format!("source '{}' has no configured base URL", source.name),
                )
            })
    }

    async fn get(
        &self,
        source: &PriceSource,
        url: String,
        api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError> {
        let mut request = self.client.get(url);
        if let Some(key) = api_key {
            request = request.bearer_auth(key);
        }
        let response = request.send().await.map_err(|error| {
            if error.is_timeout() || error.is_connect() {
                ProviderError::transient(ErrorCategory::Network, "the provider timed out")
            } else {
                ProviderError::transient(ErrorCategory::Network, "the provider is unreachable")
            }
        })?;
        let status = response.status();
        if status.is_success() {
            let view = response.json::<ReadingView>().await.map_err(|_| {
                ProviderError::permanent(
                    ErrorCategory::Data,
                    format!("source '{}' returned a malformed payload", source.name),
                )
            })?;
            return Ok(ProviderQuote {
                price: view.price,
                currency: view.currency,
                observed_at: view.observed_at,
            });
        }
        if status.as_u16() == 429 {
            return Err(ProviderError::transient(
                ErrorCategory::RateLimit,
                format!("source '{}' rate-limited the call", source.name),
            ));
        }
        if status.is_server_error() {
            return Err(ProviderError::transient(
                ErrorCategory::Provider,
                format!("source '{}' answered {}", source.name, status.as_u16()),
            ));
        }
        Err(ProviderError::permanent(
            ErrorCategory::Provider,
            format!("source '{}' answered {}", source.name, status.as_u16()),
        ))
    }
}

#[async_trait]
impl PriceProvider for HttpPriceProvider {
    async fn quote(
        &self,
        source: &PriceSource,
        asset_id: &str,
        api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError> {
        let base = self.base(source)?;
        let url = format!(
            "{}/price?asset={}&quote={}",
            base.trim_end_matches('/'),
            urlencode(asset_id),
            urlencode(&source.quote_currency)
        );
        self.get(source, url, api_key).await
    }

    async fn fx_rate(
        &self,
        source: &PriceSource,
        base: &str,
        quote: &str,
        api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError> {
        let url = format!(
            "{}/fx?base={}&quote={}",
            self.base(source)?.trim_end_matches('/'),
            urlencode(base),
            urlencode(quote)
        );
        self.get(source, url, api_key).await
    }
}
