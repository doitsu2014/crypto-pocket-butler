//! Postgres implementation of [`SupportGrantRepository`].

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, SupportGrantRepository};
use identity_domain::{
    AuditRecord, SupportAccessGrant, SupportGrantId, SupportGrantStatus, UserId,
};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};
use crate::support::outbox::enqueue;

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
        insert_grant(&self.pool, grant)
            .await
            .map_err(|error| map_sqlx_error("insert support grant", error))
    }

    async fn find_by_id(
        &self,
        id: SupportGrantId,
    ) -> Result<Option<SupportAccessGrant>, RepositoryError> {
        find_by_id(&self.pool, id).await
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

    async fn find_live_by_agent_and_user(
        &self,
        agent_id: UserId,
        user_id: UserId,
    ) -> Result<Option<SupportAccessGrant>, RepositoryError> {
        let row = sqlx::query_as::<_, GrantRow>(&format!(
            "SELECT {GRANT_COLUMNS} FROM support_access_grants
             WHERE support_agent_id = $1 AND user_id = $2
               AND status IN ('pending_approval', 'active')
             ORDER BY requested_at DESC, id
             LIMIT 1"
        ))
        .bind(agent_id.as_uuid())
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find live support grant", error))?;
        row.map(GrantRow::into_domain).transpose()
    }

    async fn list_by_status(
        &self,
        status: SupportGrantStatus,
        limit: usize,
    ) -> Result<Vec<SupportAccessGrant>, RepositoryError> {
        let rows = sqlx::query_as::<_, GrantRow>(&format!(
            "SELECT {GRANT_COLUMNS} FROM support_access_grants
             WHERE status = $1
             ORDER BY requested_at DESC, id
             LIMIT $2"
        ))
        .bind(status.as_str())
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list support grants", error))?;
        rows.into_iter().map(GrantRow::into_domain).collect()
    }

    async fn list_by_support_agent(
        &self,
        agent_id: UserId,
        limit: usize,
    ) -> Result<Vec<SupportAccessGrant>, RepositoryError> {
        let rows = sqlx::query_as::<_, GrantRow>(&format!(
            "SELECT {GRANT_COLUMNS} FROM support_access_grants
             WHERE support_agent_id = $1
             ORDER BY requested_at DESC, id
             LIMIT $2"
        ))
        .bind(agent_id.as_uuid())
        .bind(i64::try_from(limit).unwrap_or(i64::MAX))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("list my support grants", error))?;
        rows.into_iter().map(GrantRow::into_domain).collect()
    }

    async fn insert_with_event(
        &self,
        grant: &SupportAccessGrant,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin support grant request", error))?;
        // The event is written first: if the grant insert breaks the live-pair
        // index, the rollback takes the intent with it (BR3.13).
        enqueue(
            &mut *transaction,
            record,
            correlation_id,
            grant.requested_at(),
        )
        .await
        .map_err(|error| map_sqlx_error("enqueue support grant request", error))?;
        insert_grant(&mut *transaction, grant)
            .await
            .map_err(|error| map_sqlx_error("insert support grant", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit support grant request", error))
    }

    async fn approve_with_event(
        &self,
        id: SupportGrantId,
        approved_by: UserId,
        approved_at: DateTime<Utc>,
        expires_at: DateTime<Utc>,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<u64, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin support approval", error))?;
        let affected = sqlx::query(
            "UPDATE support_access_grants
             SET status = 'active', approved_by = $2, approved_at = $3, expires_at = $4
             WHERE id = $1 AND status = 'pending_approval'",
        )
        .bind(id.as_uuid())
        .bind(approved_by.as_uuid())
        .bind(approved_at)
        .bind(expires_at)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("approve support grant", error))?
        .rows_affected();
        if affected == 1 {
            enqueue(&mut *transaction, record, correlation_id, approved_at)
                .await
                .map_err(|error| map_sqlx_error("enqueue support approval", error))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit support approval", error))?;
        Ok(affected)
    }

    async fn revoke_with_event(
        &self,
        id: SupportGrantId,
        revoked_at: DateTime<Utc>,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<u64, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin support revocation", error))?;
        let affected = sqlx::query(
            "UPDATE support_access_grants
             SET status = 'revoked', revoked_at = $2
             WHERE id = $1 AND status IN ('pending_approval', 'active')",
        )
        .bind(id.as_uuid())
        .bind(revoked_at)
        .execute(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("revoke support grant", error))?
        .rows_affected();
        if affected == 1 {
            enqueue(&mut *transaction, record, correlation_id, revoked_at)
                .await
                .map_err(|error| map_sqlx_error("enqueue support revocation", error))?;
        }
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit support revocation", error))?;
        Ok(affected)
    }
}

async fn insert_grant<'e, E>(executor: E, grant: &SupportAccessGrant) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
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
    .execute(executor)
    .await
    .map(|_| ())
}

async fn find_by_id(
    pool: &PgPool,
    id: SupportGrantId,
) -> Result<Option<SupportAccessGrant>, RepositoryError> {
    let row = sqlx::query_as::<_, GrantRow>(&format!(
        "SELECT {GRANT_COLUMNS} FROM support_access_grants WHERE id = $1"
    ))
    .bind(id.as_uuid())
    .fetch_optional(pool)
    .await
    .map_err(|error| map_sqlx_error("find support grant", error))?;
    row.map(GrantRow::into_domain).transpose()
}
