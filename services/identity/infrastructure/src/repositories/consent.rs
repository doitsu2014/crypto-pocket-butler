//! Postgres implementation of [`ConsentRepository`]. Records are insert-only.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{ConsentRepository, RepositoryError};
use identity_domain::{ConsentPurpose, ConsentRecord, ConsentRecordId, ConsentSource, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

/// sqlx-backed consent history.
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

#[derive(sqlx::FromRow)]
struct ConsentRow {
    id: Uuid,
    user_id: Uuid,
    purpose: String,
    granted: bool,
    recorded_at: DateTime<Utc>,
    source: String,
}

impl ConsentRow {
    fn into_domain(self) -> Result<ConsentRecord, RepositoryError> {
        Ok(ConsentRecord::record(
            ConsentRecordId::new(self.id),
            UserId::new(self.user_id),
            ConsentPurpose::parse(&self.purpose)
                .map_err(|error| invalid_data("read consent record", error))?,
            self.granted,
            ConsentSource::parse(&self.source)
                .map_err(|error| invalid_data("read consent record", error))?,
            self.recorded_at,
        ))
    }
}

#[async_trait]
impl ConsentRepository for PgConsentRepository {
    async fn append(&self, record: &ConsentRecord) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO consent_records (id, user_id, purpose, granted, recorded_at, source)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(record.id().as_uuid())
        .bind(record.user_id().as_uuid())
        .bind(record.purpose().as_str())
        .bind(record.granted())
        .bind(record.recorded_at())
        .bind(record.source().as_str())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("append consent record", error))?;
        Ok(())
    }

    async fn latest(
        &self,
        user_id: UserId,
        purpose: ConsentPurpose,
    ) -> Result<Option<ConsentRecord>, RepositoryError> {
        let row = sqlx::query_as::<_, ConsentRow>(
            "SELECT id, user_id, purpose, granted, recorded_at, source
             FROM consent_records
             WHERE user_id = $1 AND purpose = $2
             ORDER BY recorded_at DESC, id DESC
             LIMIT 1",
        )
        .bind(user_id.as_uuid())
        .bind(purpose.as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read latest consent", error))?;
        row.map(ConsentRow::into_domain).transpose()
    }

    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError> {
        Ok(
            sqlx::query("DELETE FROM consent_records WHERE user_id = $1")
                .bind(user_id.as_uuid())
                .execute(&self.pool)
                .await
                .map_err(|error| map_sqlx_error("delete consent history", error))?
                .rows_affected(),
        )
    }
}
