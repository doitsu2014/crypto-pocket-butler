use crate::concurrency::RateLimiter;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use tracing;

/// Helper function to log detailed error messages for CoinPaprika API failures
fn log_coinpaprika_error(status: reqwest::StatusCode, error_text: &str) {
    if status.as_u16() == 429 {
        tracing::error!(
            "CoinPaprika API 429 Too Many Requests - Rate limit exceeded. Error: {}",
            error_text
        );
    } else {
        tracing::error!("CoinPaprika API error: {} - {}", status, error_text);
    }
}


/// CoinPaprika API client for fetching market data
pub struct CoinPaprikaConnector {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
    api_key: Option<String>,
}

/// Coin data from CoinPaprika API /tickers endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinMarketData {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: u32,
    #[serde(default)]
    pub circulating_supply: Option<f64>,
    #[serde(default)]
    pub total_supply: Option<f64>,
    #[serde(default)]
    pub max_supply: Option<f64>,
    #[serde(default)]
    pub beta_value: Option<f64>,
    #[serde(default)]
    pub first_data_at: Option<String>,
    #[serde(default)]
    pub last_updated: Option<String>,
    pub quotes: Quotes,
}

/// Quote data in USD
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quotes {
    #[serde(rename = "USD")]
    pub usd: UsdQuote,
}

/// USD quote details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdQuote {
    pub price: f64,
    pub volume_24h: Option<f64>,
    pub market_cap: f64,
    pub percent_change_24h: Option<f64>,
    #[serde(default)]
    pub percent_change_1h: Option<f64>,
    #[serde(default)]
    pub percent_change_7d: Option<f64>,
    #[serde(default)]
    pub percent_change_30d: Option<f64>,
    #[serde(default)]
    pub ath_price: Option<f64>,
    #[serde(default)]
    pub ath_date: Option<String>,
    #[serde(default)]
    pub percent_from_price_ath: Option<f64>,
}

/// Detailed coin data from CoinPaprika /coins/{id} API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinDetailData {
    pub id: String,
    pub name: String,
    pub symbol: String,
    /// Contracts array containing platform and contract address information
    #[serde(default)]
    pub contracts: Vec<ContractInfo>,
}

/// Contract information for a specific platform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractInfo {
    #[serde(rename = "type")]
    pub platform_type: String,
    pub contract: String,
}

/// Basic coin metadata returned by the CoinPaprika /v1/coins listing endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinBasicInfo {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub rank: u32,
    #[serde(default)]
    pub is_active: bool,
}

impl CoinPaprikaConnector {
    /// Create a new CoinPaprika connector
    /// 
    /// Checks for COINPAPRIKA_API_KEY environment variable. If set, uses Pro API.
    /// Otherwise, uses the free public API.
    pub fn new() -> Self {
        let api_key = std::env::var("COINPAPRIKA_API_KEY").ok();
        let has_api_key = api_key.is_some();
        
        let base_url = if has_api_key {
            // Use Pro API endpoint (if available)
            tracing::info!("CoinPaprika Pro API enabled with API key");
            "https://api.coinpaprika.com/v1".to_string()
        } else {
            // Use free public API endpoint
            tracing::info!("CoinPaprika using free public API (rate limited)");
            "https://api.coinpaprika.com/v1".to_string()
        };
        
        Self {
            client: Client::new(),
            base_url,
            rate_limiter: RateLimiter::coinpaprika(),
            api_key,
        }
    }

    /// Fetch top N coins by market cap
    /// 
    /// # Arguments
    /// * `limit` - Number of coins to fetch (1-250)
    /// 
    /// # Returns
    /// Vector of coin market data sorted by market cap rank
    pub async fn fetch_top_coins(&self, limit: usize) -> Result<Vec<CoinMarketData>, Box<dyn Error + Send + Sync>> {
        if limit == 0 || limit > 250 {
            return Err(format!("Invalid limit: {}. Limit must be between 1 and 250", limit).into());
        }

        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await?;
        
        tracing::info!("Fetching top {} coins from CoinPaprika", limit);

        let url = format!(
            "{}/tickers?limit={}",
            self.base_url,
            limit
        );

        let mut request = self.client
            .get(&url)
            .header("accept", "application/json");
        
        // Add API key header if available (for Pro API)
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log_coinpaprika_error(status, &error_text);
            return Err(format!("CoinPaprika API error: {} - {}", status, error_text).into());
        }

        let coins: Vec<CoinMarketData> = response.json().await?;
        
        tracing::info!("Successfully fetched {} coins from CoinPaprika", coins.len());
        
        Ok(coins)
    }

    /// Fetch all active coins from CoinPaprika
    /// 
    /// # Returns
    /// Vector of all active coin market data sorted by market cap rank
    pub async fn fetch_all_coins(&self) -> Result<Vec<CoinMarketData>, Box<dyn Error + Send + Sync>> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await?;
        
        tracing::info!("Fetching all coins from CoinPaprika");

        // Without limit parameter, CoinPaprika returns all active coins
        let url = format!("{}/tickers", self.base_url);

        let mut request = self.client
            .get(&url)
            .header("accept", "application/json");
        
        // Add API key header if available (for Pro API)
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log_coinpaprika_error(status, &error_text);
            return Err(format!("CoinPaprika API error: {} - {}", status, error_text).into());
        }

        let coins: Vec<CoinMarketData> = response.json().await?;
        
        tracing::info!("Successfully fetched {} coins from CoinPaprika", coins.len());
        
        Ok(coins)
    }

    /// Ping CoinPaprika API to check connectivity
    pub async fn ping(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/tickers?limit=1", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Fetch prices for specific coins by their CoinPaprika IDs
    /// 
    /// # Arguments
    /// * `coin_ids` - Vector of CoinPaprika coin IDs (e.g., ["btc-bitcoin", "eth-ethereum"])
    /// 
    /// # Returns
    /// Vector of coin market data for the requested coins
    pub async fn fetch_coins_by_ids(&self, coin_ids: &[String]) -> Result<Vec<CoinMarketData>, Box<dyn Error + Send + Sync>> {
        if coin_ids.is_empty() {
            return Ok(Vec::new());
        }

        tracing::info!("Fetching {} coins by ID from CoinPaprika", coin_ids.len());

        let mut results = Vec::new();

        for coin_id in coin_ids {
            // Acquire rate limit permit for each request
            let _permit = self.rate_limiter.acquire().await?;
            
            let url = format!(
                "{}/tickers/{}",
                self.base_url,
                coin_id
            );

            let mut request = self.client
                .get(&url)
                .header("accept", "application/json");
            
            // Add API key header if available (for Pro API)
            if let Some(key) = &self.api_key {
                request = request.header("Authorization", format!("Bearer {}", key));
            }
            
            let response = request.send().await?;

            if !response.status().is_success() {
                let status = response.status();
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                log_coinpaprika_error(status, &error_text);
                tracing::warn!("Failed to fetch coin {}: {} - {}", coin_id, status, error_text);
                continue;
            }

            let coin: CoinMarketData = response.json().await?;
            results.push(coin);
        }
        
        tracing::info!("Successfully fetched {} coins by ID from CoinPaprika", results.len());
        
        Ok(results)
    }

    /// Fetch detailed coin information including contract addresses
    /// 
    /// # Arguments
    /// * `coin_id` - CoinPaprika coin ID (e.g., "btc-bitcoin", "eth-ethereum")
    /// 
    /// # Returns
    /// Detailed coin data including contracts/platforms
    pub async fn fetch_coin_detail(&self, coin_id: &str) -> Result<CoinDetailData, Box<dyn Error + Send + Sync>> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await?;
        
        tracing::debug!("Fetching coin detail for {} from CoinPaprika", coin_id);

        let url = format!(
            "{}/coins/{}",
            self.base_url,
            coin_id
        );

        let mut request = self.client
            .get(&url)
            .header("accept", "application/json");
        
        // Add API key header if available (for Pro API)
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            log_coinpaprika_error(status, &error_text);
            tracing::warn!("CoinPaprika API error for {}: {} - {}", coin_id, status, error_text);
            return Err(format!("CoinPaprika API error: {} - {}", status, error_text).into());
        }

        let coin_detail: CoinDetailData = response.json().await?;
        
        tracing::debug!(
            "Successfully fetched coin detail for {}: {} contracts",
            coin_id,
            coin_detail.contracts.len()
        );
        
        Ok(coin_detail)
    }

    /// Search CoinPaprika for coins matching a given symbol.
    ///
    /// Fetches the full coin listing (`GET /v1/coins`) in a single request and filters
    /// client-side by symbol (case-insensitive). Results are sorted by rank (ascending) so
    /// the highest market-cap coin for that symbol comes first.
    ///
    /// This is intentionally **one API call**, regardless of how many coins are returned,
    /// to stay within free-tier rate limits.
    pub async fn search_coins_by_symbol(
        &self,
        symbol: &str,
    ) -> Result<Vec<CoinBasicInfo>, Box<dyn Error + Send + Sync>> {
        let _permit = self.rate_limiter.acquire().await?;

        let url = format!("{}/coins", self.base_url);

        let mut request = self.client.get(&url).header("accept", "application/json");
        if let Some(key) = &self.api_key {
            request = request.header("Authorization", format!("Bearer {}", key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            log_coinpaprika_error(status, &error_text);
            return Err(
                format!("CoinPaprika /coins error: {} - {}", status, error_text).into(),
            );
        }

        let all_coins: Vec<CoinBasicInfo> = response.json().await?;

        let symbol_upper = symbol.to_uppercase();
        let mut matches: Vec<CoinBasicInfo> = all_coins
            .into_iter()
            .filter(|c| c.is_active && c.symbol.to_uppercase() == symbol_upper)
            .collect();

        // Sort by rank ascending (rank=0 means unranked; push those to the end)
        const UNRANKED_SORT_KEY: u32 = u32::MAX;
        matches.sort_by_key(|c| if c.rank == 0 { UNRANKED_SORT_KEY } else { c.rank });

        tracing::info!(
            "CoinPaprika symbol search '{}': {} matches",
            symbol,
            matches.len()
        );

        Ok(matches)
    }

    /// Helper method to convert CoinPaprika contracts to platform map format
    /// This provides compatibility with existing code that expects HashMap<String, String>
    pub fn contracts_to_platform_map(contracts: &[ContractInfo]) -> HashMap<String, String> {
        let mut platforms = HashMap::new();
        
        for contract in contracts {
            // Normalize platform names to match what we expect
            let platform_type_lower = contract.platform_type.to_lowercase();
            let platform = match platform_type_lower.as_str() {
                "erc20" => "ethereum",
                "bep20" => "binance-smart-chain",
                "polygon" => "polygon-pos",
                platform_name => {
                    // Log unrecognized platform types for monitoring
                    if !platform_name.is_empty() {
                        tracing::warn!(
                            "Unrecognized platform type from CoinPaprika: '{}'. Consider adding mapping.",
                            platform_name
                        );
                    }
                    platform_name
                }
            };
            
            platforms.insert(platform.to_string(), contract.contract.clone());
        }
        
        platforms
    }
}

impl Default for CoinPaprikaConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_creation() {
        let connector = CoinPaprikaConnector::new();
        assert_eq!(connector.base_url, "https://api.coinpaprika.com/v1");
    }

    #[test]
    fn test_contracts_to_platform_map() {
        let contracts = vec![
            ContractInfo {
                platform_type: "ERC20".to_string(),
                contract: "0x1234".to_string(),
            },
            ContractInfo {
                platform_type: "BEP20".to_string(),
                contract: "0x5678".to_string(),
            },
        ];
        
        let platforms = CoinPaprikaConnector::contracts_to_platform_map(&contracts);
        
        assert_eq!(platforms.get("ethereum"), Some(&"0x1234".to_string()));
        assert_eq!(platforms.get("binance-smart-chain"), Some(&"0x5678".to_string()));
    }
}
