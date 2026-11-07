//! Postgres implementation of [`DeviceSessionRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{DeviceSessionRepository, RepositoryError};
use identity_domain::{DeviceSession, DeviceSessionId, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::map_sqlx_error;

/// sqlx-backed device sessions.
#[derive(Debug, Clone)]
pub struct PgDeviceSessionRepository {
    pool: PgPool,
}

impl PgDeviceSessionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    user_id: Uuid,
    auth_session_id: String,
    device: Option<String>,
    location: Option<String>,
    last_activity_at: DateTime<Utc>,
    revoked_at: Option<DateTime<Utc>>,
}

impl SessionRow {
    fn into_domain(self) -> DeviceSession {
        DeviceSession::from_stored(
            DeviceSessionId::new(self.id),
            UserId::new(self.user_id),
            self.auth_session_id,
            self.device,
            self.location,
            self.last_activity_at,
            self.revoked_at,
        )
    }
}

const SESSION_COLUMNS: &str =
    "id, user_id, auth_session_id, device, location, last_activity_at, revoked_at";

#[async_trait]
impl DeviceSessionRepository for PgDeviceSessionRepository {
    async fn upsert(&self, session: &DeviceSession) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO device_sessions
                 (id, user_id, auth_session_id, device, location, last_activity_at, revoked_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             ON CONFLICT (user_id, auth_session_id) DO UPDATE
             SET device = COALESCE(EXCLUDED.device, device_sessions.device),
                 location = EXCLUDED.location,
                 last_activity_at = GREATEST(device_sessions.last_activity_at, EXCLUDED.last_activity_at),
                 revoked_at = COALESCE(device_sessions.revoked_at, EXCLUDED.revoked_at)",
        )
        .bind(session.id().as_uuid())
        .bind(session.user_id().as_uuid())
        .bind(session.auth_session_id())
        .bind(session.device())
        .bind(session.location())
        .bind(session.last_activity_at())
        .bind(session.revoked_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("upsert device session", error))?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<DeviceSession>, RepositoryError> {
        let rows = sqlx::query_as::<_, SessionRow>(&format!(
            "SELECT {SESSION_COLUMNS} FROM device_sessions
             WHERE user_id = $1
             ORDER BY last_activity_at DESC, id DESC"
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list device sessions", error))?;
        Ok(rows.into_iter().map(SessionRow::into_domain).collect())
    }

    async fn find(&self, id: DeviceSessionId) -> Result<Option<DeviceSession>, RepositoryError> {
        let row = sqlx::query_as::<_, SessionRow>(&format!(
            "SELECT {SESSION_COLUMNS} FROM device_sessions WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find device session", error))?;
        Ok(row.map(SessionRow::into_domain))
    }

    async fn update(&self, session: &DeviceSession) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE device_sessions
             SET device = $2, location = $3, last_activity_at = $4, revoked_at = $5
             WHERE id = $1",
        )
        .bind(session.id().as_uuid())
        .bind(session.device())
        .bind(session.location())
        .bind(session.last_activity_at())
        .bind(session.revoked_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update device session", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "device session '{}' does not exist",
                session.id()
            )));
        }
        Ok(())
    }
}
