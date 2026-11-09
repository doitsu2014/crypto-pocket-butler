//! Postgres implementation of [`AlertRepository`].

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_application::ports::{
    AlertRepository, AnonymisedRows, InsertOutcome, RepositoryError,
};
use notification_domain::{Alert, AlertId, AlertStatus, NotificationType, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const ALERT_COLUMNS: &str =
    "id, user_id, \"type\", idempotency_key, payload, status, sanitised_fields, \
     next_attempt_at, created_at, requested_channel";

/// sqlx-backed alert store. The table is the queue: there is no message queue.
#[derive(Debug, Clone)]
pub struct PgAlertRepository {
    pool: PgPool,
}

impl PgAlertRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct AlertRow {
    id: Uuid,
    user_id: Uuid,
    #[sqlx(rename = "type")]
    kind: String,
    idempotency_key: String,
    payload: serde_json::Value,
    status: String,
    sanitised_fields: Vec<String>,
    next_attempt_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
    requested_channel: String,
}

impl AlertRow {
    fn into_domain(self) -> Result<Alert, RepositoryError> {
        let payload: BTreeMap<String, String> = match self.payload {
            serde_json::Value::Object(map) => map
                .into_iter()
                .map(|(key, value)| {
                    let value = match value {
                        serde_json::Value::String(text) => text,
                        other => other.to_string(),
                    };
                    (key, value)
                })
                .collect(),
            _ => {
                return Err(invalid_data(
                    "read notification alert",
                    "payload is not a JSON object",
                ));
            }
        };
        let kind = NotificationType::parse(&self.kind)
            .map_err(|error| invalid_data("read notification alert", error))?;
        Alert::from_stored_with_channel(
            AlertId::new(self.id),
            UserId::new(self.user_id),
            kind,
            &self.idempotency_key,
            payload,
            &self.status,
            self.sanitised_fields,
            self.created_at,
            self.next_attempt_at,
            &self.requested_channel,
        )
        .map_err(|error| invalid_data("read notification alert", error))
    }
}

#[async_trait]
impl AlertRepository for PgAlertRepository {
    async fn insert(&self, alert: &Alert) -> Result<InsertOutcome, RepositoryError> {
        let payload = serde_json::to_value(alert.payload())
            .map_err(|error| invalid_data("insert notification alert", error))?;
        let inserted: Option<Uuid> = sqlx::query_scalar(
            "INSERT INTO alerts
                 (id, user_id, \"type\", idempotency_key, payload, status,
                  sanitised_fields, next_attempt_at, created_at, updated_at,
                  requested_channel)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $9, $10)
             ON CONFLICT (idempotency_key) DO NOTHING
             RETURNING id",
        )
        .bind(alert.id().as_uuid())
        .bind(alert.user_id().as_uuid())
        .bind(alert.kind().as_str())
        .bind(alert.idempotency_key())
        .bind(payload)
        .bind(alert.status().as_str())
        .bind(alert.sanitised_fields().to_vec())
        .bind(alert.next_attempt_at())
        .bind(alert.created_at())
        .bind(alert.requested_channel().as_str())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert notification alert", error))?;
        if inserted.is_some() {
            return Ok(InsertOutcome::Inserted(alert.clone()));
        }
        let existing = self
            .find_by_idempotency_key(alert.idempotency_key())
            .await?
            .ok_or_else(|| {
                RepositoryError::Conflict(
                    "the idempotency key conflicted but no row is readable".to_string(),
                )
            })?;
        Ok(InsertOutcome::Duplicate(existing))
    }

    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Alert>, RepositoryError> {
        let row = sqlx::query_as::<_, AlertRow>(&format!(
            "SELECT {ALERT_COLUMNS} FROM alerts WHERE idempotency_key = $1"
        ))
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read notification alert", error))?;
        row.map(AlertRow::into_domain).transpose()
    }

    async fn list_for_user(
        &self,
        user_id: UserId,
        before: Option<(DateTime<Utc>, AlertId)>,
        limit: u32,
    ) -> Result<Vec<Alert>, RepositoryError> {
        let (before_at, before_id) = match before {
            None => (None, None),
            Some((created_at, id)) => (Some(created_at), Some(id.as_uuid())),
        };
        let rows = sqlx::query_as::<_, AlertRow>(&format!(
            "SELECT {ALERT_COLUMNS} FROM alerts
             WHERE user_id = $1
               -- C9: an email-only alert (a brief) adds no in-app entry.
               AND requested_channel <> 'email'
               -- BR2.1 and WF4.2: a type disabled for in-app never reaches the
               -- feed. A missing row keeps the default (in-app on), so only an
               -- explicit false suppresses it.
               AND NOT EXISTS (
                   SELECT 1 FROM notification_preferences AS preference
                   WHERE preference.user_id = alerts.user_id
                     AND preference.\"type\" = alerts.\"type\"
                     AND preference.channel = 'in_app'
                     AND preference.enabled = false
               )
               AND ($2::timestamptz IS NULL
                    OR (created_at, id) < ($2::timestamptz, $3::uuid))
             ORDER BY created_at DESC, id DESC
             LIMIT $4"
        ))
        .bind(user_id.as_uuid())
        .bind(before_at)
        .bind(before_id)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read notification feed", error))?;
        rows.into_iter().map(AlertRow::into_domain).collect()
    }

    async fn due(&self, now: DateTime<Utc>, limit: u32) -> Result<Vec<Alert>, RepositoryError> {
        let rows = sqlx::query_as::<_, AlertRow>(&format!(
            "SELECT {ALERT_COLUMNS} FROM alerts
             WHERE status = 'queued'
               AND next_attempt_at IS NOT NULL
               AND next_attempt_at <= $1
             ORDER BY next_attempt_at
             LIMIT $2"
        ))
        .bind(now)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read due notification alerts", error))?;
        rows.into_iter().map(AlertRow::into_domain).collect()
    }

    async fn has_due(&self, now: DateTime<Utc>) -> Result<bool, RepositoryError> {
        sqlx::query_scalar(
            "SELECT EXISTS (
                 SELECT 1 FROM alerts
                 WHERE status = 'queued'
                   AND next_attempt_at IS NOT NULL
                   AND next_attempt_at <= $1
             )",
        )
        .bind(now)
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read notification backlog", error))
    }

    async fn set_status(
        &self,
        id: AlertId,
        status: AlertStatus,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "UPDATE alerts
             SET status = $2, next_attempt_at = $3, updated_at = now()
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(status.as_str())
        .bind(next_attempt_at)
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update notification alert", error))?;
        Ok(())
    }

    async fn anonymise_user(
        &self,
        user_id: UserId,
        replacement: UserId,
    ) -> Result<AnonymisedRows, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin notification erasure", error))?;
        let alerts =
            sqlx::query("UPDATE alerts SET user_id = $2, updated_at = now() WHERE user_id = $1")
                .bind(user_id.as_uuid())
                .bind(replacement.as_uuid())
                .execute(&mut *transaction)
                .await
                .map_err(|error| map_sqlx_error("erase notification alerts", error))?
                .rows_affected();
        let attempts = sqlx::query("UPDATE delivery_attempts SET user_id = $2 WHERE user_id = $1")
            .bind(user_id.as_uuid())
            .bind(replacement.as_uuid())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("erase notification attempts", error))?
            .rows_affected();
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit notification erasure", error))?;
        Ok(AnonymisedRows { alerts, attempts })
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM alerts WHERE id IN (
                 SELECT id FROM alerts
                 WHERE created_at < $1
                 ORDER BY created_at
                 LIMIT $2
             )",
        )
        .bind(cutoff)
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete notification alerts", error))?;
        Ok(result.rows_affected())
    }
}
