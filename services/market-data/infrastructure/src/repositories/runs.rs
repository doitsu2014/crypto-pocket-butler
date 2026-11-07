//! Postgres implementation of [`RefreshRunRepository`], including the per-job
//! single-flight claim.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use market_data_application::ports::{ClaimOutcome, RefreshRunRepository, RepositoryError};
use market_data_domain::{ErrorCategory, RefreshJob, RefreshRun, RefreshRunId, RunStatus};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const COLUMNS: &str = "id, job, started_at, lease_until, finished_at, status, requested_count, \
     priced_count, unpriced_count, error_category, error_message";

/// sqlx-backed refresh runs.
#[derive(Debug, Clone)]
pub struct PgRefreshRunRepository {
    pool: PgPool,
}

impl PgRefreshRunRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct RunRow {
    id: Uuid,
    job: String,
    started_at: DateTime<Utc>,
    lease_until: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    status: String,
    requested_count: i32,
    priced_count: i32,
    unpriced_count: i32,
    error_category: Option<String>,
    error_message: Option<String>,
}

impl RunRow {
    fn into_domain(self) -> Result<RefreshRun, RepositoryError> {
        let category = match self.error_category {
            Some(category) => Some(
                ErrorCategory::parse(&category)
                    .map_err(|error| invalid_data("read refresh run", error))?,
            ),
            None => None,
        };
        Ok(RefreshRun::from_stored(
            RefreshRunId::new(self.id),
            RefreshJob::parse(&self.job)
                .map_err(|error| invalid_data("read refresh run", error))?,
            self.started_at,
            self.lease_until,
            self.finished_at,
            RunStatus::parse(&self.status)
                .map_err(|error| invalid_data("read refresh run", error))?,
            u32::try_from(self.requested_count)
                .map_err(|error| invalid_data("read refresh run", error))?,
            u32::try_from(self.priced_count)
                .map_err(|error| invalid_data("read refresh run", error))?,
            u32::try_from(self.unpriced_count)
                .map_err(|error| invalid_data("read refresh run", error))?,
            category,
            self.error_message,
        ))
    }
}

#[async_trait]
impl RefreshRunRepository for PgRefreshRunRepository {
    async fn claim(&self, run: &RefreshRun) -> Result<ClaimOutcome, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin run claim", error))?;
        // BR4.1: an expired lease is failed before the claim so it never blocks
        // a later run.
        sqlx::query(
            "UPDATE refresh_runs
             SET status = 'failed', finished_at = $2,
                 error_category = COALESCE(error_category, 'data'),
                 error_message = COALESCE(error_message, 'lease expired')
             WHERE job = $1 AND status = 'running' AND lease_until <= $2",
        )
        .bind(run.job().as_str())
        .bind(run.started_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("expire overdue run", error))?;

        let active = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {COLUMNS} FROM refresh_runs
             WHERE job = $1 AND status = 'running'
             FOR UPDATE"
        ))
        .bind(run.job().as_str())
        .fetch_optional(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("find active run", error))?;
        if let Some(active) = active {
            transaction
                .commit()
                .await
                .map_err(|error| map_sqlx_error("commit run claim", error))?;
            return Ok(ClaimOutcome::Held(active.into_domain()?));
        }

        let insert = sqlx::query(
            "INSERT INTO refresh_runs
                 (id, job, started_at, lease_until, finished_at, status, requested_count,
                  priced_count, unpriced_count, error_category, error_message)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(run.id().as_uuid())
        .bind(run.job().as_str())
        .bind(run.started_at())
        .bind(run.lease_until())
        .bind(run.finished_at())
        .bind(run.status().as_str())
        .bind(i32::try_from(run.requested_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(run.priced_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(run.unpriced_count()).unwrap_or(i32::MAX))
        .bind(run.error_category().map(ErrorCategory::as_str))
        .bind(run.error_message())
        .execute(&mut *transaction)
        .await;
        match insert {
            Ok(_) => transaction
                .commit()
                .await
                .map_err(|error| map_sqlx_error("commit run claim", error))
                .map(|()| ClaimOutcome::Claimed),
            Err(error) => {
                let _ = transaction.rollback().await;
                // A concurrent claim won the unique index. Report the winner.
                let active = self.active_for_job(run.job(), run.started_at()).await?;
                match active {
                    Some(active) => Ok(ClaimOutcome::Held(active)),
                    None => Err(map_sqlx_error("claim refresh run", error)),
                }
            }
        }
    }

    async fn find_by_id(&self, id: RefreshRunId) -> Result<Option<RefreshRun>, RepositoryError> {
        let row = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {COLUMNS} FROM refresh_runs WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find refresh run", error))?;
        row.map(RunRow::into_domain).transpose()
    }

    async fn active_for_job(
        &self,
        job: RefreshJob,
        now: DateTime<Utc>,
    ) -> Result<Option<RefreshRun>, RepositoryError> {
        let row = sqlx::query_as::<_, RunRow>(&format!(
            "SELECT {COLUMNS} FROM refresh_runs
             WHERE job = $1 AND status = 'running' AND lease_until > $2
             ORDER BY started_at DESC LIMIT 1"
        ))
        .bind(job.as_str())
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find active refresh run", error))?;
        row.map(RunRow::into_domain).transpose()
    }

    async fn renew(
        &self,
        id: RefreshRunId,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let affected = sqlx::query(
            "UPDATE refresh_runs SET lease_until = $2
             WHERE id = $1 AND status = 'running' AND lease_until > $3",
        )
        .bind(id.as_uuid())
        .bind(lease_until)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("renew refresh run", error))?
        .rows_affected();
        Ok(affected > 0)
    }

    async fn finish(&self, run: &RefreshRun) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE refresh_runs
             SET status = $2, finished_at = $3, requested_count = $4, priced_count = $5,
                 unpriced_count = $6, error_category = $7, error_message = $8
             WHERE id = $1 AND status = 'running'",
        )
        .bind(run.id().as_uuid())
        .bind(run.status().as_str())
        .bind(run.finished_at())
        .bind(i32::try_from(run.requested_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(run.priced_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(run.unpriced_count()).unwrap_or(i32::MAX))
        .bind(run.error_category().map(ErrorCategory::as_str))
        .bind(run.error_message())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("finish refresh run", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(
                "the run is no longer running".to_string(),
            ));
        }
        Ok(())
    }

    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<RefreshRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(&format!(
            "UPDATE refresh_runs
             SET status = 'failed', finished_at = $1,
                 error_category = COALESCE(error_category, 'data'),
                 error_message = COALESCE(error_message, 'lease expired')
             WHERE status = 'running' AND lease_until <= $1
             RETURNING {COLUMNS}"
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("sweep expired refresh runs", error))?;
        rows.into_iter().map(RunRow::into_domain).collect()
    }
}
