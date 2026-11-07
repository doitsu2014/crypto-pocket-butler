//! Postgres implementation of [`ExportRepository`]. The claim is one atomic
//! `UPDATE ... FOR UPDATE SKIP LOCKED`, so two workers cannot execute the same
//! record.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use identity_application::ports::{ExportRepository, RepositoryError};
use identity_domain::{
    pending_contributions, ContributionState, Contributions, Contributor, ExportRequest,
    ExportRequestId, ExportStatus, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const EXPORT_COLUMNS: &str = "id, user_id, status, progress, contributions, requested_at, \
     assembly_deadline, file_reference, download_expires_at, failure_reason, completed_at, \
     lease_owner, lease_until";

/// sqlx-backed export requests.
#[derive(Debug, Clone)]
pub struct PgExportRepository {
    pool: PgPool,
}

impl PgExportRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ExportRow {
    id: Uuid,
    user_id: Uuid,
    status: String,
    progress: i16,
    contributions: serde_json::Value,
    requested_at: DateTime<Utc>,
    assembly_deadline: DateTime<Utc>,
    file_reference: Option<String>,
    download_expires_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    completed_at: Option<DateTime<Utc>>,
    lease_owner: Option<String>,
    lease_until: Option<DateTime<Utc>>,
}

fn contributions_to_json(contributions: &Contributions) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for contributor in Contributor::ALL {
        let state = contributions
            .get(&contributor)
            .copied()
            .unwrap_or(ContributionState::Pending);
        object.insert(
            contributor.as_str().to_string(),
            serde_json::Value::String(state.as_str().to_string()),
        );
    }
    serde_json::Value::Object(object)
}

fn contributions_from_json(value: &serde_json::Value) -> Result<Contributions, RepositoryError> {
    let mut contributions = pending_contributions();
    let object = value
        .as_object()
        .ok_or_else(|| invalid_data("read export", "contributions is not an object"))?;
    for (key, raw) in object {
        let contributor =
            Contributor::parse(key).map_err(|error| invalid_data("read export", error))?;
        let state = raw
            .as_str()
            .ok_or_else(|| invalid_data("read export", "a contribution is not a string"))?;
        contributions.insert(
            contributor,
            ContributionState::parse(state).map_err(|error| invalid_data("read export", error))?,
        );
    }
    Ok(contributions)
}

impl ExportRow {
    fn into_domain(self) -> Result<ExportRequest, RepositoryError> {
        let progress =
            u8::try_from(self.progress).map_err(|error| invalid_data("read export", error))?;
        Ok(ExportRequest::from_stored(
            ExportRequestId::new(self.id),
            UserId::new(self.user_id),
            ExportStatus::parse(&self.status)
                .map_err(|error| invalid_data("read export", error))?,
            progress,
            contributions_from_json(&self.contributions)?,
            self.requested_at,
            self.assembly_deadline,
            self.file_reference,
            self.download_expires_at,
            self.failure_reason,
            self.completed_at,
            self.lease_owner,
            self.lease_until,
        ))
    }
}

#[async_trait]
impl ExportRepository for PgExportRepository {
    async fn insert(&self, export: &ExportRequest) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO export_requests
                 (id, user_id, status, progress, contributions, requested_at, assembly_deadline,
                  file_reference, download_expires_at, failure_reason, completed_at, lease_owner,
                  lease_until)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(export.id().as_uuid())
        .bind(export.user_id().as_uuid())
        .bind(export.status().as_str())
        .bind(i16::from(export.progress()))
        .bind(contributions_to_json(export.contributions()))
        .bind(export.requested_at())
        .bind(export.assembly_deadline())
        .bind(export.file_reference())
        .bind(export.download_expires_at())
        .bind(export.failure_reason())
        .bind(export.completed_at())
        .bind(export.lease_owner())
        .bind(export.lease_until())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert export", error))?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: ExportRequestId,
    ) -> Result<Option<ExportRequest>, RepositoryError> {
        let row = sqlx::query_as::<_, ExportRow>(&format!(
            "SELECT {EXPORT_COLUMNS} FROM export_requests WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find export", error))?;
        row.map(ExportRow::into_domain).transpose()
    }

    async fn update(&self, export: &ExportRequest) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE export_requests
             SET status = $2, progress = $3, contributions = $4, file_reference = $5,
                 download_expires_at = $6, failure_reason = $7, completed_at = $8,
                 lease_owner = $9, lease_until = $10
             WHERE id = $1",
        )
        .bind(export.id().as_uuid())
        .bind(export.status().as_str())
        .bind(i16::from(export.progress()))
        .bind(contributions_to_json(export.contributions()))
        .bind(export.file_reference())
        .bind(export.download_expires_at())
        .bind(export.failure_reason())
        .bind(export.completed_at())
        .bind(export.lease_owner())
        .bind(export.lease_until())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update export", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "export '{}' does not exist",
                export.id()
            )));
        }
        Ok(())
    }

    async fn find_active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<ExportRequest>, RepositoryError> {
        let row = sqlx::query_as::<_, ExportRow>(&format!(
            "SELECT {EXPORT_COLUMNS} FROM export_requests
             WHERE user_id = $1 AND status IN ('pending', 'building', 'ready')
             ORDER BY requested_at DESC
             LIMIT 1"
        ))
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find active export", error))?;
        row.map(ExportRow::into_domain).transpose()
    }

    async fn claim_next(
        &self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<Option<ExportRequest>, RepositoryError> {
        let row = sqlx::query_as::<_, ExportRow>(&format!(
            "UPDATE export_requests
             SET lease_owner = $1, lease_until = $2,
                 status = CASE WHEN status = 'pending' THEN 'building' ELSE status END
             WHERE id = (
                 SELECT id FROM export_requests
                 WHERE status IN ('pending', 'building')
                   AND (lease_until IS NULL OR lease_until <= $3)
                 ORDER BY requested_at
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1
             )
             AND (lease_until IS NULL OR lease_until <= $3)
             RETURNING {EXPORT_COLUMNS}"
        ))
        .bind(worker)
        .bind(now + lease)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("claim export", error))?;
        row.map(ExportRow::into_domain).transpose()
    }

    async fn list_expired_ready(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ExportRequest>, RepositoryError> {
        let rows = sqlx::query_as::<_, ExportRow>(&format!(
            "SELECT {EXPORT_COLUMNS} FROM export_requests
             WHERE status = 'ready' AND download_expires_at <= $1"
        ))
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list expired exports", error))?;
        rows.into_iter().map(ExportRow::into_domain).collect()
    }
}
