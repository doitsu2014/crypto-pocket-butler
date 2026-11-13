//! Postgres implementation of [`DeletionRepository`]. The claim is one atomic
//! `UPDATE ... FOR UPDATE SKIP LOCKED`, so a retry or a second worker resumes the
//! same record without double execution.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use identity_application::ports::{DeletionRepository, RepositoryError};
use identity_domain::{
    DeletionProgress, DeletionRecord, DeletionRequest, DeletionRequestId, DeletionStatus,
    RetainedCategory, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const DELETION_COLUMNS: &str = "id, user_id, status, progress, retained_categories, requested_at, \
     completed_at, failure_reason, lease_owner, lease_until, recoverable_until, cancelled_at, \
     erased_at, attempt_count, next_attempt_at";

/// sqlx-backed deletion requests.
#[derive(Debug, Clone)]
pub struct PgDeletionRepository {
    pool: PgPool,
}

impl PgDeletionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct DeletionRow {
    id: Uuid,
    user_id: Uuid,
    status: String,
    progress: serde_json::Value,
    retained_categories: serde_json::Value,
    requested_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    lease_owner: Option<String>,
    lease_until: Option<DateTime<Utc>>,
    recoverable_until: DateTime<Utc>,
    cancelled_at: Option<DateTime<Utc>>,
    erased_at: Option<DateTime<Utc>>,
    attempt_count: i32,
    next_attempt_at: DateTime<Utc>,
}

fn progress_to_json(progress: &DeletionProgress) -> serde_json::Value {
    let mut anonymised = serde_json::Map::new();
    for contributor in identity_domain::Contributor::ALL {
        anonymised.insert(
            contributor.as_str().to_string(),
            serde_json::Value::Bool(progress.is_anonymised(contributor)),
        );
    }
    serde_json::json!({
        "anonymised": anonymised,
        "authDeleted": progress.auth_deleted(),
        "identityAnonymised": progress.identity_anonymised(),
        "keyDestroyed": progress.key_destroyed(),
        "retained": retained_to_json(progress.retained()),
    })
}

fn progress_from_json(value: &serde_json::Value) -> Result<DeletionProgress, RepositoryError> {
    let mut progress = DeletionProgress::default();
    if let Some(anonymised) = value
        .get("anonymised")
        .and_then(serde_json::Value::as_object)
    {
        for (key, raw) in anonymised {
            if raw.as_bool().unwrap_or(false) {
                let contributor = identity_domain::Contributor::parse(key)
                    .map_err(|error| invalid_data("read deletion", error))?;
                progress.record_anonymised(contributor);
            }
        }
    }
    if let Some(anonymised) = value
        .get("identityAnonymised")
        .and_then(serde_json::Value::as_bool)
    {
        if anonymised {
            progress.record_identity_anonymised();
        }
    }
    if value
        .get("authDeleted")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        progress.record_auth_deleted();
    }
    if let Some(retained) = value.get("retained") {
        let categories = retained_from_json(retained)?;
        progress.record_retained(categories);
    }
    if value
        .get("keyDestroyed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        progress
            .record_key_destroyed()
            .map_err(|error| invalid_data("read deletion", error))?;
    }
    Ok(progress)
}

fn retained_to_json(categories: &[RetainedCategory]) -> serde_json::Value {
    serde_json::Value::Array(
        categories
            .iter()
            .map(|category| {
                serde_json::json!({
                    "category": category.category(),
                    "basis": category.basis(),
                })
            })
            .collect(),
    )
}

fn retained_from_json(value: &serde_json::Value) -> Result<Vec<RetainedCategory>, RepositoryError> {
    let array = value
        .as_array()
        .ok_or_else(|| invalid_data("read deletion", "retained categories is not an array"))?;
    let mut categories = Vec::with_capacity(array.len());
    for entry in array {
        let category = entry
            .get("category")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_data("read deletion", "a retained category has no label"))?;
        let basis = entry
            .get("basis")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| invalid_data("read deletion", "a retained category has no basis"))?;
        categories.push(
            RetainedCategory::new(category, basis)
                .map_err(|error| invalid_data("read deletion", error))?,
        );
    }
    Ok(categories)
}

impl DeletionRow {
    fn into_domain(self) -> Result<DeletionRequest, RepositoryError> {
        Ok(DeletionRequest::from_stored(DeletionRecord {
            id: DeletionRequestId::new(self.id),
            user_id: UserId::new(self.user_id),
            status: DeletionStatus::parse(&self.status)
                .map_err(|error| invalid_data("read deletion", error))?,
            progress: progress_from_json(&self.progress)?,
            requested_at: self.requested_at,
            retained_categories: retained_from_json(&self.retained_categories)?,
            completed_at: self.completed_at,
            failure_reason: self.failure_reason,
            lease_owner: self.lease_owner,
            lease_until: self.lease_until,
            recoverable_until: self.recoverable_until,
            cancelled_at: self.cancelled_at,
            erased_at: self.erased_at,
            attempt_count: u32::try_from(self.attempt_count.max(0)).unwrap_or(u32::MAX),
            next_attempt_at: self.next_attempt_at,
        }))
    }
}

#[async_trait]
impl DeletionRepository for PgDeletionRepository {
    async fn insert(&self, deletion: &DeletionRequest) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO deletion_requests
                 (id, user_id, status, progress, retained_categories, requested_at, completed_at,
                  failure_reason, lease_owner, lease_until, recoverable_until, cancelled_at,
                  erased_at, attempt_count, next_attempt_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)",
        )
        .bind(deletion.id().as_uuid())
        .bind(deletion.user_id().as_uuid())
        .bind(deletion.status().as_str())
        .bind(progress_to_json(deletion.progress()))
        .bind(retained_to_json(deletion.retained_categories()))
        .bind(deletion.requested_at())
        .bind(deletion.completed_at())
        .bind(deletion.failure_reason())
        .bind(deletion.lease_owner())
        .bind(deletion.lease_until())
        .bind(deletion.recoverable_until())
        .bind(deletion.cancelled_at())
        .bind(deletion.erased_at())
        .bind(i32::try_from(deletion.attempt_count()).unwrap_or(i32::MAX))
        .bind(deletion.next_attempt_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert deletion", error))?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: DeletionRequestId,
    ) -> Result<Option<DeletionRequest>, RepositoryError> {
        let row = sqlx::query_as::<_, DeletionRow>(&format!(
            "SELECT {DELETION_COLUMNS} FROM deletion_requests WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find deletion", error))?;
        row.map(DeletionRow::into_domain).transpose()
    }

    async fn update(&self, deletion: &DeletionRequest) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE deletion_requests
             SET status = $2, progress = $3, retained_categories = $4, completed_at = $5,
                 failure_reason = $6, lease_owner = $7, lease_until = $8, recoverable_until = $9,
                 cancelled_at = $10, erased_at = $11, attempt_count = $12, next_attempt_at = $13
             WHERE id = $1",
        )
        .bind(deletion.id().as_uuid())
        .bind(deletion.status().as_str())
        .bind(progress_to_json(deletion.progress()))
        .bind(retained_to_json(deletion.retained_categories()))
        .bind(deletion.completed_at())
        .bind(deletion.failure_reason())
        .bind(deletion.lease_owner())
        .bind(deletion.lease_until())
        .bind(deletion.recoverable_until())
        .bind(deletion.cancelled_at())
        .bind(deletion.erased_at())
        .bind(i32::try_from(deletion.attempt_count()).unwrap_or(i32::MAX))
        .bind(deletion.next_attempt_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update deletion", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "deletion '{}' does not exist",
                deletion.id()
            )));
        }
        Ok(())
    }

    async fn find_active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<DeletionRequest>, RepositoryError> {
        let row = sqlx::query_as::<_, DeletionRow>(&format!(
            "SELECT {DELETION_COLUMNS} FROM deletion_requests
             WHERE user_id = $1 AND cancelled_at IS NULL AND erased_at IS NULL
             ORDER BY requested_at DESC
             LIMIT 1"
        ))
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find open deletion", error))?;
        row.map(DeletionRow::into_domain).transpose()
    }

    async fn claim_next(
        &self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<Option<DeletionRequest>, RepositoryError> {
        // W4: only an open request past its recovery window, due for a retry,
        // and not under a live lease is claimed. The claim commits before any
        // cascade work, so a long cascade never holds the row lock.
        let row = sqlx::query_as::<_, DeletionRow>(&format!(
            "UPDATE deletion_requests
             SET lease_owner = $1, lease_until = $2, status = 'in_progress'
             WHERE id = (
                 SELECT id FROM deletion_requests
                 WHERE cancelled_at IS NULL AND erased_at IS NULL
                   AND status IN ('requested', 'in_progress', 'failed')
                   AND next_attempt_at <= $3
                   AND recoverable_until <= $3
                   AND (lease_until IS NULL OR lease_until <= $3)
                 ORDER BY recoverable_until
                 FOR UPDATE SKIP LOCKED
                 LIMIT 1
             )
             AND (lease_until IS NULL OR lease_until <= $3)
             RETURNING {DELETION_COLUMNS}"
        ))
        .bind(worker)
        .bind(now + lease)
        .bind(now)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("claim deletion", error))?;
        row.map(DeletionRow::into_domain).transpose()
    }
}
