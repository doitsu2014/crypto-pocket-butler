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
///
/// The `quantity` field is **always a normalized (human-readable) decimal value**.
/// For EVM connectors, raw on-chain integers are converted using `normalize_token_balance`
/// before being stored here. OKX already returns human-readable values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    /// Normalized (human-readable) decimal quantity, e.g. "1.5" for 1.5 ETH
    pub quantity: String,
    pub available: String,
    pub frozen: String,
    /// Number of decimal places for this token (e.g., 18 for ETH, 6 for USDC)
    /// Kept as metadata; the quantity field is already normalized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decimals: Option<u8>,
}

/// Trait for exchange connectors
#[async_trait]
pub trait ExchangeConnector: Send + Sync {
    /// Fetch spot balances from the exchange
    async fn fetch_spot_balances(&self) -> Result<Vec<Balance>, Box<dyn Error + Send + Sync>>;
}
