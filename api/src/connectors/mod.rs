pub mod okx;
pub mod evm;
pub mod coinpaprika;
// Legacy CoinGecko connector (deprecated in favor of CoinPaprika)
// pub mod coingecko;
// TODO: Solana connector temporarily disabled due to dependency conflicts
// pub mod solana;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Balance information for a single asset
/// 
/// NOTE: This struct contains NO price or valuation fields by design.
/// Holdings store only quantities. Price/valuation is calculated separately
/// during portfolio construction using reference price data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub quantity: String,
    pub available: String,
    pub frozen: String,
}

/// Trait for exchange connectors
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    /// Fetch spot balances from the exchange
    async fn fetch_spot_balances(&self) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>>;
}
