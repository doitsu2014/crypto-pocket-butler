//! Postgres implementation of [`AttemptRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_application::ports::{AttemptRepository, RepositoryError};
use notification_domain::{
    AlertId, Channel, DeliveryAttempt, DeliveryAttemptId, ErrorCategory, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const ATTEMPT_COLUMNS: &str =
    "id, alert_id, user_id, channel, attempt, status, error_category, attempted_at";

/// sqlx-backed delivery-attempt store.
#[derive(Debug, Clone)]
pub struct PgAttemptRepository {
    pool: PgPool,
}

impl PgAttemptRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AttemptRow {
    id: Uuid,
    alert_id: Uuid,
    user_id: Uuid,
    channel: String,
    attempt: i16,
    status: String,
    error_category: Option<String>,
    attempted_at: DateTime<Utc>,
}

impl AttemptRow {
    fn into_domain(self) -> Result<DeliveryAttempt, RepositoryError> {
        let channel = Channel::parse(&self.channel)
            .map_err(|error| invalid_data("read delivery attempt", error))?;
        let category = match self.error_category.as_deref() {
            None => None,
            Some(raw) => Some(
                ErrorCategory::parse(raw)
                    .map_err(|error| invalid_data("read delivery attempt", error))?,
            ),
        };
        let attempt = u8::try_from(self.attempt)
            .map_err(|error| invalid_data("read delivery attempt", error))?;
        DeliveryAttempt::from_stored(
            DeliveryAttemptId::new(self.id),
            AlertId::new(self.alert_id),
            UserId::new(self.user_id),
            channel,
            attempt,
            &self.status,
            category,
            self.attempted_at,
        )
        .map_err(|error| invalid_data("read delivery attempt", error))
    }
}

#[async_trait]
impl AttemptRepository for PgAttemptRepository {
    async fn insert(&self, attempt: &DeliveryAttempt) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO delivery_attempts
                 (id, alert_id, user_id, channel, attempt, status, error_category, attempted_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(attempt.id().as_uuid())
        .bind(attempt.alert_id().as_uuid())
        .bind(attempt.user_id().as_uuid())
        .bind(attempt.channel().as_str())
        .bind(i16::from(attempt.attempt()))
        .bind(attempt.status().as_str())
        .bind(attempt.error_category().map(ErrorCategory::as_str))
        .bind(attempt.attempted_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert delivery attempt", error))?;
        Ok(())
    }

    async fn list_for_alert(
        &self,
        alert_id: AlertId,
    ) -> Result<Vec<DeliveryAttempt>, RepositoryError> {
        let rows = sqlx::query_as::<_, AttemptRow>(&format!(
            "SELECT {ATTEMPT_COLUMNS} FROM delivery_attempts
             WHERE alert_id = $1
             ORDER BY channel, attempt"
        ))
        .bind(alert_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read delivery attempts", error))?;
        rows.into_iter().map(AttemptRow::into_domain).collect()
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM delivery_attempts WHERE id IN (
                 SELECT id FROM delivery_attempts
                 WHERE attempted_at < $1
                 ORDER BY attempted_at
                 LIMIT $2
             )",
        )
        .bind(cutoff)
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete delivery attempts", error))?;
        Ok(result.rows_affected())
    }
}
