use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing;

/// CoinGecko API client for fetching market data
pub struct CoinGeckoConnector {
    client: Client,
    base_url: String,
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
    pub market_cap_change_percentage_24h: Option<f64>,
}

impl CoinGeckoConnector {
    /// Create a new CoinGecko connector
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            base_url: "https://api.coingecko.com/api/v3".to_string(),
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
            return Err("Limit must be between 1 and 250".into());
        }

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
