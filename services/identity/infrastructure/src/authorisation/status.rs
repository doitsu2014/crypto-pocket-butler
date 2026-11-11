//! The status read over the Identity service's own pool, bounded at 200 ms
//! acquire and 100 ms statement, with no retry.
//!
//! U2 is embedded in the Identity service, so this is the only status adapter a
//! deployment can exercise: the C4 route terminates here, and an in-process
//! caller already holds the same pool. A read that outlives its bound is a 503
//! (or an outage-cache answer), never a slow allow.

use std::time::Duration;

use async_trait::async_trait;
use identity_application::authorisation::ports::{StatusReadError, StatusReader};
use identity_domain::{UserId, UserStatus};
use sqlx::{Acquire, PgPool};

/// The pool-acquire bound.
pub const ACQUIRE_TIMEOUT: Duration = Duration::from_millis(200);
/// The statement bound.
pub const STATEMENT_TIMEOUT: Duration = Duration::from_millis(100);
/// The statement bound as Postgres sees it.
const STATEMENT_TIMEOUT_SQL: &str = "SET LOCAL statement_timeout = '100ms'";

/// The sqlx-backed status read.
#[derive(Debug, Clone)]
pub struct StatusReaderLocal {
    pool: PgPool,
}

impl StatusReaderLocal {
    /// Creates the reader over the service's pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl StatusReader for StatusReaderLocal {
    async fn status_of(&self, user_id: UserId) -> Result<Option<UserStatus>, StatusReadError> {
        let mut connection = tokio::time::timeout(ACQUIRE_TIMEOUT, self.pool.acquire())
            .await
            .map_err(|_| StatusReadError::Timeout)?
            .map_err(map_sqlx_error)?;

        // `SET LOCAL` needs a transaction; it scopes the statement bound to this
        // read and leaves the pooled connection's timeout untouched.
        let mut transaction = connection.begin().await.map_err(map_sqlx_error)?;
        sqlx::query(STATEMENT_TIMEOUT_SQL)
            .execute(&mut *transaction)
            .await
            .map_err(map_sqlx_error)?;

        let status = tokio::time::timeout(
            STATEMENT_TIMEOUT,
            sqlx::query_scalar::<_, String>("SELECT status FROM users WHERE id = $1")
                .bind(user_id.as_uuid())
                .fetch_optional(&mut *transaction),
        )
        .await
        .map_err(|_| StatusReadError::Timeout)?
        .map_err(map_sqlx_error)?;

        // The read has no side effect; roll the transaction back so the
        // connection returns to the pool immediately.
        transaction.rollback().await.map_err(map_sqlx_error)?;

        match status {
            None => Ok(None),
            // A malformed status is unavailable, never `active` (reliability
            // design, failure-mode checklist).
            Some(raw) => UserStatus::parse(&raw)
                .map(Some)
                .map_err(|error| StatusReadError::Malformed(error.to_string())),
        }
    }
}

fn map_sqlx_error(error: sqlx::Error) -> StatusReadError {
    match error {
        sqlx::Error::PoolTimedOut => StatusReadError::Timeout,
        other => StatusReadError::Unavailable(other.to_string()),
    }
}
