use super::{Balance, ExchangeConnector};
use async_trait::async_trait;
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest;
use serde::Deserialize;
use sha2::Sha256;
use std::error::Error;
use tracing;

type HmacSha256 = Hmac<Sha256>;

const OKX_API_BASE_URL: &str = "https://www.okx.com";

/// OKX API response wrapper
#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    code: String,
    msg: String,
    data: Vec<T>,
}

/// OKX balance data structure
/// 
/// NOTE: This struct intentionally omits valuation fields (eq, totalEq, upl) from the OKX API response.
/// Account holdings store ONLY quantities, not valuations. Price/valuation is calculated separately
/// during portfolio construction using reference price data.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OkxBalanceData {
    avail_bal: String,
    bal: String,
    ccy: String,
    frozen_bal: String,
}

/// OKX connector for read-only access
pub struct OkxConnector {
    api_key: String,
    api_secret: String,
    passphrase: String,
    client: reqwest::Client,
}

impl OkxConnector {
    /// Create a new OKX connector with API credentials
    pub fn new(api_key: String, api_secret: String, passphrase: String) -> Self {
        Self {
            api_key,
            api_secret,
            passphrase,
            client: reqwest::Client::new(),
        }
    }

    /// Generate signature for OKX API request
    fn generate_signature(&self, timestamp: &str, method: &str, request_path: &str) -> String {
        let prehash = format!("{}{}{}", timestamp, method, request_path);
        
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(prehash.as_bytes());
        
        let result = mac.finalize();
        general_purpose::STANDARD.encode(result.into_bytes())
    }

    /// Make an authenticated GET request to OKX API
    async fn get_request<T: for<'de> Deserialize<'de>>(
        &self,
        endpoint: &str,
    ) -> Result<T, Box<dyn Error + Send + Sync>> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        let method = "GET";
        let request_path = endpoint;
        
        let signature = self.generate_signature(&timestamp, method, request_path);
        
        let url = format!("{}{}", OKX_API_BASE_URL, endpoint);
        
        tracing::debug!("OKX API Request: {} {}", method, url);
        
        let response = self
            .client
            .get(&url)
            .header("OK-ACCESS-KEY", &self.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.passphrase)
            .header("Content-Type", "application/json")
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;
        
        tracing::debug!("OKX API Response Status: {}", status);
        tracing::debug!("OKX API Response Body: {}", body);

        if !status.is_success() {
            return Err(format!("OKX API error: {} - {}", status, body).into());
        }

        serde_json::from_str(&body).map_err(|e| {
            tracing::error!("Failed to parse OKX response: {}", e);
            format!("Failed to parse OKX response: {}", e).into()
        })
    }
}

#[async_trait]
impl ExchangeConnector for OkxConnector {
    async fn fetch_spot_balances(&self) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>> {
        // OKX API endpoint for account balance
        // Using trading account (balance details)
        let endpoint = "/api/v5/account/balance";
        
        let response: OkxResponse<serde_json::Value> = self.get_request(endpoint).await?;
        
        if response.code != "0" {
            return Err(format!("OKX API error: {} - {}", response.code, response.msg).into());
        }

        let mut balances = Vec::new();

        // Parse the balance data
        // OKX returns balance in a nested structure
        for item in response.data {
            if let Some(details) = item.get("details").and_then(|d| d.as_array()) {
                for detail in details {
                    if let Ok(balance_data) = serde_json::from_value::<OkxBalanceData>(detail.clone()) {
                        // Only include assets with non-zero balance
                        if let Ok(bal) = balance_data.bal.parse::<f64>() {
                            if bal > 0.0 {
                                balances.push(Balance {
                                    asset: balance_data.ccy,
                                    quantity: balance_data.bal,
                                    available: balance_data.avail_bal,
                                    frozen: balance_data.frozen_bal,
                                    decimals: None, // OKX doesn't provide decimal information
                                });
                            }
                        }
                    }
                }
            }
        }

        tracing::info!("Fetched {} balances from OKX", balances.len());
        Ok(balances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_generation() {
        let connector = OkxConnector::new(
            "test-api-key".to_string(),
            "test-secret".to_string(),
            "test-passphrase".to_string(),
        );
        
        let timestamp = "2024-01-01T00:00:00.000Z";
        let method = "GET";
        let path = "/api/v5/account/balance";
        
        let signature = connector.generate_signature(timestamp, method, path);
        
        // Signature should be a non-empty base64 string
        assert!(!signature.is_empty());
        assert!(general_purpose::STANDARD.decode(&signature).is_ok());
    }
}
