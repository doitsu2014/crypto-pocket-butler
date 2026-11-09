//! Postgres implementation of [`DeliveryControl`]: the durable wake flag and
//! the single-flight delivery lease (NFR2.5, NFR20.4).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_application::ports::{DeliveryControl, RepositoryError};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_error;

/// The singleton control row.
const CONTROL_ID: &str = "notification-delivery";

/// sqlx-backed wake flag and lease.
#[derive(Debug, Clone)]
pub struct PgDeliveryControl {
    pool: PgPool,
}

impl PgDeliveryControl {
    /// Creates the control over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DeliveryControl for PgDeliveryControl {
    async fn request_wake(&self, at: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE delivery_control SET wake_requested_at = $1 WHERE id = $2")
            .bind(at)
            .bind(CONTROL_ID)
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("set the delivery wake flag", error))?;
        Ok(())
    }

    async fn wake_requested(&self) -> Result<bool, RepositoryError> {
        let requested: Option<bool> = sqlx::query_scalar(
            "SELECT wake_requested_at IS NOT NULL FROM delivery_control WHERE id = $1",
        )
        .bind(CONTROL_ID)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read the delivery wake flag", error))?;
        Ok(requested.unwrap_or(false))
    }

    async fn clear_wake(&self) -> Result<(), RepositoryError> {
        sqlx::query("UPDATE delivery_control SET wake_requested_at = NULL WHERE id = $1")
            .bind(CONTROL_ID)
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("clear the delivery wake flag", error))?;
        Ok(())
    }

    async fn acquire_lease(
        &self,
        owner: Uuid,
        now: DateTime<Utc>,
        ttl_seconds: i64,
    ) -> Result<bool, RepositoryError> {
        let expires_at = now + chrono::Duration::seconds(ttl_seconds);
        let acquired: Option<String> = sqlx::query_scalar(
            "UPDATE delivery_control
             SET lease_owner = $1, lease_expires_at = $2, emails_sent = 0
             WHERE id = $3
               AND (
                   lease_owner IS NULL
                   OR lease_expires_at IS NULL
                   OR lease_expires_at <= $4
               )
             RETURNING id",
        )
        .bind(owner)
        .bind(expires_at)
        .bind(CONTROL_ID)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("acquire the delivery lease", error))?;
        Ok(acquired.is_some())
    }

    async fn reserve_email(&self, owner: Uuid, cap: u32) -> Result<bool, RepositoryError> {
        // One statement, so two concurrent senders can never both take the last
        // slot in the run's allowance.
        let reserved: Option<i32> = sqlx::query_scalar(
            "UPDATE delivery_control
             SET emails_sent = emails_sent + 1
             WHERE id = $3 AND lease_owner = $1 AND emails_sent < $2
             RETURNING emails_sent",
        )
        .bind(owner)
        .bind(i32::try_from(cap).unwrap_or(i32::MAX))
        .bind(CONTROL_ID)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("reserve a provider call", error))?;
        Ok(reserved.is_some())
    }

    async fn release_lease(&self, owner: Uuid) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE delivery_control
             SET lease_owner = NULL, lease_expires_at = NULL, last_completed_at = now()
             WHERE id = $1 AND lease_owner = $2",
        )
        .bind(CONTROL_ID)
        .bind(owner)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("release the delivery lease", error))?;
        Ok(())
    }
}
