//! Postgres implementation of [`AnonymisationRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_application::ports::{AnonymisationRepository, RepositoryError};
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed idempotency record for completed anonymisations.
#[derive(Debug, Clone)]
pub struct PgAnonymisationRepository {
    pool: PgPool,
}

impl PgAnonymisationRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AnonymisationRepository for PgAnonymisationRepository {
    async fn find(&self, idempotency_key: &str) -> Result<Option<String>, RepositoryError> {
        sqlx::query_scalar::<_, String>(
            "SELECT anonymised_ref FROM anonymisation_requests WHERE idempotency_key = $1",
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read anonymisation request", error))
    }

    async fn record(
        &self,
        idempotency_key: &str,
        anonymised_ref: &str,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO anonymisation_requests (idempotency_key, anonymised_ref, completed_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (idempotency_key) DO NOTHING",
        )
        .bind(idempotency_key)
        .bind(anonymised_ref)
        .bind(at)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("record anonymisation request", error))?;
        Ok(())
    }
}
