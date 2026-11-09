//! The wallet chain client: native balances over JSON-RPC for EVM chains and
//! Solana. A slow or failed chain fails its own run only (reliability design).

use std::time::Duration;

use account_application::ports::{ChainProvider, ExternalError, ObservedBalance};
use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::json;

use crate::http_retry::{is_transient, pause_before_retry};

/// Reads native balances over JSON-RPC.
pub struct HttpChainProvider {
    client: reqwest::Client,
    evm_rpc: String,
    solana_rpc: String,
}

impl HttpChainProvider {
    /// Creates the provider.
    #[must_use]
    pub fn new(evm_rpc: impl Into<String>, solana_rpc: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(1))
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            client,
            evm_rpc: evm_rpc.into(),
            solana_rpc: solana_rpc.into(),
        }
    }

    async fn call(
        &self,
        url: &str,
        body: serde_json::Value,
    ) -> Result<serde_json::Value, ExternalError> {
        let mut last_error = String::new();
        for attempt in 0..3u32 {
            match self.client.post(url).json(&body).send().await {
                Ok(response) if response.status().is_success() => {
                    return response.json().await.map_err(|error| {
                        ExternalError::Unavailable(format!("the RPC answer is unreadable: {error}"))
                    });
                }
                Ok(response) if is_transient(response.status().as_u16()) && attempt < 2 => {
                    pause_before_retry(attempt).await;
                }
                Ok(response) => {
                    return Err(ExternalError::Refused(format!(
                        "the RPC answered {}",
                        response.status()
                    )));
                }
                Err(error) if attempt < 2 => {
                    last_error = error.to_string();
                    pause_before_retry(attempt).await;
                }
                Err(error) => {
                    return Err(ExternalError::Unavailable(format!(
                        "the RPC is unreachable: {error}"
                    )));
                }
            }
        }
        Err(ExternalError::Unavailable(format!(
            "the RPC did not answer: {last_error}"
        )))
    }

    async fn evm_balance(&self, address: &str) -> Result<Decimal, ExternalError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "eth_getBalance",
            "params": [address, "latest"],
        });
        let answer = self.call(self.evm_rpc.trim_end(), body).await?;
        let result = answer
            .get("result")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ExternalError::Unavailable("the RPC returned no balance".to_string()))?;
        let hex = result.trim_start_matches("0x");
        let wei = u128::from_str_radix(hex, 16)
            .map_err(|_| ExternalError::Unavailable("the balance is not a number".to_string()))?;
        Ok(Decimal::from(wei) / Decimal::from(1_000_000_000_000_000_000u64))
    }

    async fn solana_balance(&self, address: &str) -> Result<Decimal, ExternalError> {
        let body = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBalance",
            "params": [address],
        });
        let answer = self.call(self.solana_rpc.trim_end(), body).await?;
        let lamports = answer
            .get("result")
            .and_then(|result| result.get("value"))
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| ExternalError::Unavailable("the RPC returned no balance".to_string()))?;
        Ok(Decimal::from(lamports) / Decimal::from(1_000_000_000u64))
    }
}

#[async_trait]
impl ChainProvider for HttpChainProvider {
    async fn balances(
        &self,
        chain: &str,
        address: &str,
    ) -> Result<Vec<ObservedBalance>, ExternalError> {
        if chain == "solana" {
            let quantity = self.solana_balance(address).await?;
            return Ok(vec![ObservedBalance {
                observed_token: "SOL".to_string(),
                quantity,
            }]);
        }
        if let Some(network) = chain.strip_prefix("evm:") {
            let quantity = self.evm_balance(address).await?;
            return Ok(vec![ObservedBalance {
                observed_token: native_symbol(network).to_string(),
                quantity,
            }]);
        }
        Err(ExternalError::Refused(format!(
            "chain '{chain}' is not supported"
        )))
    }
}

fn native_symbol(network: &str) -> &'static str {
    match network {
        "1" | "10" | "42161" | "8453" => "ETH",
        "137" => "POL",
        "56" => "BNB",
        "43114" => "AVAX",
        _ => "ETH",
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RpcError {
    code: i64,
    message: String,
}
