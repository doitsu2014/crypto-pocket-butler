//! Simple caching layer for external data with short TTL
//!
//! This module provides in-memory caching for frequently accessed data like prices
//! to reduce load on external APIs and databases.
//!
//! # Design
//!
//! - Uses moka cache for async-friendly, thread-safe caching
//! - Short TTLs (30-60 seconds) to balance freshness vs performance
//! - Size-bounded to prevent memory issues
//!
//! # Usage
//!
//! ```rust
//! use crate::cache::PriceCache;
//!
//! let cache = PriceCache::new();
//! if let Some(price) = cache.get(&asset_id).await {
//!     return price;
//! }
//! // Fetch from DB/API and cache
//! cache.insert(asset_id, price).await;
//! ```

use moka::future::Cache;
use rust_decimal::Decimal;
use std::time::Duration;
use uuid::Uuid;

/// Cache for asset prices with 60 second TTL
#[derive(Clone)]
pub struct PriceCache {
    cache: Cache<Uuid, Decimal>,
}

impl PriceCache {
    /// Create a new price cache
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(10_000) // Cache up to 10k assets
                .time_to_live(Duration::from_secs(60)) // 60 second TTL
                .build(),
        }
    }

    /// Get a cached price
    pub async fn get(&self, asset_id: &Uuid) -> Option<Decimal> {
        self.cache.get(asset_id).await
    }

    /// Insert a price into the cache
    pub async fn insert(&self, asset_id: Uuid, price: Decimal) {
        self.cache.insert(asset_id, price).await;
    }

    /// Batch insert multiple prices
    pub async fn insert_many(&self, prices: Vec<(Uuid, Decimal)>) {
        for (asset_id, price) in prices {
            self.cache.insert(asset_id, price).await;
        }
    }
}

impl Default for PriceCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Cache for chain RPC responses with 30 second TTL
#[derive(Clone)]
pub struct ChainDataCache {
    cache: Cache<String, String>,
}

impl ChainDataCache {
    /// Create a new chain data cache
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(1_000) // Cache up to 1k responses
                .time_to_live(Duration::from_secs(30)) // 30 second TTL
                .build(),
        }
    }

    /// Get cached data by key (e.g., "ethereum:0xaddress:balance")
    pub async fn get(&self, key: &str) -> Option<String> {
        self.cache.get(key).await
    }

    /// Insert data into the cache
    pub async fn insert(&self, key: String, data: String) {
        self.cache.insert(key, data).await;
    }
}

impl Default for ChainDataCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_price_cache_basic() {
        let cache = PriceCache::new();
        let asset_id = Uuid::new_v4();
        let price = Decimal::from_str("100.50").unwrap();

        // Should be empty initially
        assert!(cache.get(&asset_id).await.is_none());

        // Insert and retrieve
        cache.insert(asset_id, price).await;
        assert_eq!(cache.get(&asset_id).await, Some(price));
    }

    #[tokio::test]
    async fn test_price_cache_batch_insert() {
        let cache = PriceCache::new();
        let prices = vec![
            (Uuid::new_v4(), Decimal::from_str("10.0").unwrap()),
            (Uuid::new_v4(), Decimal::from_str("20.0").unwrap()),
            (Uuid::new_v4(), Decimal::from_str("30.0").unwrap()),
        ];

        cache.insert_many(prices.clone()).await;

        for (asset_id, price) in prices {
            assert_eq!(cache.get(&asset_id).await, Some(price));
        }
    }

    #[tokio::test]
    async fn test_chain_data_cache() {
        let cache = ChainDataCache::new();
        let key = "ethereum:0x123:balance".to_string();
        let data = "1000000000000000000".to_string();

        assert!(cache.get(&key).await.is_none());

        cache.insert(key.clone(), data.clone()).await;
        assert_eq!(cache.get(&key).await, Some(data));
    }
}
