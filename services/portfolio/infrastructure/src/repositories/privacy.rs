//! The privacy anonymisation ledger. It stores only the random opaque reference,
//! never the original user id, so a completed erasure leaves no subject behind.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use portfolio_application::ports::{AnonymisationRecord, PrivacyRepository, RepositoryError};
use portfolio_domain::UserId;
use sqlx::PgPool;
use uuid::Uuid;

use super::map_error;

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
    anonymised_ref: Uuid,
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
        let row = sqlx::query_as::<_, AnonymisationRow>(
            "SELECT anonymised_ref, idempotency_key, retained_categories, completed_at
             FROM privacy_anonymisations WHERE idempotency_key = $1",
        )
        .bind(idempotency_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_error)?;
        row.map(|row| {
            let retained: Vec<String> = serde_json::from_value(row.retained_categories)
                .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
            Ok(AnonymisationRecord {
                anonymised_ref: UserId::new(row.anonymised_ref),
                idempotency_key: row.idempotency_key,
                retained_categories: retained,
                completed_at: row.completed_at,
            })
        })
        .transpose()
    }

    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError> {
        let retained = serde_json::to_value(&record.retained_categories)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        sqlx::query(
            "INSERT INTO privacy_anonymisations
                (anonymised_ref, idempotency_key, retained_categories, completed_at)
             VALUES ($1, $2, $3::jsonb, $4)
             ON CONFLICT (idempotency_key) DO NOTHING",
        )
        .bind(record.anonymised_ref.as_uuid())
        .bind(&record.idempotency_key)
        .bind(retained)
        .bind(record.completed_at)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(map_error)
    }
}
