//! Postgres implementation of [`ExportRepository`].

use async_trait::async_trait;
use audit_application::ports::{ExportRepository, RepositoryError};
use audit_domain::{AuditExport, ExportFilter, ExportOutcome};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use super::{invalid_data, map_sqlx_error};

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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FilterDocument {
    action_prefixes: Vec<String>,
    target: Option<String>,
    sensitivity: Option<String>,
    recorded_from: DateTime<Utc>,
    recorded_to: DateTime<Utc>,
    occurred_from: Option<DateTime<Utc>>,
    occurred_to: Option<DateTime<Utc>>,
}

impl From<&ExportFilter> for FilterDocument {
    fn from(filter: &ExportFilter) -> Self {
        Self {
            action_prefixes: filter.action_prefixes.clone(),
            target: filter.target.clone(),
            sensitivity: filter.sensitivity.map(|value| value.as_str().to_string()),
            recorded_from: filter.recorded_from,
            recorded_to: filter.recorded_to,
            occurred_from: filter.occurred_from,
            occurred_to: filter.occurred_to,
        }
    }
}

#[async_trait]
impl ExportRepository for PgExportRepository {
    async fn insert(&self, export: &AuditExport) -> Result<(), RepositoryError> {
        let filter = serde_json::to_value(FilterDocument::from(export.filter()))
            .map_err(|error| invalid_data("write export filter", error))?;
        sqlx::query(
            "INSERT INTO audit_exports
                 (id, requested_by, filter, requested_at, completed_at, row_count, state,
                  outcome, content_sha256, failure_reason)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(export.id().as_uuid())
        .bind(export.requested_by().as_uuid())
        .bind(filter)
        .bind(export.requested_at())
        .bind(export.completed_at())
        .bind(i32::try_from(export.row_count()).unwrap_or(i32::MAX))
        .bind(export.state().as_str())
        .bind(export.outcome().map(ExportOutcome::as_str))
        .bind(export.content_sha256())
        .bind(export.failure_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert audit export", error))?;
        Ok(())
    }

    async fn update(&self, export: &AuditExport) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE audit_exports
             SET state = $2, completed_at = $3, row_count = $4, outcome = $5,
                 content_sha256 = $6, failure_reason = $7
             WHERE id = $1",
        )
        .bind(export.id().as_uuid())
        .bind(export.state().as_str())
        .bind(export.completed_at())
        .bind(i32::try_from(export.row_count()).unwrap_or(i32::MAX))
        .bind(export.outcome().map(ExportOutcome::as_str))
        .bind(export.content_sha256())
        .bind(export.failure_reason())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update audit export", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "audit export '{}' does not exist",
                export.id()
            )));
        }
        Ok(())
    }

    async fn mark_stale_failed(
        &self,
        before: DateTime<Utc>,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError> {
        let affected = sqlx::query(
            "UPDATE audit_exports
             SET state = 'failed', outcome = 'failed', completed_at = $2, failure_reason = $3
             WHERE state = 'requested' AND requested_at < $1",
        )
        .bind(before)
        .bind(at)
        .bind(audit_domain::export::REPLICA_LOST_REASON)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("recover stale exports", error))?
        .rows_affected();
        Ok(affected)
    }
}
