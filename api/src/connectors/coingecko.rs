use crate::concurrency::RateLimiter;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use tracing;

/// CoinGecko API client for fetching market data
pub struct CoinGeckoConnector {
    client: Client,
    base_url: String,
    rate_limiter: RateLimiter,
}

/// Coin data from CoinGecko API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinMarketData {
    pub id: String,
    pub symbol: String,
    pub name: String,
    pub image: String,
    pub current_price: f64,
    pub market_cap: f64,
    pub market_cap_rank: Option<u32>,
    pub total_volume: Option<f64>,
    pub price_change_percentage_24h: Option<f64>,
    // Note: CoinGecko doesn't provide market dominance in the markets endpoint
    // Market dominance would need to be calculated separately using total market cap
}

/// Detailed coin data from CoinGecko /coins/{id} API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoinDetailData {
    pub id: String,
    pub symbol: String,
    pub name: String,
    /// Map of blockchain platforms to contract addresses
    /// e.g., {"ethereum": "0x...", "polygon-pos": "0x...", "binance-smart-chain": "0x..."}
    pub platforms: HashMap<String, String>,
}

impl CoinGeckoConnector {
    /// Create a new CoinGecko connector
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.coingecko.com/api/v3".to_string(),
            rate_limiter: RateLimiter::coingecko(),
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
        
        tracing::info!("Fetching top {} coins from CoinGecko", limit);

        let url = format!(
            "{}/coins/markets?vs_currency=usd&order=market_cap_desc&per_page={}&page=1&sparkline=false&price_change_percentage=24h",
            self.base_url,
            limit
        );

        let response = self.client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("CoinGecko API error: {} - {}", status, error_text);
            return Err(format!("CoinGecko API error: {} - {}", status, error_text).into());
        }

        let coins: Vec<CoinMarketData> = response.json().await?;
        
        tracing::info!("Successfully fetched {} coins from CoinGecko", coins.len());
        
        Ok(coins)
    }

    /// Ping CoinGecko API to check connectivity
    pub async fn ping(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let url = format!("{}/ping", self.base_url);
        
        let response = self.client
            .get(&url)
            .send()
            .await?;

        Ok(response.status().is_success())
    }

    /// Fetch prices for specific coins by their CoinGecko IDs
    /// 
    /// # Arguments
    /// * `coin_ids` - Vector of CoinGecko coin IDs (e.g., ["bitcoin", "ethereum"])
    /// 
    /// # Returns
    /// Vector of coin market data for the requested coins
    pub async fn fetch_coins_by_ids(&self, coin_ids: &[String]) -> Result<Vec<CoinMarketData>, Box<dyn Error + Send + Sync>> {
        if coin_ids.is_empty() {
            return Ok(Vec::new());
        }

        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await?;
        
        // CoinGecko API accepts comma-separated list of IDs
        let ids_param = coin_ids.join(",");
        
        tracing::info!("Fetching {} coins by ID from CoinGecko", coin_ids.len());

        let url = format!(
            "{}/coins/markets?vs_currency=usd&ids={}&sparkline=false&price_change_percentage=24h",
            self.base_url,
            ids_param
        );

        let response = self.client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            tracing::error!("CoinGecko API error: {} - {}", status, error_text);
            return Err(format!("CoinGecko API error: {} - {}", status, error_text).into());
        }

        let coins: Vec<CoinMarketData> = response.json().await?;
        
        tracing::info!("Successfully fetched {} coins by ID from CoinGecko", coins.len());
        
        Ok(coins)
    }

    /// Fetch detailed coin information including contract addresses
    /// 
    /// # Arguments
    /// * `coin_id` - CoinGecko coin ID (e.g., "bitcoin", "ethereum")
    /// 
    /// # Returns
    /// Detailed coin data including platforms/contract addresses
    pub async fn fetch_coin_detail(&self, coin_id: &str) -> Result<CoinDetailData, Box<dyn Error + Send + Sync>> {
        // Acquire rate limit permit
        let _permit = self.rate_limiter.acquire().await?;
        
        tracing::debug!("Fetching coin detail for {} from CoinGecko", coin_id);

        let url = format!(
            "{}/coins/{}?localization=false&tickers=false&market_data=false&community_data=false&developer_data=false&sparkline=false",
            self.base_url,
            coin_id
        );

        let response = self.client
            .get(&url)
            .header("accept", "application/json")
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            tracing::warn!("CoinGecko API error for {}: {} - {}", coin_id, status, error_text);
            return Err(format!("CoinGecko API error: {} - {}", status, error_text).into());
        }

        let coin_detail: CoinDetailData = response.json().await?;
        
        tracing::debug!(
            "Successfully fetched coin detail for {}: {} platforms",
            coin_id,
            coin_detail.platforms.len()
        );
        
        Ok(coin_detail)
    }

}

impl Default for CoinGeckoConnector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_creation() {
        let connector = CoinGeckoConnector::new();
        assert_eq!(connector.base_url, "https://api.coingecko.com/api/v3");
    }
}
