//! The shared Postgres token bucket (BR4.3).
//!
//! One row per source is refilled and taken under a row lock, so every replica
//! of the worker shares the same budget. The arithmetic is the domain's
//! [`market_data_domain::rate::refill_tokens`] rule.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_data_application::ports::{RateLimiter, RepositoryError};
use market_data_domain::rate::refill_tokens;
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed shared token buckets.
#[derive(Debug, Clone)]
pub struct PgRateLimiter {
    pool: PgPool,
}

impl PgRateLimiter {
    /// Creates the limiter over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct BucketRow {
    tokens: i32,
    updated_at: DateTime<Utc>,
}

#[async_trait]
impl RateLimiter for PgRateLimiter {
    async fn try_acquire(
        &self,
        source_name: &str,
        rate_limit_per_minute: u32,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let capacity = i64::from(rate_limit_per_minute);
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin rate limit take", error))?;
        // First use starts full; the take below consumes one.
        sqlx::query(
            "INSERT INTO source_rate_buckets (source_name, tokens, updated_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (source_name) DO NOTHING",
        )
        .bind(source_name)
        .bind(i32::try_from(capacity).unwrap_or(i32::MAX))
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("create rate bucket", error))?;

        let bucket = sqlx::query_as::<_, BucketRow>(
            "SELECT tokens, updated_at FROM source_rate_buckets
             WHERE source_name = $1 FOR UPDATE",
        )
        .bind(source_name)
        .fetch_one(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("read rate bucket", error))?;

        let elapsed = (now - bucket.updated_at).num_seconds();
        let available = refill_tokens(i64::from(bucket.tokens), capacity, elapsed);
        let acquired = available >= 1;
        let remaining = if acquired { available - 1 } else { available };
        sqlx::query(
            "UPDATE source_rate_buckets SET tokens = $2, updated_at = $3
             WHERE source_name = $1",
        )
        .bind(source_name)
        .bind(i32::try_from(remaining).unwrap_or(i32::MAX))
        .bind(now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("update rate bucket", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit rate limit take", error))?;
        Ok(acquired)
    }
}
