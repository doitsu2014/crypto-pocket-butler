//! Postgres implementation of [`ReconcileLease`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{ReconcileLease, RepositoryError};
use sqlx::PgPool;

use super::map_sqlx_error;

const LEASE_ID: &str = "ledger-reconcile";

/// sqlx-backed single-row reconcile lease.
#[derive(Debug, Clone)]
pub struct PgReconcileLease {
    pool: PgPool,
}

impl PgReconcileLease {
    /// Creates the lease over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ReconcileLease for PgReconcileLease {
    async fn acquire(
        &self,
        now: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "UPDATE reconcile_leases
             SET leased_until = $2
             WHERE id = $1 AND (leased_until IS NULL OR leased_until <= $3)",
        )
        .bind(LEASE_ID)
        .bind(until)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("acquire reconcile lease", error))?;
        Ok(result.rows_affected() > 0)
    }

    async fn release(&self, now: DateTime<Utc>) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE reconcile_leases
             SET leased_until = NULL, last_completed_at = $2
             WHERE id = $1",
        )
        .bind(LEASE_ID)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("release reconcile lease", error))?;
        Ok(())
    }

    async fn last_completed(&self) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
            "SELECT last_completed_at FROM reconcile_leases WHERE id = $1",
        )
        .bind(LEASE_ID)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read last reconcile time", error))
    }
}
