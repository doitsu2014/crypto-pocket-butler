//! The C7 Ledger client used to record a confirmed trade.

use std::time::Duration;

use async_trait::async_trait;
use portfolio_application::ports::{ExternalError, LedgerClient, LedgerTradeCommand};
use portfolio_domain::{PriceSource, UserId};
use serde::Deserialize;
use serde_json::json;

/// Records trades on the Ledger service.
pub struct HttpLedgerClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpLedgerClient {
    /// Creates the client with the C7 budget of a 300 ms request.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(300))
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
struct TradeResponse {
    transaction: TransactionDto,
}

#[derive(Debug, Deserialize)]
struct TransactionDto {
    id: String,
}

#[async_trait]
impl LedgerClient for HttpLedgerClient {
    async fn record_trade(
        &self,
        user_id: UserId,
        command: &LedgerTradeCommand,
        idempotency_key: &str,
    ) -> Result<String, ExternalError> {
        let url = format!(
            "{}/v1/users/{}/trades",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let body = json!({
            "assetId": command.asset_id,
            "kind": command.side.as_str(),
            "quantity": command.quantity.to_string(),
            "price": command.price.to_string(),
            "priceSource": command.price_source.as_str(),
            "currency": command.currency,
            "executedAt": command.executed_at,
        });
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.service_token)
            .header("idempotency-key", idempotency_key)
            .json(&body)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ExternalError::Refused(format!(
                "the ledger service answered {}",
                response.status()
            )));
        }
        let parsed = response
            .json::<TradeResponse>()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        Ok(parsed.transaction.id)
    }
}

/// The price source label the ledger accepts, exposed for a focused test.
#[must_use]
pub fn price_source_label(source: PriceSource) -> &'static str {
    source.as_str()
}
