/// Asset domain entities: AssetPrice and AssetContract

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A price record for an asset on a specific chain or exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPrice {
    /// Asset symbol (e.g. "BTC").
    pub asset: String,
    /// Price in USD.
    pub price_usd: Decimal,
    /// When this price was last updated.
    pub updated_at: DateTime<Utc>,
    /// Market cap rank (lower = higher market cap).
    pub rank: Option<i32>,
    /// 24 h trading volume in USD.
    pub volume_24h_usd: Option<Decimal>,
    /// 24 h price change as a percentage.
    pub change_percent_24h: Option<Decimal>,
}

impl AssetPrice {
    pub fn new(asset: impl Into<String>, price_usd: Decimal) -> Self {
        Self {
            asset: asset.into(),
            price_usd,
            updated_at: Utc::now(),
            rank: None,
            volume_24h_usd: None,
            change_percent_24h: None,
        }
    }
}

/// A smart-contract address binding an asset to a chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetContract {
    /// Asset symbol.
    pub asset: String,
    /// Chain identifier (e.g. "ethereum", "bsc").
    pub chain: String,
    /// Contract address (hex for EVM, base58 for Solana).
    pub contract_address: String,
    /// Token decimal places.
    pub decimals: Option<u8>,
}

impl AssetContract {
    pub fn new(
        asset: impl Into<String>,
        chain: impl Into<String>,
        contract_address: impl Into<String>,
        decimals: Option<u8>,
    ) -> Self {
        Self {
            asset: asset.into(),
            chain: chain.into(),
            contract_address: contract_address.into(),
            decimals,
        }
    }
}
