//! Scrape-time backlog readings for the support audit outbox (NFR17.7, NFR17.8).
//!
//! The pending count and the oldest row's age are read with `COUNT(*)` and
//! `MIN(created_at)` when `/metrics` is scraped, so the gauge is correct even
//! when the drainer has stopped.

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Reads the support outbox's pending backlog.
#[derive(Debug, Clone)]
pub struct SupportOutboxBacklog {
    pool: PgPool,
}

impl SupportOutboxBacklog {
    /// Creates the reader over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Returns `(pending rows, oldest pending row's age in seconds)`.
    ///
    /// # Errors
    /// Returns the driver error when the read fails.
    pub async fn read(&self, now: DateTime<Utc>) -> Result<(u64, i64), sqlx::Error> {
        let (pending, oldest_age): (i64, i64) = sqlx::query_as(
            "SELECT count(*)::bigint,
                    COALESCE(EXTRACT(EPOCH FROM ($1 - MIN(created_at)))::bigint, 0)
             FROM support_audit_outbox
             WHERE state = 'pending'",
        )
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        Ok((pending.max(0) as u64, oldest_age.max(0)))
    }
}
