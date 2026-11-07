//! Postgres implementation of [`SyncRunRepository`], including the fenced
//! snapshot write.

use account_application::ports::{RepositoryError, SyncRunRepository};
use account_domain::{
    AccountId, ErrorCategory, HoldingSnapshot, SyncRun, SyncRunId, SyncRunStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, account_id, started_at, lease_until, finished_at, status, \
     error_category, error_message, retries";

/// sqlx-backed sync runs.
#[derive(Debug, Clone)]
pub struct PgSyncRunRepository {
    pool: PgPool,
}

impl PgSyncRunRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RunRow {
    id: Uuid,
    account_id: Uuid,
    started_at: DateTime<Utc>,
    lease_until: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    status: String,
    error_category: Option<String>,
    error_message: Option<String>,
    retries: i32,
}

impl RunRow {
    fn into_domain(self) -> Result<SyncRun, RepositoryError> {
        let category = match self.error_category {
            Some(category) => Some(
                ErrorCategory::parse(&category)
                    .map_err(|error| invalid_data("read sync run", error))?,
            ),
            None => None,
        };
        Ok(SyncRun::from_stored(
            SyncRunId::new(self.id),
            AccountId::new(self.account_id),
            self.started_at,
            self.lease_until,
            self.finished_at,
            SyncRunStatus::parse(&self.status)
                .map_err(|error| invalid_data("read sync run", error))?,
            category,
            self.error_message,
            self.retries as u32,
        ))
    }
}

#[async_trait]
impl SyncRunRepository for PgSyncRunRepository {
    async fn claim(&self, run: &SyncRun) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin run claim", error))?;
        // BR4.2: an expired lease is failed before the claim so it never blocks
        // a later run.
        sqlx::query(
            "UPDATE sync_runs
             SET status = 'failed', finished_at = $2, error_category = COALESCE(error_category, 'data'),
                 error_message = COALESCE(error_message, 'lease expired')
             WHERE account_id = $1 AND status = 'running' AND lease_until <= $2",
        )
        .bind(run.account_id().as_uuid())
        .bind(run.started_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("expire overdue run", error))?;
        let insert = sqlx::query(
            "INSERT INTO sync_runs
                 (id, account_id, started_at, lease_until, finished_at, status, error_category,
                  error_message, retries)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(run.id().as_uuid())
        .bind(run.account_id().as_uuid())
        .bind(run.started_at())
        .bind(run.lease_until())
        .bind(run.finished_at())
        .bind(run.status().as_str())
        .bind(run.error_category().map(ErrorCategory::as_str))
        .bind(run.error_message())
        .bind(i32::try_from(run.retries()).unwrap_or(i32::MAX))
        .execute(&mut *transaction)
        .await;
        match insert {
            Ok(_) => transaction
                .commit()
                .await
                .map_err(|error| map_sqlx_error("commit run claim", error)),
            Err(error) => {
                let _ = transaction.rollback().await;
                Err(map_sqlx_error("claim sync run", error))
            }
        }
    }

    async fn find_by_id(&self, id: SyncRunId) -> Result<Option<SyncRun>, RepositoryError> {
        let row =
            sqlx::query_as::<_, RunRow>(&format!("SELECT {COLUMNS} FROM sync_runs WHERE id = $1"))
                .bind(id.as_uuid())
                .fetch_optional(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("find sync run", error))?;
        row.map(RunRow::into_domain).transpose()
    }

    async fn running_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<SyncRun>, RepositoryError> {
        let row = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {COLUMNS} FROM sync_runs
             WHERE account_id = $1 AND status = 'running'
             ORDER BY started_at DESC LIMIT 1"
        ))
        .bind(account_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find running sync run", error))?;
        row.map(RunRow::into_domain).transpose()
    }

    async fn list_running(&self, now: DateTime<Utc>) -> Result<Vec<SyncRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {COLUMNS} FROM sync_runs
             WHERE status = 'running' AND lease_until > $1
             ORDER BY started_at"
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list running sync runs", error))?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }

    async fn renew(
        &self,
        id: SyncRunId,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let affected = sqlx::query(
            "UPDATE sync_runs SET lease_until = $2
             WHERE id = $1 AND status = 'running' AND lease_until > $3",
        )
        .bind(id.as_uuid())
        .bind(lease_until)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("renew sync run", error))?
        .rows_affected();
        Ok(affected > 0)
    }

    async fn finish(&self, run: &SyncRun) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE sync_runs
             SET status = $2, finished_at = $3, error_category = $4, error_message = $5, retries = $6
             WHERE id = $1 AND status = 'running'",
        )
        .bind(run.id().as_uuid())
        .bind(run.status().as_str())
        .bind(run.finished_at())
        .bind(run.error_category().map(ErrorCategory::as_str))
        .bind(run.error_message())
        .bind(i32::try_from(run.retries()).unwrap_or(i32::MAX))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("finish sync run", error))?;
        Ok(())
    }

    async fn write_snapshot(
        &self,
        run: &SyncRun,
        holdings: &[HoldingSnapshot],
        last_synced_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin snapshot write", error))?;
        let live: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM sync_runs
             WHERE id = $1 AND status = 'running' AND lease_until > $2
             FOR UPDATE",
        )
        .bind(run.id().as_uuid())
        .bind(now)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("fence snapshot write", error))?;
        if live.is_none() {
            let _ = transaction.rollback().await;
            return Err(RepositoryError::Conflict(
                "the run no longer owns the lease".to_string(),
            ));
        }
        for holding in holdings {
            sqlx::query(
                "INSERT INTO holding_snapshots
                     (id, sync_run_id, account_id, observed_token, asset_id, quantity, observed_at,
                      spam)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(holding.id().as_uuid())
            .bind(holding.sync_run_id().as_uuid())
            .bind(holding.account_id().as_uuid())
            .bind(holding.observed_token())
            .bind(holding.asset_id())
            .bind(holding.quantity())
            .bind(holding.observed_at())
            .bind(holding.spam())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("insert holding snapshot", error))?;
        }
        sqlx::query("UPDATE accounts SET last_synced_at = $2, updated_at = $2 WHERE id = $1")
            .bind(run.account_id().as_uuid())
            .bind(last_synced_at)
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("stamp account sync time", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit snapshot write", error))
    }

    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<SyncRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "UPDATE sync_runs
             SET status = 'failed', finished_at = $1, error_category = 'data',
                 error_message = 'lease expired'
             WHERE status = 'running' AND lease_until <= $1
             RETURNING {COLUMNS}"
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("sweep expired sync runs", error))?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }
}
