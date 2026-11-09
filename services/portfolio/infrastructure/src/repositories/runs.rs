//! The single-flight valuation-run lease repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{RepositoryError, RunRepository, RunStatus, ValuationRun};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

/// sqlx-backed valuation runs.
#[derive(Debug, Clone)]
pub struct PgRunRepository {
    pool: PgPool,
}

impl PgRunRepository {
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
    owner: String,
    started_at: DateTime<Utc>,
    lease_until: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    status: String,
    detail: Option<String>,
}

fn parse_run(row: RunRow) -> Result<ValuationRun, RepositoryError> {
    let status = match row.status.as_str() {
        "running" => RunStatus::Running,
        "succeeded" => RunStatus::Succeeded,
        "failed" => RunStatus::Failed,
        other => {
            return Err(RepositoryError::InvalidData(format!(
                "stored run status {other} is unknown"
            )))
        }
    };
    Ok(ValuationRun {
        id: row.id,
        job: row.job,
        owner: row.owner,
        started_at: row.started_at,
        lease_until: row.lease_until,
        finished_at: row.finished_at,
        status,
        detail: row.detail,
    })
}

const _: () = ();

#[async_trait]
impl RunRepository for PgRunRepository {
    async fn claim(&self, run: &ValuationRun) -> Result<bool, RepositoryError> {
        // An expired lease never blocks a later run.
        sqlx::query(
            "UPDATE valuation_runs
             SET status = 'failed', finished_at = $2, detail = 'lease expired'
             WHERE job = $1 AND status = 'running' AND lease_until <= $2",
        )
        .bind(&run.job)
        .bind(run.started_at)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        let result = sqlx::query(
            "INSERT INTO valuation_runs
                (id, job, owner, started_at, lease_until, finished_at, status, detail)
             VALUES ($1, $2, $3, $4, $5, NULL, 'running', NULL)
             ON CONFLICT DO NOTHING",
        )
        .bind(run.id)
        .bind(&run.job)
        .bind(&run.owner)
        .bind(run.started_at)
        .bind(run.lease_until)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn renew(
        &self,
        id: Uuid,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError> {
        let result = sqlx::query(
            "UPDATE valuation_runs
             SET lease_until = $2
             WHERE id = $1 AND status = 'running' AND lease_until > $3",
        )
        .bind(id)
        .bind(lease_until)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(result.rows_affected() == 1)
    }

    async fn finish(&self, run: &ValuationRun) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE valuation_runs
             SET status = $2, finished_at = $3, detail = $4
             WHERE id = $1 AND status = 'running'",
        )
        .bind(run.id)
        .bind(run.status.as_str())
        .bind(run.finished_at)
        .bind(&run.detail)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }

    async fn sweep_expired(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ValuationRun>, RepositoryError> {
        let rows = sqlx::query_as::<_, RunRow>(
            "UPDATE valuation_runs
             SET status = 'failed', finished_at = $1, detail = 'lease expired'
             WHERE status = 'running' AND lease_until <= $1
             RETURNING id, job, owner, started_at, lease_until, finished_at, status, detail",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(map_error)?;
        rows.into_iter().map(parse_run).collect()
    }
}
