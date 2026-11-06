//! Postgres implementation of [`PrivacyDeletionRepository`].

use async_trait::async_trait;
use audit_application::ports::{PrivacyDeletionRepository, RepositoryError};
use audit_domain::{
    AuditEventId, DeletionRequestId, EventLabel, PrivacyDeletion, PrivacyDeletionId,
    PrivacyDeletionState, UserId,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const DELETION_COLUMNS: &str = "id, deletion_request_id, subject_user_id, requested_at, state, \
     completed_at, deleted_count, kept_count, kept_with_legal_basis, defects, failure_reason";

/// sqlx-backed privacy deletions.
#[derive(Debug, Clone)]
pub struct PgPrivacyDeletionRepository {
    pool: PgPool,
}

impl PgPrivacyDeletionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct DeletionRow {
    id: Uuid,
    deletion_request_id: Uuid,
    subject_user_id: Uuid,
    requested_at: DateTime<Utc>,
    state: String,
    completed_at: Option<DateTime<Utc>>,
    deleted_count: i32,
    kept_count: i32,
    kept_with_legal_basis: i32,
    defects: i32,
    failure_reason: Option<String>,
}

impl DeletionRow {
    fn into_domain(self) -> Result<PrivacyDeletion, RepositoryError> {
        let state = PrivacyDeletionState::parse(&self.state)
            .map_err(|error| invalid_data("read privacy deletion", error))?;
        Ok(PrivacyDeletion::from_stored(
            PrivacyDeletionId::new(self.id),
            DeletionRequestId::new(self.deletion_request_id),
            UserId::new(self.subject_user_id),
            self.requested_at,
            state,
            self.completed_at,
            i64::from(self.deleted_count),
            i64::from(self.kept_count),
            i64::from(self.kept_with_legal_basis),
            i64::from(self.defects),
            self.failure_reason,
        ))
    }
}

#[async_trait]
impl PrivacyDeletionRepository for PgPrivacyDeletionRepository {
    async fn insert(&self, deletion: &PrivacyDeletion) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO privacy_deletions
                 (id, deletion_request_id, subject_user_id, requested_at, state, completed_at,
                  deleted_count, kept_count, kept_with_legal_basis, defects, failure_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)",
        )
        .bind(deletion.id().as_uuid())
        .bind(deletion.deletion_request_id().as_uuid())
        .bind(deletion.subject_user_id().as_uuid())
        .bind(deletion.requested_at())
        .bind(deletion.state().as_str())
        .bind(deletion.completed_at())
        .bind(i32::try_from(deletion.deleted_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(deletion.kept_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(deletion.kept_with_legal_basis()).unwrap_or(i32::MAX))
        .bind(i32::try_from(deletion.defects()).unwrap_or(i32::MAX))
        .bind(deletion.failure_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert privacy deletion", error))?;
        Ok(())
    }

    async fn update(&self, deletion: &PrivacyDeletion) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE privacy_deletions
             SET state = $2, completed_at = $3, deleted_count = $4, kept_count = $5,
                 kept_with_legal_basis = $6, defects = $7, failure_reason = $8
             WHERE id = $1",
        )
        .bind(deletion.id().as_uuid())
        .bind(deletion.state().as_str())
        .bind(deletion.completed_at())
        .bind(i32::try_from(deletion.deleted_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(deletion.kept_count()).unwrap_or(i32::MAX))
        .bind(i32::try_from(deletion.kept_with_legal_basis()).unwrap_or(i32::MAX))
        .bind(i32::try_from(deletion.defects()).unwrap_or(i32::MAX))
        .bind(deletion.failure_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update privacy deletion", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "privacy deletion '{}' does not exist",
                deletion.id()
            )));
        }
        Ok(())
    }

    async fn find_by_request(
        &self,
        request: DeletionRequestId,
    ) -> Result<Option<PrivacyDeletion>, RepositoryError> {
        let row = sqlx::query_as::<_, DeletionRow>(&format!(
            "SELECT {DELETION_COLUMNS} FROM privacy_deletions WHERE deletion_request_id = $1"
        ))
        .bind(request.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find privacy deletion", error))?;
        row.map(DeletionRow::into_domain).transpose()
    }

    async fn next_requested(&self, limit: u32) -> Result<Vec<PrivacyDeletion>, RepositoryError> {
        let rows = sqlx::query_as::<_, DeletionRow>(&format!(
            "SELECT {DELETION_COLUMNS} FROM privacy_deletions
             WHERE state = 'requested'
             ORDER BY requested_at ASC
             LIMIT $1"
        ))
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list privacy deletions", error))?;
        rows.into_iter().map(DeletionRow::into_domain).collect()
    }

    async fn labels_for_subject(
        &self,
        subject: UserId,
        limit: u32,
    ) -> Result<Vec<EventLabel>, RepositoryError> {
        let rows = sqlx::query_as::<_, (Uuid, Option<String>)>(
            "SELECT id, legal_basis FROM audit_events
             WHERE actor_id = $1
             ORDER BY recorded_at ASC
             LIMIT $2",
        )
        .bind(subject.as_uuid())
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("scan subject events", error))?;
        Ok(rows
            .into_iter()
            .map(|(id, legal_basis)| EventLabel {
                id: AuditEventId::new(id),
                legal_basis,
            })
            .collect())
    }

    async fn delete_unlabelled_for_subject(&self, subject: UserId) -> Result<u64, RepositoryError> {
        let affected =
            sqlx::query("DELETE FROM audit_events WHERE actor_id = $1 AND legal_basis IS NULL")
                .bind(subject.as_uuid())
                .execute(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("delete subject events", error))?
                .rows_affected();
        Ok(affected)
    }
}
