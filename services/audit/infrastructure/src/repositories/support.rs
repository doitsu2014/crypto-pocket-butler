//! Postgres implementation of [`SupportSessionRepository`].

use async_trait::async_trait;
use audit_application::ports::{RepositoryError, SupportSessionRepository};
use audit_domain::{
    SupportScope, SupportSession, SupportSessionId, SupportSessionOutcome, SupportSessionRequest,
    SupportSessionState, UserId,
};
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const SESSION_COLUMNS: &str = "id, support_agent_id, subject_user_id, reason, second_approver_id, \
     scope, started_at, expires_at, ended_at, state, approved_at, outcome";

/// sqlx-backed support sessions.
#[derive(Debug, Clone)]
pub struct PgSupportSessionRepository {
    pool: PgPool,
}

impl PgSupportSessionRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    id: Uuid,
    support_agent_id: Uuid,
    subject_user_id: Uuid,
    reason: String,
    second_approver_id: Uuid,
    scope: String,
    started_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    ended_at: Option<DateTime<Utc>>,
    state: String,
    approved_at: Option<DateTime<Utc>>,
    outcome: Option<String>,
}

impl SessionRow {
    fn into_domain(self) -> Result<SupportSession, RepositoryError> {
        let request = SupportSessionRequest::new(
            UserId::new(self.support_agent_id),
            UserId::new(self.subject_user_id),
            &self.reason,
            UserId::new(self.second_approver_id),
            SupportScope::parse(&self.scope)
                .map_err(|error| invalid_data("read support session", error))?,
            self.started_at,
            self.expires_at,
        )
        .map_err(|error| invalid_data("read support session", error))?;
        let state = SupportSessionState::parse(&self.state)
            .map_err(|error| invalid_data("read support session", error))?;
        let outcome = match self.outcome.as_deref() {
            None => None,
            Some(raw) => Some(
                SupportSessionOutcome::parse(raw)
                    .map_err(|error| invalid_data("read support session", error))?,
            ),
        };
        Ok(SupportSession::from_stored(
            SupportSessionId::new(self.id),
            request,
            state,
            self.approved_at,
            self.ended_at,
            outcome,
        ))
    }
}

#[async_trait]
impl SupportSessionRepository for PgSupportSessionRepository {
    async fn insert(&self, session: &SupportSession) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO support_access_records
                 (id, support_agent_id, subject_user_id, reason, second_approver_id, scope,
                  started_at, expires_at, ended_at, state, approved_at, outcome)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(session.id().as_uuid())
        .bind(session.support_agent_id().as_uuid())
        .bind(session.subject_user_id().as_uuid())
        .bind(session.reason())
        .bind(session.second_approver_id().as_uuid())
        .bind(session.scope().as_str())
        .bind(session.started_at())
        .bind(session.expires_at())
        .bind(session.ended_at())
        .bind(session.stored_state().as_str())
        .bind(session.approved_at())
        .bind(session.outcome().map(SupportSessionOutcome::as_str))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("insert support session", error))?;
        Ok(())
    }

    async fn find_by_id(
        &self,
        id: SupportSessionId,
    ) -> Result<Option<SupportSession>, RepositoryError> {
        let row = sqlx::query_as::<_, SessionRow>(&format!(
            "SELECT {SESSION_COLUMNS} FROM support_access_records WHERE id = $1"
        ))
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("find support session", error))?;
        row.map(SessionRow::into_domain).transpose()
    }

    async fn update(&self, session: &SupportSession) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE support_access_records
             SET state = $2, ended_at = $3, outcome = $4
             WHERE id = $1",
        )
        .bind(session.id().as_uuid())
        .bind(session.stored_state().as_str())
        .bind(session.ended_at())
        .bind(session.outcome().map(SupportSessionOutcome::as_str))
        .execute(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("update support session", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "support session '{}' does not exist",
                session.id()
            )));
        }
        Ok(())
    }
}
