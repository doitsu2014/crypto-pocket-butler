//! The support-access audit outbox (BR3.13, NFR1.4).
//!
//! Rows are written by the `SupportGrantRepository` adapter in the same
//! transaction as the grant change, and drained to C10 by the drainer hosted in
//! `identity-api`. This module owns the write and the claim/backoff/dead-letter
//! statements; the delivery loop lives beside the consent and status sinks.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use identity_application::ports::{RepositoryError, SupportOutbox};
use identity_domain::AuditRecord;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::repositories::map_sqlx_error;

/// How many times one row is posted before it is marked dead.
pub const MAX_ATTEMPTS: i32 = 3;
/// How long a claimed row is hidden from other drainers.
const CLAIM_LEASE_SECONDS: i64 = 30;
/// How long a delivered row is kept for a manual replay.
const DELIVERED_RETENTION_HOURS: i32 = 24;

/// The delivery outcome of one row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportDelivery {
    /// C10 accepted the event.
    Delivered,
    /// The row is scheduled for another attempt.
    Retryable,
    /// The row will not be retried.
    Dead,
}

/// One claimed row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SupportOutboxRow {
    /// The outbox row identifier.
    pub id: Uuid,
    /// The C9 payload.
    pub payload: Value,
    /// The key C10 honours across retries.
    pub idempotency_key: String,
    /// How many times the row has been claimed.
    pub attempts: i32,
}

/// Reads and writes `support_audit_outbox`.
#[derive(Debug, Clone)]
pub struct SupportOutboxStore {
    pool: PgPool,
}

impl SupportOutboxStore {
    /// Creates the store over a pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Writes one pending row stamped with the caller's clock, so one request
    /// has one `now` across its statements.
    ///
    /// # Errors
    /// Returns the driver error when the insert fails.
    pub async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
        now: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        enqueue(&self.pool, record, correlation_id, now).await
    }

    /// Claims up to `limit` due rows under `FOR UPDATE SKIP LOCKED`, incrementing
    /// their attempt count and pushing the lease out by 30 seconds.
    ///
    /// # Errors
    /// Returns the driver error when the claim fails.
    pub async fn claim(
        &self,
        now: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<SupportOutboxRow>, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, SupportOutboxRow>(
            "UPDATE support_audit_outbox
             SET next_attempt_at = $1 + $3 * interval '1 second',
                 attempts = attempts + 1
             WHERE id IN (
                 SELECT id FROM support_audit_outbox
                 WHERE state = 'pending' AND next_attempt_at <= $1
                 ORDER BY next_attempt_at, id
                 LIMIT $2
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING id, payload, idempotency_key, attempts",
        )
        .bind(now)
        .bind(limit)
        .bind(CLAIM_LEASE_SECONDS)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(rows)
    }

    /// Marks a row delivered.
    ///
    /// # Errors
    /// Returns the driver error when the update fails.
    pub async fn mark_delivered(&self, id: Uuid, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE support_audit_outbox
             SET state = 'delivered', delivered_at = $2, last_error = NULL
             WHERE id = $1",
        )
        .bind(id)
        .bind(now)
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    /// Applies a failed delivery: a permanent failure or the third attempt is
    /// dead-lettered; anything else backs off.
    ///
    /// # Errors
    /// Returns the driver error when the update fails.
    pub async fn record_failure(
        &self,
        row: &SupportOutboxRow,
        now: DateTime<Utc>,
        detail: &str,
        transient: bool,
    ) -> Result<SupportDelivery, sqlx::Error> {
        if !transient || row.attempts >= MAX_ATTEMPTS {
            self.mark_dead(row.id, detail).await?;
            return Ok(SupportDelivery::Dead);
        }
        let next_attempt = now + chrono::Duration::seconds(backoff_seconds(row));
        sqlx::query(
            "UPDATE support_audit_outbox
             SET next_attempt_at = $2, last_error = $3
             WHERE id = $1",
        )
        .bind(row.id)
        .bind(next_attempt)
        .bind(truncate(detail, 300))
        .execute(&self.pool)
        .await?;
        Ok(SupportDelivery::Retryable)
    }

    /// Marks a row dead without another attempt.
    ///
    /// # Errors
    /// Returns the driver error when the update fails.
    pub async fn mark_dead(&self, id: Uuid, detail: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE support_audit_outbox
             SET state = 'dead', last_error = $2
             WHERE id = $1",
        )
        .bind(id)
        .bind(truncate(detail, 300))
        .execute(&self.pool)
        .await
        .map(|_| ())
    }

    /// Deletes delivered rows older than 24 hours and returns how many went.
    ///
    /// # Errors
    /// Returns the driver error when the delete fails.
    pub async fn cleanup(&self, now: DateTime<Utc>) -> Result<u64, sqlx::Error> {
        let removed = sqlx::query(
            "DELETE FROM support_audit_outbox
             WHERE state = 'delivered' AND delivered_at < $1 - $2 * interval '1 hour'",
        )
        .bind(now)
        .bind(DELIVERED_RETENTION_HOURS)
        .execute(&self.pool)
        .await?
        .rows_affected();
        Ok(removed)
    }
}

#[async_trait]
impl SupportOutbox for SupportOutboxStore {
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        enqueue(&self.pool, record, correlation_id, now)
            .await
            .map_err(|error| map_sqlx_error("append support audit intent", error))
    }
}

/// Inserts one pending row through any executor — a pool for a refusal's own
/// transaction, or the transaction that also writes the grant change.
///
/// # Errors
/// Returns the driver error when the insert fails.
pub async fn enqueue<'e, E>(
    executor: E,
    record: &AuditRecord,
    correlation_id: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let payload = payload_for(record, correlation_id);
    let idempotency_key = format!("support:{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO support_audit_outbox
             (payload, idempotency_key, state, attempts, next_attempt_at, created_at)
         VALUES ($1, $2, 'pending', 0, $3, $3)",
    )
    .bind(payload)
    .bind(idempotency_key)
    .bind(now)
    .execute(executor)
    .await
    .map(|_| ())
}

/// The payload C10 receives. Identifiers and enum values only — never the
/// reason body, a credential, a token, or an address.
#[must_use]
pub fn payload_for(record: &AuditRecord, correlation_id: &str) -> Value {
    serde_json::json!({
        "actorId": record.actor_id().map(|id| id.to_string()),
        "actorKind": "user",
        "action": record.action(),
        "target": record.target(),
        "occurredAt": record.occurred_at(),
        "outcome": record.outcome().as_str(),
        "context": record.context(),
        "correlationId": correlation_id,
        "sensitivity": "security",
    })
}

/// 30 s, 60 s, 120 s: exponential and bounded, with a per-row jitter derived
/// from the identifier so a batch does not retry in lockstep.
fn backoff_seconds(row: &SupportOutboxRow) -> i64 {
    let base = 30i64 << row.attempts.clamp(0, 5);
    let jitter = i64::from(row.id.as_bytes()[15] % 10);
    base + jitter
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
