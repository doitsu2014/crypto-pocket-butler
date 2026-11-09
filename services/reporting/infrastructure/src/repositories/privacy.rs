//! The privacy anonymisation ledger repository.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_application::ports::{AnonymisationRecord, PrivacyRepository, RepositoryError};
use reporting_domain::UserId;
use sqlx::PgPool;

use super::map_error;

/// sqlx-backed anonymisation ledger.
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
struct RecordRow {
    anonymised_ref: uuid::Uuid,
    idempotency_key: String,
    retained_categories: serde_json::Value,
    completed_at: DateTime<Utc>,
}

#[async_trait]
impl PrivacyRepository for PgPrivacyRepository {
    async fn find_anonymisation(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, RecordRow>(
            "SELECT anonymised_ref, idempotency_key, retained_categories, completed_at
             FROM privacy_anonymisations
             WHERE idempotency_key = $1",
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(row.map(|row| {
            let categories: Vec<String> =
                serde_json::from_value(row.retained_categories).unwrap_or_default();
            AnonymisationRecord {
                anonymised_ref: UserId::new(row.anonymised_ref),
                idempotency_key: row.idempotency_key,
                retained_categories: categories,
                completed_at: row.completed_at,
            }
        }))
    }

    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError> {
        let categories = serde_json::to_value(&record.retained_categories)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        sqlx::query(
            "INSERT INTO privacy_anonymisations
                 (anonymised_ref, idempotency_key, retained_categories, completed_at)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (idempotency_key) DO NOTHING",
        )
        .bind(record.anonymised_ref.as_uuid())
        .bind(&record.idempotency_key)
        .bind(categories)
        .bind(record.completed_at)
        .execute(&self.pool)
        .await
        .map_err(map_error)?;
        Ok(())
    }
}
