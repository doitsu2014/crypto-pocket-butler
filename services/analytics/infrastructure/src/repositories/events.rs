//! Postgres implementation of [`EventRepository`].

use std::collections::BTreeMap;

use analytics_application::ports::{EventRepository, RepositoryError};
use analytics_domain::{AnalyticsEvent, AnalyticsEventId, ConsentCheck};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const EVENT_COLUMNS: &str =
    "id, event_name, occurred_at, properties, user_ref, consent_purpose, ingested_at";

/// sqlx-backed event store.
#[derive(Debug, Clone)]
pub struct PgEventRepository {
    pool: PgPool,
}

impl PgEventRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct EventRow {
    id: Uuid,
    event_name: String,
    occurred_at: DateTime<Utc>,
    properties: serde_json::Value,
    user_ref: Option<String>,
    consent_purpose: String,
    ingested_at: DateTime<Utc>,
}

impl EventRow {
    fn into_domain(self) -> Result<AnalyticsEvent, RepositoryError> {
        let properties: BTreeMap<String, String> = match self.properties {
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
                    "read analytics event",
                    "properties is not a JSON object",
                ));
            }
        };
        AnalyticsEvent::from_stored(
            AnalyticsEventId::new(self.id),
            &self.event_name,
            self.occurred_at,
            properties,
            self.user_ref,
            &self.consent_purpose,
            self.ingested_at,
        )
        .map_err(|error| invalid_data("read analytics event", error))
    }
}

#[async_trait]
impl EventRepository for PgEventRepository {
    async fn insert(
        &self,
        event: &AnalyticsEvent,
        consent: Option<&ConsentCheck>,
    ) -> Result<(), RepositoryError> {
        let properties = serde_json::to_value(event.properties())
            .map_err(|error| invalid_data("insert event", error))?;
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin event insert", error))?;

        sqlx::query(
            "INSERT INTO events
                 (id, event_name, event_class, occurred_at, properties, user_ref,
                  consent_purpose, ingested_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
        )
        .bind(event.id().as_uuid())
        .bind(event.event_name())
        .bind(event.class().as_str())
        .bind(event.occurred_at())
        .bind(properties)
        .bind(event.user_ref())
        .bind(event.consent_purpose().as_str())
        .bind(event.ingested_at())
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("insert analytics event", error))?;

        if let Some(consent) = consent {
            sqlx::query(
                "INSERT INTO consent_checks (id, purpose, granted, checked_at, user_ref)
                 VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(consent.id().as_uuid())
            .bind(consent.purpose().as_str())
            .bind(consent.granted())
            .bind(consent.checked_at())
            .bind(consent.user_ref())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("insert consent check", error))?;
        }

        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit event insert", error))?;
        Ok(())
    }

    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AnalyticsEvent>, RepositoryError> {
        let rows = sqlx::query_as::<_, EventRow>(&format!(
            "SELECT {EVENT_COLUMNS} FROM events
             WHERE occurred_at >= $1 AND occurred_at < $2
             ORDER BY occurred_at LIMIT $3"
        ))
        .bind(from)
        .bind(to)
        .bind(i64::from(limit))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read analytics events", error))?;
        rows.into_iter().map(EventRow::into_domain).collect()
    }

    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError> {
        let result = sqlx::query(
            "DELETE FROM events WHERE id IN (
                 SELECT id FROM events WHERE occurred_at < $1 ORDER BY occurred_at LIMIT $2
             )",
        )
        .bind(cutoff)
        .bind(i64::from(limit))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("delete analytics events", error))?;
        Ok(result.rows_affected())
    }

    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError> {
        let result = sqlx::query("DELETE FROM events WHERE user_ref = ANY($1)")
            .bind(user_refs.to_vec())
            .execute(&self.pool)
            .await
            .map_err(|error| map_sqlx_error("erase analytics events", error))?;
        Ok(result.rows_affected())
    }
}
