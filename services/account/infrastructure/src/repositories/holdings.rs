//! Postgres implementation of [`HoldingRepository`].

use account_application::ports::{HoldingRepository, RepositoryError};
use account_domain::{AccountId, HoldingSnapshot, HoldingSnapshotId, SyncRunId, UserId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_error;

const COLUMNS: &str = "h.id, h.sync_run_id, h.account_id, h.observed_token, h.asset_id, \
     h.quantity, h.observed_at, h.spam";

/// sqlx-backed holding reads.
#[derive(Debug, Clone)]
pub struct PgHoldingRepository {
    pool: PgPool,
}

impl PgHoldingRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct HoldingRow {
    id: Uuid,
    sync_run_id: Uuid,
    account_id: Uuid,
    observed_token: String,
    asset_id: Option<String>,
    quantity: Decimal,
    observed_at: DateTime<Utc>,
    spam: bool,
}

impl HoldingRow {
    fn into_domain(self) -> HoldingSnapshot {
        HoldingSnapshot::from_stored(
            HoldingSnapshotId::new(self.id),
            SyncRunId::new(self.sync_run_id),
            AccountId::new(self.account_id),
            self.observed_token,
            self.asset_id,
            self.quantity,
            self.observed_at,
            self.spam,
        )
    }
}

#[async_trait]
impl HoldingRepository for PgHoldingRepository {
    async fn latest_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<HoldingSnapshot>, RepositoryError> {
        let rows = sqlx::query_as::<_, HoldingRow>(&format!(
            "SELECT {COLUMNS}
             FROM holding_snapshots h
             WHERE h.account_id = $1
               AND h.sync_run_id = (
                   SELECT r.id FROM sync_runs r
                   WHERE r.account_id = $1 AND r.status = 'succeeded'
                   ORDER BY r.finished_at DESC NULLS LAST, r.started_at DESC
                   LIMIT 1
               )
             ORDER BY h.observed_token"
        ))
        .bind(account_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read latest holdings", error))?;
        Ok(rows.into_iter().map(HoldingRow::into_domain).collect())
    }

    async fn snapshots_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<HoldingSnapshot>, RepositoryError> {
        let rows = sqlx::query_as::<_, HoldingRow>(&format!(
            "SELECT {COLUMNS}
             FROM holding_snapshots h
             JOIN accounts a ON a.id = h.account_id
             WHERE a.user_id = $1
             ORDER BY h.observed_at, h.id"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read user holdings", error))?;
        Ok(rows.into_iter().map(HoldingRow::into_domain).collect())
    }
}
