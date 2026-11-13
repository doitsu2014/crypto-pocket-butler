//! Postgres implementation of [`StatusTransitionStore`] (US4.1, US7.2, BR5.9).
//!
//! Every write path is one transaction: `users.status`, the append-only
//! `status_transitions` row, and the `status_audit_outbox` intent commit
//! together or not at all. An accepted status change therefore always has a
//! durable audit intent, which is the guarantee BR5.9 makes.
//!
//! The status update is guarded on the status the caller observed. A row that
//! moved between the read and the write (a concurrent restore, a second admin)
//! matches nothing and is refused as a conflict, so the column can never
//! disagree with the newest transition.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, StatusChange, StatusTransitionStore};
use identity_domain::{DeletionRequest, StatusTransition, StatusTransitionId, UserId, UserStatus};
use serde_json::json;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use super::{invalid_data, map_sqlx_error};

const TRANSITION_COLUMNS: &str =
    "id, user_id, from_status, to_status, actor, reason, occurred_at, seq";

/// sqlx-backed status changes, transitions, and audit intents.
#[derive(Debug, Clone)]
pub struct PgStatusTransitionStore {
    pool: PgPool,
}

impl PgStatusTransitionStore {
    /// Creates the store over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn begin(&self, operation: &str) -> Result<Transaction<'_, Postgres>, RepositoryError> {
        self.pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error(operation, error))
    }

    /// Writes the status the caller observed changing to the status it asked for.
    async fn write_status(
        transaction: &mut Transaction<'_, Postgres>,
        change: &StatusChange,
    ) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE users
             SET status = $3, updated_at = $4
             WHERE id = $1 AND status = $2",
        )
        .bind(change.user_id.as_uuid())
        .bind(change.from.as_str())
        .bind(change.to.as_str())
        .bind(change.occurred_at)
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_error("apply status change", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(format!(
                "the account is no longer '{}'",
                change.from.as_str()
            )));
        }
        Ok(())
    }

    /// Appends the transition and its audit intent, inside the caller's
    /// transaction.
    async fn append(
        transaction: &mut Transaction<'_, Postgres>,
        change: &StatusChange,
    ) -> Result<StatusTransition, RepositoryError> {
        // The domain validates the edge, the actor, and the reason a second
        // time, so a caller that bypassed the use case still cannot record an
        // illegal transition.
        let transition = StatusTransition::new(
            StatusTransitionId::generate(),
            change.user_id,
            change.from,
            change.to,
            &change.actor,
            change.reason.as_deref(),
            change.occurred_at,
        )
        .map_err(|error| RepositoryError::Conflict(error.to_string()))?;

        let seq: i64 = sqlx::query_scalar(
            "INSERT INTO status_transitions
                 (id, user_id, from_status, to_status, actor, reason, occurred_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)
             RETURNING seq",
        )
        .bind(transition.id().as_uuid())
        .bind(transition.user_id().as_uuid())
        .bind(transition.from_status().as_str())
        .bind(transition.to_status().as_str())
        .bind(transition.actor())
        .bind(transition.reason())
        .bind(transition.occurred_at())
        .fetch_one(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_error("append status transition", error))?;

        let payload = audit_payload(change);
        sqlx::query(
            "INSERT INTO status_audit_outbox
                 (transition_id, payload, idempotency_key, next_attempt_at, created_at)
             VALUES ($1, $2, $3, $4, $4)",
        )
        .bind(transition.id().as_uuid())
        .bind(payload)
        .bind(format!("status-transition:{}", transition.id()))
        .bind(change.occurred_at)
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_error("enqueue status audit", error))?;

        Ok(StatusTransition::from_stored(
            transition.id(),
            transition.user_id(),
            transition.from_status(),
            transition.to_status(),
            transition.actor().to_string(),
            transition.reason().map(ToString::to_string),
            transition.occurred_at(),
            seq,
        ))
    }

    /// Inserts the open request inside the caller's transaction.
    async fn insert_request(
        transaction: &mut Transaction<'_, Postgres>,
        request: &DeletionRequest,
    ) -> Result<(), RepositoryError> {
        sqlx::query(
            "INSERT INTO deletion_requests
                 (id, user_id, status, progress, retained_categories, requested_at, completed_at,
                  failure_reason, lease_owner, lease_until, recoverable_until, cancelled_at,
                  erased_at, attempt_count, next_attempt_at)
             VALUES ($1, $2, $3, '{}'::jsonb, '[]'::jsonb, $4, NULL, NULL, NULL, NULL,
                     $5, $6, $7, $8, $9)",
        )
        .bind(request.id().as_uuid())
        .bind(request.user_id().as_uuid())
        .bind(request.status().as_str())
        .bind(request.requested_at())
        .bind(request.recoverable_until())
        .bind(request.cancelled_at())
        .bind(request.erased_at())
        .bind(i32::try_from(request.attempt_count()).unwrap_or(i32::MAX))
        .bind(request.next_attempt_at())
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_error("open deletion request", error))?;
        Ok(())
    }

    /// Records the cancellation inside the caller's transaction.
    async fn cancel_request(
        transaction: &mut Transaction<'_, Postgres>,
        request: &DeletionRequest,
    ) -> Result<(), RepositoryError> {
        let affected = sqlx::query(
            "UPDATE deletion_requests
             SET cancelled_at = $2
             WHERE id = $1 AND cancelled_at IS NULL AND erased_at IS NULL",
        )
        .bind(request.id().as_uuid())
        .bind(request.cancelled_at())
        .execute(&mut **transaction)
        .await
        .map_err(|error| map_sqlx_error("cancel deletion request", error))?
        .rows_affected();
        if affected == 0 {
            return Err(RepositoryError::Conflict(
                "the deletion request is already closed".to_string(),
            ));
        }
        Ok(())
    }
}

/// The payload C9 receives. Identifiers, enum values, and the admin reason; no
/// email, subject, or token.
fn audit_payload(change: &StatusChange) -> serde_json::Value {
    let mut context = serde_json::Map::new();
    context.insert(
        "fromStatus".to_string(),
        serde_json::Value::String(change.from.as_str().to_string()),
    );
    context.insert(
        "toStatus".to_string(),
        serde_json::Value::String(change.to.as_str().to_string()),
    );
    if let Some(reason) = &change.reason {
        context.insert(
            "reason".to_string(),
            serde_json::Value::String(reason.clone()),
        );
    }
    json!({
        "actorId": change.actor,
        "actorKind": change.actor_kind,
        "action": "identity.account.status_changed",
        "target": change.user_id.to_string(),
        "occurredAt": change.occurred_at,
        "outcome": "success",
        "context": context,
        "sensitivity": "security",
    })
}

#[derive(sqlx::FromRow)]
struct TransitionRow {
    id: Uuid,
    user_id: Uuid,
    from_status: String,
    to_status: String,
    actor: String,
    reason: Option<String>,
    occurred_at: DateTime<Utc>,
    seq: i64,
}

impl TransitionRow {
    fn into_domain(self) -> Result<StatusTransition, RepositoryError> {
        Ok(StatusTransition::from_stored(
            StatusTransitionId::new(self.id),
            UserId::new(self.user_id),
            UserStatus::parse(&self.from_status)
                .map_err(|error| invalid_data("read transition", error))?,
            UserStatus::parse(&self.to_status)
                .map_err(|error| invalid_data("read transition", error))?,
            self.actor,
            self.reason,
            self.occurred_at,
            self.seq,
        ))
    }
}

#[async_trait]
impl StatusTransitionStore for PgStatusTransitionStore {
    async fn apply(&self, change: &StatusChange) -> Result<StatusTransition, RepositoryError> {
        let mut transaction = self.begin("begin status change").await?;
        Self::write_status(&mut transaction, change).await?;
        let transition = Self::append(&mut transaction, change).await?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit status change", error))?;
        Ok(transition)
    }

    async fn open_deletion(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, RepositoryError> {
        let mut transaction = self.begin("begin deletion request").await?;
        Self::write_status(&mut transaction, change).await?;
        Self::insert_request(&mut transaction, request).await?;
        let transition = Self::append(&mut transaction, change).await?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit deletion request", error))?;
        Ok(transition)
    }

    async fn cancel_deletion(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, RepositoryError> {
        let mut transaction = self.begin("begin deletion cancellation").await?;
        Self::cancel_request(&mut transaction, request).await?;
        Self::write_status(&mut transaction, change).await?;
        let transition = Self::append(&mut transaction, change).await?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit deletion cancellation", error))?;
        Ok(transition)
    }

    async fn newest_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<StatusTransition>, RepositoryError> {
        let row = sqlx::query_as::<_, TransitionRow>(&format!(
            "SELECT {TRANSITION_COLUMNS} FROM status_transitions
             WHERE user_id = $1
             ORDER BY occurred_at DESC, seq DESC
             LIMIT 1"
        ))
        .bind(user_id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read newest transition", error))?;
        row.map(TransitionRow::into_domain).transpose()
    }
}
