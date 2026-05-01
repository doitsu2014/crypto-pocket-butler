/// Asset aggregate root

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::entities::{AssetContract, AssetPrice};

/// The Asset aggregate root.
///
/// Represents a financial asset (cryptocurrency, token) with its pricing and
/// contract addresses across chains.
///
/// # Invariants
/// - `symbol` is unique together with `name` in the system.
/// - At most one price record per asset (the latest).
/// - At most one contract address per (asset, chain) pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    /// Asset symbol (e.g. "BTC", "ETH").
    pub symbol: String,
    /// Full asset name (e.g. "Bitcoin", "Ethereum").
    pub name: String,
    /// CoinPaprika coin ID for price lookups (e.g. "btc-bitcoin").
    pub coinpaprika_id: Option<String>,
    /// Market cap rank (lower = higher market cap).
    pub rank: Option<i32>,
    /// Whether this asset is actively tracked.
    pub is_active: bool,

    /// Current price data (if available).
    price: Option<AssetPrice>,
    /// Contract addresses per chain.
    contracts: HashMap<String, AssetContract>,
}

impl Asset {
    pub fn new(symbol: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            symbol: symbol.into(),
            name: name.into(),
            coinpaprika_id: None,
            rank: None,
            is_active: true,
            price: None,
            contracts: HashMap::new(),
        }
    }

    /// Reconstruct from persisted state.
    pub fn from_persistence(
        symbol: String,
        name: String,
        coinpaprika_id: Option<String>,
        rank: Option<i32>,
        is_active: bool,
        price: Option<AssetPrice>,
        contracts: Vec<AssetContract>,
    ) -> Self {
        let contracts = contracts
            .into_iter()
            .map(|c| (c.chain.clone(), c))
            .collect();
        Self {
            symbol,
            name,
            coinpaprika_id,
            rank,
            is_active,
            price,
            contracts,
        }
    }

    // ─── Query methods ──────────────────────────────────────────────────────────

    pub fn current_price(&self) -> Option<&AssetPrice> {
        self.price.as_ref()
    }

    pub fn current_price_usd(&self) -> Option<Decimal> {
        self.price.as_ref().map(|p| p.price_usd)
    }

    pub fn contract_for_chain(&self, chain: &str) -> Option<&AssetContract> {
        self.contracts.get(chain)
    }

    pub fn contracts(&self) -> impl Iterator<Item = &AssetContract> {
        self.contracts.values()
    }

    pub fn is_priced(&self) -> bool {
        self.price.is_some()
    }

    // ─── Command methods ─────────────────────────────────────────────────────────

    /// Update (replace) the current price record.
    pub fn update_price(&mut self, price: AssetPrice) {
        self.price = Some(price);
    }

    /// Add or replace a contract address for a chain.
    pub fn add_contract(&mut self, contract: AssetContract) {
        self.contracts.insert(contract.chain.clone(), contract);
    }

    /// Remove the contract for a specific chain. Returns `true` if removed.
    pub fn remove_contract(&mut self, chain: &str) -> bool {
        self.contracts.remove(chain).is_some()
    }
}

/// Domain errors for the Asset aggregate.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum AssetError {
    #[error("asset not found")]
    NotFound,
    #[error("persistence error: {0}")]
    PersistenceError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_asset_price() {
        let mut asset = Asset::new("BTC", "Bitcoin");
        assert!(!asset.is_priced());
        asset.update_price(AssetPrice::new("BTC", Decimal::new(50000, 0)));
        assert!(asset.is_priced());
        assert_eq!(asset.current_price_usd(), Some(Decimal::new(50000, 0)));
    }

    #[test]
    fn test_asset_contract() {
        let mut asset = Asset::new("USDC", "USD Coin");
        asset.add_contract(AssetContract::new(
            "USDC",
            "ethereum",
            "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48",
            Some(6),
        ));
        assert!(asset.contract_for_chain("ethereum").is_some());
        assert!(asset.remove_contract("ethereum"));
        assert!(asset.contract_for_chain("ethereum").is_none());
    }
}
