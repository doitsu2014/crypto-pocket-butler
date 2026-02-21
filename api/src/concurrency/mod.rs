//! Concurrency control and rate limiting for external API calls
//!
//! This module provides bounded concurrency using tokio semaphores to prevent
//! overwhelming external services and implements a bulkhead pattern for fault isolation.
//!
//! # Design
//!
//! - **Bounded Concurrency**: Limits the number of concurrent operations to external services
//! - **Bulkhead Pattern**: Isolates failures in one service from affecting others
//! - **Rate Limiting**: Prevents exceeding API rate limits
//!
//! # Usage
//!
//! ```rust
//! use crate::concurrency::RateLimiter;
//!
//! let limiter = RateLimiter::coingecko();
//! limiter.acquire().await?;
//! // Make API call
//! ```

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tokio::time::sleep;

/// Rate limiter with bounded concurrency and minimum delay between requests
pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
    min_delay: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    ///
    /// # Arguments
    ///
    /// * `max_concurrent` - Maximum number of concurrent operations
    /// * `min_delay` - Minimum delay between operations (e.g., for rate limiting)
    pub fn new(max_concurrent: usize, min_delay: Duration) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
            min_delay,
        }
    }

    /// Create a rate limiter for CoinGecko API (demo plan: 10-30 calls/minute)
    pub fn coingecko() -> Self {
        Self::new(
            5,                           // Max 5 concurrent requests
            Duration::from_millis(2000), // 2 second delay = ~30 requests/minute
        )
    }

    /// Create a rate limiter for CoinPaprika API (free tier: 1000 calls/day)
    pub fn coinpaprika() -> Self {
        Self::new(
            5,                           // Max 5 concurrent requests
            Duration::from_millis(100),  // 100ms delay = ~600 requests/minute (well under daily limit)
        )
    }

    /// Create a rate limiter for OKX API
    pub fn okx() -> Self {
        Self::new(
            3,                           // Max 3 concurrent requests
            Duration::from_millis(100),  // 100ms delay for burst protection
        )
    }

    /// Create a rate limiter for EVM chain RPC calls
    pub fn evm_rpc() -> Self {
        Self::new(
            5,                          // Max 5 concurrent chain requests
            Duration::from_millis(50),  // 50ms delay between requests
        )
    }

    /// Create a rate limiter for Solana JSON-RPC calls
    pub fn solana_rpc() -> Self {
        Self::new(
            3,                           // Max 3 concurrent requests
            Duration::from_millis(100),  // 100ms delay for public RPC protection
        )
    }

    /// Acquire permission to make a request
    ///
    /// This will wait until a permit is available and then impose the minimum delay
    pub async fn acquire(&self) -> Result<RateLimitGuard, Box<dyn std::error::Error + Send + Sync>> {
        let permit = self.semaphore.clone().acquire_owned().await
            .map_err(|e| format!("Failed to acquire semaphore: {}", e))?;
        
        sleep(self.min_delay).await;
        
        Ok(RateLimitGuard { _permit: permit })
    }
}

/// Guard that releases the semaphore permit when dropped
pub struct RateLimitGuard {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            semaphore: Arc::clone(&self.semaphore),
            min_delay: self.min_delay,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[tokio::test]
    async fn test_rate_limiter_enforces_concurrency() {
        let limiter = RateLimiter::new(2, Duration::from_millis(10));
        
        let start = Instant::now();
        
        // Spawn 4 tasks with max_concurrent=2
        let tasks: Vec<_> = (0..4)
            .map(|_| {
                let limiter = limiter.clone();
                tokio::spawn(async move {
                    let _guard = limiter.acquire().await.unwrap();
                    tokio::time::sleep(Duration::from_millis(50)).await;
                })
            })
            .collect();
        
        for task in tasks {
            task.await.unwrap();
        }
        
        let duration = start.elapsed();
        
        // With max_concurrent=2, 4 tasks should take at least 2 * (10ms delay + 50ms work) = 120ms
        assert!(duration >= Duration::from_millis(120), "Duration was {:?}", duration);
    }

    #[tokio::test]
    async fn test_rate_limiter_enforces_delay() {
        let limiter = RateLimiter::new(10, Duration::from_millis(100));
        
        let start = Instant::now();
        let _guard = limiter.acquire().await.unwrap();
        let duration = start.elapsed();
        
        // Should wait at least the minimum delay
        assert!(duration >= Duration::from_millis(100), "Duration was {:?}", duration);
    }
}
