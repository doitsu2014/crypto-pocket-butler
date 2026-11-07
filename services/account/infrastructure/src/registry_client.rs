//! The C4 Asset Registry client: supported chains and token resolution.

use std::time::Duration;

use account_application::ports::{AssetRegistry, ExternalError, Observation, SupportedChain};
use async_trait::async_trait;
use serde::Deserialize;

use crate::http_retry::{is_transient, pause_before_retry};

/// Reads the Asset Registry over internal HTTP.
pub struct HttpGetAssetRegistry {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

#[derive(Debug, Deserialize)]
struct ChainView {
    id: String,
    #[serde(rename = "nativeSymbol", alias = "native_symbol")]
    native_symbol: String,
}

#[derive(Debug, Deserialize)]
struct AssetView {
    id: String,
    #[serde(default = "default_true")]
    canonical: bool,
}

fn default_true() -> bool {
    true
}

impl HttpGetAssetRegistry {
    /// Creates the client with the C4 budget of 1 s connect / 10 s total.
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

    async fn get(&self, url: &str, operation: &str) -> Result<reqwest::Response, ExternalError> {
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            match self
                .client
                .get(url)
                .bearer_auth(&self.service_token)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) if response.status().as_u16() == 404 => return Ok(response),
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "the asset registry answered {} for {operation}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    last_error = error.to_string();
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "the asset registry is unreachable for {operation}: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "the asset registry did not answer {operation}: {last_error}"
        )))
    }
}

#[async_trait]
impl AssetRegistry for HttpGetAssetRegistry {
    async fn resolve(&self, observation: &Observation) -> Result<Option<String>, ExternalError> {
        let mut url = format!(
            "{}/v1/assets/resolve?chain={}",
            self.base_url.trim_end_matches('/'),
            urlencode(observation.chain.as_deref().unwrap_or("unknown"))
        );
        if let Some(contract) = &observation.contract_address {
            url.push_str(&format!("&contractAddress={}", urlencode(contract)));
        }
        if let Some(symbol) = &observation.symbol {
            url.push_str(&format!("&symbol={}", urlencode(symbol)));
        }
        let response = self.get(&url, "resolve").await?;
        if response.status().as_u16() == 404 {
            return Ok(None);
        }
        let asset = response.json::<AssetView>().await.map_err(|error| {
            ExternalError::Unavailable(format!("the asset shape is invalid: {error}"))
        })?;
        if asset.canonical {
            Ok(Some(asset.id))
        } else {
            Ok(None)
        }
    }

    async fn supported_chains(&self) -> Result<Vec<SupportedChain>, ExternalError> {
        let url = format!("{}/v1/chains", self.base_url.trim_end_matches('/'));
        let response = self.get(&url, "supported chains").await?;
        let chains = response.json::<Vec<ChainView>>().await.map_err(|error| {
            ExternalError::Unavailable(format!("the chain shape is invalid: {error}"))
        })?;
        Ok(chains
            .into_iter()
            .map(|chain| SupportedChain {
                id: chain.id,
                native_symbol: chain.native_symbol,
            })
            .collect())
    }
}

fn urlencode(value: &str) -> String {
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
