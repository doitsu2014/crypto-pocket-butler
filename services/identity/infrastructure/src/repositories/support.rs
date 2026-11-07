//! Postgres implementation of [`SupportGrantRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, SupportGrantRepository};
use identity_domain::{SupportAccessGrant, SupportGrantId, SupportGrantStatus, UserId};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const GRANT_COLUMNS: &str = "id, user_id, support_agent_id, reason, status, requested_at, \
     approved_by, approved_at, expires_at, revoked_at";

/// sqlx-backed support grants.
#[derive(Debug, Clone)]
pub struct PgSupportGrantRepository {
    pool: PgPool,
}

impl PgSupportGrantRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct GrantRow {
    id: Uuid,
    user_id: Uuid,
    support_agent_id: Uuid,
    reason: String,
    status: String,
    requested_at: DateTime<Utc>,
    approved_by: Option<Uuid>,
    approved_at: Option<DateTime<Utc>>,
    expires_at: Option<DateTime<Utc>>,
    revoked_at: Option<DateTime<Utc>>,
}

impl GrantRow {
    fn into_domain(self) -> Result<SupportAccessGrant, RepositoryError> {
        Ok(SupportAccessGrant::from_stored(
            SupportGrantId::new(self.id),
            UserId::new(self.user_id),
            UserId::new(self.support_agent_id),
            self.reason,
            SupportGrantStatus::parse(&self.status)
                .map_err(|error| invalid_data("read support grant", error))?,
            self.requested_at,
            self.approved_by.map(UserId::new),
            self.approved_at,
            self.expires_at,
            self.revoked_at,
        ))
    }
}

#[async_trait]
impl SupportGrantRepository for PgSupportGrantRepository {
    async fn insert(&self, grant: &SupportAccessGrant) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO support_access_grants
                 (id, user_id, support_agent_id, reason, status, requested_at, approved_by,
                  approved_at, expires_at, revoked_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
        )
        .bind(grant.id().as_uuid())
        .bind(grant.user_id().as_uuid())
        .bind(grant.support_agent_id().as_uuid())
        .bind(grant.reason())
        .bind(grant.status().as_str())
        .bind(grant.requested_at())
        .bind(grant.approved_by().map(|id| id.as_uuid()))
        .bind(grant.approved_at())
        .bind(grant.expires_at())
        .bind(grant.revoked_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert support grant", error))?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: SupportGrantId,
    ) -> Result<Option<SupportAccessGrant>, RepositoryError> {
        let row = sqlx::query_as::<_, GrantRow>(&format!(
            "SELECT {GRANT_COLUMNS} FROM support_access_grants WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find support grant", error))?;
        row.map(GrantRow::into_domain).transpose()
    }

    async fn update(&self, grant: &SupportAccessGrant) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE support_access_grants
             SET status = $2, approved_by = $3, approved_at = $4, expires_at = $5, revoked_at = $6
             WHERE id = $1",
        )
        .bind(grant.id().as_uuid())
        .bind(grant.status().as_str())
        .bind(grant.approved_by().map(|id| id.as_uuid()))
        .bind(grant.approved_at())
        .bind(grant.expires_at())
        .bind(grant.revoked_at())
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update support grant", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "support grant '{}' does not exist",
                grant.id()
            )));
        }
        Ok(())
    }
}
