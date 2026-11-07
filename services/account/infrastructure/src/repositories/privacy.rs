//! Postgres implementation of [`PrivacyRepository`].

use account_application::ports::{AnonymisationRecord, PrivacyRepository, RepositoryError};
use account_domain::UserId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use super::map_sqlx_error;

/// sqlx-backed privacy ledger.
#[derive(Debug, Clone)]
pub struct PgPrivacyRepository {
    pool: PgPool,
}

impl PgPrivacyRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AnonymisationRow {
    idempotency_key: String,
    retained_categories: serde_json::Value,
    completed_at: DateTime<Utc>,
}

#[async_trait]
impl PrivacyRepository for PgPrivacyRepository {
    async fn find_anonymisation(
        &self,
        user_id: UserId,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, AnonymisationRow>(
            "SELECT idempotency_key, retained_categories, completed_at
             FROM privacy_anonymisations WHERE user_id = $1",
        )
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find privacy anonymisation", error))?;
        let Some(row) = row else {
            return Ok(None);
        };
        let retained_categories: Vec<String> = serde_json::from_value(row.retained_categories)
            .map_err(|error| {
                RepositoryError::InvalidData(format!("read privacy anonymisation: {error}"))
            })?;
        Ok(Some(AnonymisationRecord {
            user_id,
            idempotency_key: row.idempotency_key,
            retained_categories,
            completed_at: row.completed_at,
        }))
    }

    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError> {
        let categories = serde_json::to_value(&record.retained_categories).map_err(|error| {
            RepositoryError::InvalidData(format!("store privacy anonymisation: {error}"))
        })?;
        sqlx::query(
            "INSERT INTO privacy_anonymisations
                 (user_id, idempotency_key, retained_categories, completed_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (user_id) DO NOTHING",
        )
        .bind(record.user_id.as_uuid())
        .bind(&record.idempotency_key)
        .bind(categories)
        .bind(record.completed_at)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("record privacy anonymisation", error))?;
        Ok(())
    }
}
