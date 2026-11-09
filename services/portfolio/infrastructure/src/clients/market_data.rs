//! The C6 prices and FX client.

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{ExternalError, MarketDataClient};
use portfolio_domain::{Freshness, FxRate, Quote};
use rust_decimal::Decimal;
use serde::Deserialize;

/// Reads prices and FX from the Market Data service.
pub struct HttpMarketDataClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpMarketDataClient {
    /// Creates the client with the C6 budget of a 150 ms request.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(150))
            .timeout(Duration::from_millis(150))
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
struct PriceDto {
    asset_id: String,
    price: Option<String>,
    currency: Option<String>,
    source: Option<String>,
    observed_at: Option<DateTime<Utc>>,
    freshness: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FxRateDto {
    base: String,
    quote: String,
    rate: String,
    observed_at: DateTime<Utc>,
    source: String,
    #[serde(default)]
    derived_from: Vec<String>,
}

#[async_trait]
impl MarketDataClient for HttpMarketDataClient {
    async fn prices(&self, asset_ids: &[String]) -> Result<Vec<Quote>, ExternalError> {
        if asset_ids.is_empty() {
            return Ok(Vec::new());
        }
        let url = format!("{}/v1/prices", self.base_url.trim_end_matches('/'));
        let joined = asset_ids.join(",");
        let response = self
            .client
            .get(url)
            .query(&[("assetIds", joined)])
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ExternalError::Unavailable(format!(
                "the market data service answered {}",
                response.status()
            )));
        }
        let rows = response
            .json::<Vec<PriceDto>>()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        let mut quotes = Vec::with_capacity(rows.len());
        for row in rows {
            let price = row
                .price
                .as_deref()
                .map(|price| Decimal::from_str(price.trim()))
                .transpose()
                .map_err(|_| {
                    ExternalError::Refused("C6 returned a non-decimal price".to_string())
                })?;
            let freshness = Freshness::parse(&row.freshness)
                .map_err(|error| ExternalError::Refused(error.to_string()))?;
            let quote = Quote::new(
                &row.asset_id,
                price,
                row.currency,
                row.source,
                row.observed_at,
                freshness,
            )
            .map_err(|error| ExternalError::Refused(error.to_string()))?;
            quotes.push(quote);
        }
        Ok(quotes)
    }

    async fn fx(&self, base: &str, quote: &str) -> Result<FxRate, ExternalError> {
        let url = format!("{}/v1/fx", self.base_url.trim_end_matches('/'));
        let response = self
            .client
            .get(url)
            .query(&[("base", base), ("quote", quote)])
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ExternalError::Unavailable(format!(
                "the market data service answered {}",
                response.status()
            )));
        }
        let row = response
            .json::<FxRateDto>()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        let rate = Decimal::from_str(row.rate.trim())
            .map_err(|_| ExternalError::Refused("C6 returned a non-decimal rate".to_string()))?;
        FxRate::new(
            &row.base,
            &row.quote,
            rate,
            row.observed_at,
            &row.source,
            row.derived_from,
        )
        .map_err(|error| ExternalError::Refused(error.to_string()))
    }
}
