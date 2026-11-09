//! Postgres implementation of [`PreferenceRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_application::ports::{PreferenceRepository, RepositoryError};
use notification_domain::{
    Channel, NotificationPreference, NotificationType, PreferenceId, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const PREFERENCE_COLUMNS: &str = "id, user_id, \"type\", channel, enabled, updated_at";

/// sqlx-backed preference store.
#[derive(Debug, Clone)]
pub struct PgPreferenceRepository {
    pool: PgPool,
}

impl PgPreferenceRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct PreferenceRow {
    id: Uuid,
    user_id: Uuid,
    #[sqlx(rename = "type")]
    kind: String,
    channel: String,
    enabled: bool,
    updated_at: DateTime<Utc>,
}

impl PreferenceRow {
    fn into_domain(self) -> Result<NotificationPreference, RepositoryError> {
        let kind = NotificationType::parse(&self.kind)
            .map_err(|error| invalid_data("read notification preference", error))?;
        let channel = Channel::parse(&self.channel)
            .map_err(|error| invalid_data("read notification preference", error))?;
        Ok(NotificationPreference::stored(
            PreferenceId::new(self.id),
            UserId::new(self.user_id),
            kind,
            channel,
            self.enabled,
            self.updated_at,
        ))
    }
}

#[async_trait]
impl PreferenceRepository for PgPreferenceRepository {
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<NotificationPreference>, RepositoryError> {
        let rows = sqlx::query_as::<_, PreferenceRow>(&format!(
            "SELECT {PREFERENCE_COLUMNS} FROM notification_preferences
             WHERE user_id = $1
             ORDER BY \"type\", channel"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read notification preferences", error))?;
        rows.into_iter().map(PreferenceRow::into_domain).collect()
    }

    async fn upsert(&self, preference: &NotificationPreference) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO notification_preferences (id, user_id, \"type\", channel, enabled, updated_at)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (user_id, \"type\", channel)
             DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = EXCLUDED.updated_at",
        )
        .bind(preference.id().as_uuid())
        .bind(preference.user_id().as_uuid())
        .bind(preference.kind().as_str())
        .bind(preference.channel().as_str())
        .bind(preference.enabled())
        .bind(preference.updated_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("save notification preference", error))?;
        Ok(())
    }

    async fn upsert_all(
        &self,
        preferences: &[NotificationPreference],
    ) -> Result<(), RepositoryError> {
        if preferences.is_empty() {
            return Ok(());
        }
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin preference save", error))?;
        for preference in preferences {
            sqlx::query(
                "INSERT INTO notification_preferences
                     (id, user_id, \"type\", channel, enabled, updated_at)
                 VALUES ($1, $2, $3, $4, $5, $6)
                 ON CONFLICT (user_id, \"type\", channel)
                 DO UPDATE SET enabled = EXCLUDED.enabled, updated_at = EXCLUDED.updated_at",
            )
            .bind(preference.id().as_uuid())
            .bind(preference.user_id().as_uuid())
            .bind(preference.kind().as_str())
            .bind(preference.channel().as_str())
            .bind(preference.enabled())
            .bind(preference.updated_at())
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("save notification preference", error))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit preference save", error))?;
        Ok(())
    }
}
