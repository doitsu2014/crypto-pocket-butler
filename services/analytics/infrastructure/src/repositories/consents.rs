//! Postgres implementation of [`ConsentRepository`].

use analytics_application::ports::{ConsentRepository, RepositoryError};
use analytics_domain::ConsentCheck;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed consent-check store.
#[derive(Debug, Clone)]
pub struct PgConsentRepository {
    pool: PgPool,
}

impl PgConsentRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ConsentRepository for PgConsentRepository {
    async fn insert(&self, check: &ConsentCheck) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO consent_checks (id, purpose, granted, checked_at, user_ref)
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(check.id().as_uuid())
        .bind(check.purpose().as_str())
        .bind(check.granted())
        .bind(check.checked_at())
        .bind(check.user_ref())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert consent check", error))?;
        Ok(())
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM consent_checks WHERE id IN (
                 SELECT id FROM consent_checks WHERE checked_at < $1
                 ORDER BY checked_at LIMIT $2
             )",
        )
        .bind(cutoff)
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete consent checks", error))?;
        Ok(result.rows_affected())
    }

    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError> {
        let result = sqlx::query("DELETE FROM consent_checks WHERE user_ref = ANY($1)")
            .bind(user_refs.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("erase consent checks", error))?;
        Ok(result.rows_affected())
    }
}
