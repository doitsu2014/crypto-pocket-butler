/// AssetRepository trait — persistence interface for the Asset domain.

use async_trait::async_trait;

use super::aggregate::{Asset, AssetError};

/// Persistence interface for `Asset` aggregates.
#[async_trait]
pub trait AssetRepository: Send + Sync {
    /// Find an asset by symbol (case-sensitive).
    async fn find_by_symbol(&self, symbol: &str) -> Result<Option<Asset>, AssetError>;

    /// Find an asset by (symbol, name) pair.
    async fn find_by_symbol_and_name(
        &self,
        symbol: &str,
        name: &str,
    ) -> Result<Option<Asset>, AssetError>;

    /// Find an asset by CoinPaprika ID.
    async fn find_by_coinpaprika_id(&self, id: &str) -> Result<Option<Asset>, AssetError>;

    /// Return all tracked assets.
    async fn find_all(&self) -> Result<Vec<Asset>, AssetError>;

    /// Persist a new or modified asset.
    async fn save(&self, asset: &Asset) -> Result<(), AssetError>;
}
