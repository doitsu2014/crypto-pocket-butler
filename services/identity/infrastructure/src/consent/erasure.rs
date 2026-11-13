//! The erasure adapter. It runs on a pool built from
//! `IDENTITY_RETENTION_DATABASE_URL` and connects as `identity_retention`, the
//! only role `0005` leaves with DELETE on the consent and preference tables. The
//! application role cannot reach this path, so the append-only lock holds
//! (BR4.4, NFR22.3).

use async_trait::async_trait;
use identity_application::ports::{ErasureStore, RepositoryError};
use identity_domain::UserId;
use sqlx::PgPool;

use crate::repositories::map_sqlx_error;

/// sqlx-backed erasure of the consent-scoped rows.
#[derive(Debug, Clone)]
pub struct PgRetentionStore {
    pool: PgPool,
}

impl PgRetentionStore {
    /// Creates the adapter over the retention role's pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ErasureStore for PgRetentionStore {
    async fn erase_consent(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin erasure", error))?;
        let removed = sqlx::query("DELETE FROM consent_records WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("erase consent records", error))?
            .rows_affected();
        sqlx::query("DELETE FROM user_preferences WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("erase preferences", error))?;
        sqlx::query("DELETE FROM consent_records_legacy_archive WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("erase archived consent", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit erasure", error))?;
        Ok(removed)
    }
}
