pub mod okx;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Balance information for a single asset
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
