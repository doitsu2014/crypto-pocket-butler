//! sqlx-backed consent events: the newest-per-kind read, paged history, the
//! insert, and the transactional withdrawal behind a per-(user, kind) advisory
//! lock (BR4.4, BR4.5, BR4.8).

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::consent::ports::{
    AuditIntent, ConsentRepository as ConsentEventRepository, ConsentTransaction, HistoryCursor,
};
use identity_application::ports::RepositoryError;
use identity_domain::{
    ConsentAction, ConsentEvent, ConsentEventDraft, ConsentKind, ConsentRecordId, UserId,
};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::repositories::{invalid_data, map_sqlx_error};

/// The columns one event read returns. A row written by a replica still running
/// the previous code has no event columns; the read maps its legacy columns so
/// it can never be silently missing from the gate. After 0005 every stored row
/// has the event shape, and `COALESCE` then returns it unchanged.
const EVENT_KIND: &str = "COALESCE(kind, CASE purpose
        WHEN 'analytics' THEN 'analytics'
        WHEN 'marketing' THEN 'marketing'
        WHEN 'disclosures' THEN 'terms'
    END)";
const EVENT_ACTION: &str =
    "COALESCE(action, CASE WHEN granted THEN 'granted' ELSE 'withdrawn' END)";
const EVENT_AT: &str = "COALESCE(occurred_at, recorded_at)";
const EVENT_POLICY: &str = "COALESCE(policy_version, 'legacy:' || COALESCE(source, 'settings'))";

fn event_columns() -> String {
    format!(
        "id, seq, user_id,
         {EVENT_KIND} AS kind,
         {EVENT_ACTION} AS action,
         {EVENT_AT} AS occurred_at,
         {EVENT_POLICY} AS policy_version"
    )
}

/// sqlx-backed consent events.
#[derive(Debug, Clone)]
pub struct PgConsentEventRepository {
    pool: PgPool,
}

impl PgConsentEventRepository {
    /// Creates the repository over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[derive(sqlx::FromRow)]
struct ConsentEventRow {
    id: Uuid,
    seq: i64,
    user_id: Uuid,
    kind: String,
    action: String,
    occurred_at: DateTime<Utc>,
    policy_version: String,
}

impl ConsentEventRow {
    fn into_domain(self) -> Result<ConsentEvent, RepositoryError> {
        Ok(ConsentEvent::from_stored(
            ConsentRecordId::new(self.id),
            self.seq,
            UserId::new(self.user_id),
            ConsentKind::parse(&self.kind)
                .map_err(|error| invalid_data("read consent event", error))?,
            ConsentAction::parse(&self.action)
                .map_err(|error| invalid_data("read consent event", error))?,
            self.occurred_at,
            self.policy_version,
        ))
    }
}

/// The payload C9 receives. It carries identifiers and enum values only.
fn audit_payload(intent: &AuditIntent) -> serde_json::Value {
    serde_json::json!({
        "actorId": intent.actor_id.to_string(),
        "actorKind": "user",
        "action": intent.action,
        "target": intent.target,
        "occurredAt": intent.occurred_at,
        "outcome": intent.outcome.as_str(),
        "context": intent.context,
        "sensitivity": "security",
    })
}

#[async_trait]
impl ConsentEventRepository for PgConsentEventRepository {
    async fn newest_per_kind(&self, user_id: UserId) -> Result<Vec<ConsentEvent>, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin consent-state read", error))?;
        sqlx::query("SET LOCAL statement_timeout = '100ms'")
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("bound consent-state read", error))?;
        let rows = sqlx::query_as::<_, ConsentEventRow>(&format!(
            "SELECT DISTINCT ON ({EVENT_KIND}) {}
             FROM consent_records
             WHERE user_id = $1 AND {EVENT_KIND} IS NOT NULL
             ORDER BY {EVENT_KIND}, {EVENT_AT} DESC, seq DESC",
            event_columns()
        ))
        .bind(user_id.as_uuid())
        .fetch_all(&mut *transaction)
        .await
        .map_err(|error| map_sqlx_error("read consent state", error))?;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit consent-state read", error))?;
        rows.into_iter().map(ConsentEventRow::into_domain).collect()
    }

    async fn history(
        &self,
        user_id: UserId,
        limit: u32,
        before: Option<HistoryCursor>,
    ) -> Result<Vec<ConsentEvent>, RepositoryError> {
        let (before_at, before_seq) = match before {
            Some(cursor) => (Some(cursor.occurred_at), Some(cursor.seq)),
            None => (None, None),
        };
        let rows = sqlx::query_as::<_, ConsentEventRow>(&format!(
            "SELECT {}
             FROM consent_records
             WHERE user_id = $1
               AND {EVENT_KIND} IS NOT NULL
               AND ($2::timestamptz IS NULL
                    OR {EVENT_AT} < $2
                    OR ({EVENT_AT} = $2 AND seq < $3))
             ORDER BY {EVENT_AT} DESC, seq DESC
             LIMIT $4",
            event_columns()
        ))
        .bind(user_id.as_uuid())
        .bind(before_at)
        .bind(before_seq)
        .bind(i64::from(limit.clamp(1, 100)))
        .fetch_all(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("read consent history", error))?;
        rows.into_iter().map(ConsentEventRow::into_domain).collect()
    }

    async fn insert(&self, draft: &ConsentEventDraft) -> Result<ConsentEvent, RepositoryError> {
        let row = sqlx::query_as::<_, ConsentEventRow>(&format!(
            "INSERT INTO consent_records (user_id, kind, action, occurred_at, policy_version)
             VALUES ($1, $2, $3, now(), $4)
             RETURNING {}",
            event_columns()
        ))
        .bind(draft.user_id().as_uuid())
        .bind(draft.kind().as_str())
        .bind(draft.action().as_str())
        .bind(draft.policy_version().as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|error| map_sqlx_error("append consent event", error))?;
        row.into_domain()
    }

    async fn begin_withdraw(
        &self,
        user_id: UserId,
        kind: ConsentKind,
    ) -> Result<Box<dyn ConsentTransaction>, RepositoryError> {
        let mut transaction = self
            .pool
            .begin()
            .await
            .map_err(|error| map_sqlx_error("begin withdrawal", error))?;
        // The timeout is set before the lock so a saturated pair cannot make the
        // request wait out the pool's 10 s statement ceiling.
        sqlx::query("SET LOCAL statement_timeout = '2s'")
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("bound withdrawal", error))?;
        // The serialisation point: a row lock is not enough under READ COMMITTED,
        // because the second transaction's snapshot predates the first insert.
        let key = format!("{user_id}:{}", kind.as_str());
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
            .bind(key)
            .execute(&mut *transaction)
            .await
            .map_err(|error| map_sqlx_error("lock withdrawal", error))?;
        Ok(Box::new(PgWithdrawTransaction {
            transaction,
            user_id,
        }))
    }
}

/// One open withdrawal transaction. The application decides, then commits or
/// rolls back.
struct PgWithdrawTransaction {
    transaction: Transaction<'static, Postgres>,
    user_id: UserId,
}

#[async_trait]
impl ConsentTransaction for PgWithdrawTransaction {
    async fn newest(&mut self, kind: ConsentKind) -> Result<Option<ConsentEvent>, RepositoryError> {
        let row = sqlx::query_as::<_, ConsentEventRow>(&format!(
            "SELECT {}
             FROM consent_records
             WHERE user_id = $1 AND {EVENT_KIND} = $2
             ORDER BY {EVENT_AT} DESC, seq DESC
             LIMIT 1",
            event_columns()
        ))
        .bind(self.user_id.as_uuid())
        .bind(kind.as_str())
        .fetch_optional(&mut *self.transaction)
        .await
        .map_err(|error| map_sqlx_error("read newest consent event", error))?;
        row.map(ConsentEventRow::into_domain).transpose()
    }

    async fn insert_event(
        &mut self,
        draft: &ConsentEventDraft,
    ) -> Result<ConsentEvent, RepositoryError> {
        let row = sqlx::query_as::<_, ConsentEventRow>(&format!(
            "INSERT INTO consent_records (user_id, kind, action, occurred_at, policy_version)
             VALUES ($1, $2, $3, now(), $4)
             RETURNING {}",
            event_columns()
        ))
        .bind(draft.user_id().as_uuid())
        .bind(draft.kind().as_str())
        .bind(draft.action().as_str())
        .bind(draft.policy_version().as_str())
        .fetch_one(&mut *self.transaction)
        .await
        .map_err(|error| map_sqlx_error("append withdrawal event", error))?;
        row.into_domain()
    }

    async fn enqueue_audit(&mut self, intent: &AuditIntent) -> Result<(), RepositoryError> {
        let payload = audit_payload(intent);
        sqlx::query(
            "INSERT INTO consent_audit_outbox
                 (consent_record_id, payload, idempotency_key, state, attempts,
                  next_attempt_at, created_at)
             VALUES ($1, $2, $3, 'pending', 0, now(), now())",
        )
        .bind(intent.event_id.as_uuid())
        .bind(payload)
        .bind(&intent.idempotency_key)
        .execute(&mut *self.transaction)
        .await
        .map_err(|error| map_sqlx_error("enqueue consent audit", error))?;
        Ok(())
    }

    async fn commit(self: Box<Self>) -> Result<(), RepositoryError> {
        let Self { transaction, .. } = *self;
        transaction
            .commit()
            .await
            .map_err(|error| map_sqlx_error("commit withdrawal", error))
    }

    async fn rollback(self: Box<Self>) -> Result<(), RepositoryError> {
        let Self { transaction, .. } = *self;
        transaction
            .rollback()
            .await
            .map_err(|error| map_sqlx_error("roll back withdrawal", error))
    }
}
