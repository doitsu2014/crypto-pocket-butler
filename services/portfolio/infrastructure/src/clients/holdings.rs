//! The C5 holdings and venue-transaction client.

use std::str::FromStr;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{
    ExternalError, HoldingObservation, HoldingsClient, VenueTradeObservation,
};
use portfolio_domain::{AccountId, OrderSide, UserId};
use rust_decimal::Decimal;
use serde::Deserialize;

/// Reads holdings and transactions from the Account service.
pub struct HttpHoldingsClient {
    client: reqwest::Client,
    base_url: String,
    service_token: String,
}

impl HttpHoldingsClient {
    /// Creates the client with the C5 budget of a 200 ms request.
    #[must_use]
    pub fn new(base_url: impl Into<String>, service_token: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_millis(200))
            .timeout(Duration::from_millis(200))
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
struct HoldingDto {
    account_id: String,
    asset_id: Option<String>,
    quantity: String,
    observed_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransactionDto {
    asset_id: String,
    kind: String,
    quantity: String,
    executed_at: DateTime<Utc>,
    external_id: Option<String>,
}

#[async_trait]
impl HoldingsClient for HttpHoldingsClient {
    async fn holdings_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<HoldingObservation>, ExternalError> {
        let url = format!(
            "{}/v1/users/{}/holdings",
            self.base_url.trim_end_matches('/'),
            user_id
        );
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.service_token)
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ExternalError::Unavailable(format!(
                "the account service answered {}",
                response.status()
            )));
        }
        let rows = response
            .json::<Vec<HoldingDto>>()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        let mut holdings = Vec::with_capacity(rows.len());
        for row in rows {
            let Some(asset_id) = row.asset_id else {
                // A spam holding has no asset id and cannot be valued.
                continue;
            };
            let account_id = AccountId::parse(&row.account_id)
                .map_err(|error| ExternalError::Refused(error.to_string()))?;
            holdings.push(HoldingObservation {
                account_id,
                asset_id,
                quantity: parse(&row.quantity, "quantity")?,
                observed_at: row.observed_at,
            });
        }
        Ok(holdings)
    }

    async fn transactions_for_account(
        &self,
        account_id: AccountId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<VenueTradeObservation>, ExternalError> {
        let url = format!(
            "{}/v1/accounts/{}/transactions",
            self.base_url.trim_end_matches('/'),
            account_id
        );
        let mut request = self.client.get(url).bearer_auth(&self.service_token);
        if let Some(since) = since {
            request = request.query(&[("since", since.to_rfc3339())]);
        }
        let response = request
            .send()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        if !response.status().is_success() {
            return Err(ExternalError::Unavailable(format!(
                "the account service answered {}",
                response.status()
            )));
        }
        let rows = response
            .json::<Vec<TransactionDto>>()
            .await
            .map_err(|error| ExternalError::Unavailable(error.to_string()))?;
        let mut trades = Vec::new();
        for row in rows {
            let side = match row.kind.as_str() {
                "buy" => OrderSide::Buy,
                "sell" => OrderSide::Sell,
                _ => continue,
            };
            trades.push(VenueTradeObservation {
                external_id: row.external_id,
                asset_id: row.asset_id,
                side,
                quantity: parse(&row.quantity, "quantity")?,
                executed_at: row.executed_at,
            });
        }
        Ok(trades)
    }
}

fn parse(raw: &str, field: &str) -> Result<Decimal, ExternalError> {
    Decimal::from_str(raw.trim())
        .map_err(|_| ExternalError::Refused(format!("C5 returned a non-decimal {field}")))
}
