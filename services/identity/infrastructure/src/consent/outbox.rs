//! The audit-outbox drainer: due rows under `FOR UPDATE SKIP LOCKED`, one C9
//! post per row with its idempotency key, and the retry/dead bookkeeping
//! (reliability design; contract amendment: C9 honours `Idempotency-Key`).
//!
//! Two outboxes share the one drainer and its client: `consent_audit_outbox`
//! (U4) and `status_audit_outbox` (U5, BR5.9). Each pass drains both; a failure
//! in one never blocks the other or a caller's transition.
//!
//! The drainer is a unit of work in the `identity-api` process, not a separate
//! deployable (infrastructure specification). It never blocks a withdrawal or a
//! status change: a C9 outage only grows the outbox.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use identity_application::consent::ports::{DeliveryOutcome, DrainerObservation};
use sqlx::PgPool;
use uuid::Uuid;

/// How many times one row is posted before it is marked dead.
const MAX_ATTEMPTS: i32 = 3;
/// How many due rows one pass takes.
const DEFAULT_BATCH: i64 = 20;
/// How long a claimed row is hidden from other drainers. Longer than the 10 s
/// HTTP budget, so a slow post is never double-claimed.
const CLAIM_LEASE_SECONDS: i64 = 30;
/// How long a delivered row is kept for a manual replay.
const DELIVERED_RETENTION_HOURS: i32 = 24;

/// The consent event outbox (U4).
const CONSENT_OUTBOX: &str = "consent_audit_outbox";
/// The account status-transition outbox (U5, BR5.9).
const STATUS_OUTBOX: &str = "status_audit_outbox";

/// One pass's counts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DrainerReport {
    /// Rows C9 accepted.
    pub delivered: u32,
    /// Rows scheduled for another attempt.
    pub retryable: u32,
    /// Rows that will not be retried.
    pub dead: u32,
}

impl DrainerReport {
    fn merge(&mut self, other: Self) {
        self.delivered += other.delivered;
        self.retryable += other.retryable;
        self.dead += other.dead;
    }
}

enum DeliveryError {
    Transient(String),
    Permanent(String),
}

#[derive(sqlx::FromRow)]
struct OutboxRow {
    id: Uuid,
    payload: serde_json::Value,
    idempotency_key: String,
    attempts: i32,
}

/// Drains `consent_audit_outbox` into C9.
pub struct AuditOutboxDrainer {
    pool: PgPool,
    client: reqwest::Client,
    audit_url: String,
    service_token: String,
    observation: Arc<dyn DrainerObservation>,
    interval: Duration,
    batch: i64,
}

impl AuditOutboxDrainer {
    /// Creates the drainer. The Client carries the reliability design's 5 s
    /// connect / 10 s read budget.
    #[must_use]
    pub fn new(
        pool: PgPool,
        audit_url: impl Into<String>,
        service_token: impl Into<String>,
        observation: Arc<dyn DrainerObservation>,
        interval: Duration,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            pool,
            client,
            audit_url: audit_url.into().trim_end_matches('/').to_string(),
            service_token: service_token.into(),
            observation,
            interval,
            batch: DEFAULT_BATCH,
        }
    }

    /// Runs one pass and records the backlog gauges. A drain failure is logged
    /// and counted, never propagated to a caller.
    pub async fn run_once(&self, now: DateTime<Utc>) -> DrainerReport {
        if let Err(error) = self.cleanup(now).await {
            tracing::warn!(%error, "consent outbox cleanup failed");
        }
        let mut report = match self.process(now).await {
            Ok(report) => report,
            Err(error) => {
                tracing::warn!(%error, "consent outbox drain failed");
                DrainerReport::default()
            }
        };
        match self.backlog(now).await {
            Ok((pending, oldest_age_seconds)) => {
                self.observation.record_pass(pending, oldest_age_seconds);
            }
            Err(error) => tracing::warn!(%error, "consent outbox backlog read failed"),
        }

        // U5: the same pass drains the status-transition outbox (BR5.9). A
        // failure here never blocks the consent drain or a transition.
        match self.process_status(now).await {
            Ok(status_report) => report.merge(status_report),
            Err(error) => tracing::warn!(%error, "status outbox drain failed"),
        }
        match self.status_backlog(now).await {
            Ok((pending, oldest_age_seconds)) => {
                self.observation
                    .record_status_pass(pending, oldest_age_seconds);
            }
            Err(error) => tracing::warn!(%error, "status outbox backlog read failed"),
        }
        report
    }

    /// Loops until the task is aborted. The interval is the drain cadence.
    pub async fn run_forever(self: Arc<Self>) {
        let mut ticker = tokio::time::interval(self.interval);
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            let report = self.run_once(Utc::now()).await;
            if report.delivered + report.retryable + report.dead > 0 {
                tracing::info!(
                    delivered = report.delivered,
                    retryable = report.retryable,
                    dead = report.dead,
                    "consent audit outbox pass delivered"
                );
            }
        }
    }

    async fn process(&self, now: DateTime<Utc>) -> Result<DrainerReport, sqlx::Error> {
        self.drain(CONSENT_OUTBOX, now, |row, outcome| {
            self.observation.record_delivery(outcome);
            let _ = row;
        })
        .await
    }

    async fn process_status(&self, now: DateTime<Utc>) -> Result<DrainerReport, sqlx::Error> {
        self.drain(STATUS_OUTBOX, now, |_, outcome| {
            self.observation.record_status_delivery(outcome);
        })
        .await
    }

    async fn drain(
        &self,
        table: &'static str,
        now: DateTime<Utc>,
        observe: impl Fn(&OutboxRow, DeliveryOutcome),
    ) -> Result<DrainerReport, sqlx::Error> {
        // Claim the batch in one short transaction. The `FOR UPDATE SKIP LOCKED`
        // lock is released at commit; the lease in `next_attempt_at` keeps other
        // drainers off the rows while the posts happen outside any transaction.
        let claimed = self.claim(table, now).await?;

        let mut report = DrainerReport::default();
        for row in claimed {
            let outcome = self.deliver(table, &row, now).await?;
            match outcome {
                DeliveryOutcome::Delivered => report.delivered += 1,
                DeliveryOutcome::Retryable => report.retryable += 1,
                DeliveryOutcome::Dead => report.dead += 1,
            }
            observe(&row, outcome);
        }
        Ok(report)
    }

    async fn claim(
        &self,
        table: &'static str,
        now: DateTime<Utc>,
    ) -> Result<Vec<OutboxRow>, sqlx::Error> {
        let mut transaction = self.pool.begin().await?;
        let rows = sqlx::query_as::<_, OutboxRow>(&format!(
            "UPDATE {table}
             SET next_attempt_at = $1 + $3 * interval '1 second',
                 attempts = attempts + 1
             WHERE id IN (
                 SELECT id FROM {table}
                 WHERE state = 'pending' AND next_attempt_at <= $1
                 ORDER BY next_attempt_at, id
                 LIMIT $2
                 FOR UPDATE SKIP LOCKED
             )
             RETURNING id, payload, idempotency_key, attempts"
        ))
        .bind(now)
        .bind(self.batch)
        .bind(CLAIM_LEASE_SECONDS)
        .fetch_all(&mut *transaction)
        .await?;
        transaction.commit().await?;
        Ok(rows)
    }

    async fn deliver(
        &self,
        table: &'static str,
        row: &OutboxRow,
        now: DateTime<Utc>,
    ) -> Result<DeliveryOutcome, sqlx::Error> {
        if !payload_is_safe(&row.payload) {
            mark_dead(&self.pool, table, row, "payload refused by the deny-list").await?;
            return Ok(DeliveryOutcome::Dead);
        }
        match self.post(row).await {
            Ok(()) => {
                // Each outcome is its own short transaction; no connection is
                // held across the C9 call.
                sqlx::query(&format!(
                    "UPDATE {table}
                     SET state = 'delivered', delivered_at = $2, last_error = NULL
                     WHERE id = $1"
                ))
                .bind(row.id)
                .bind(now)
                .execute(&self.pool)
                .await?;
                Ok(DeliveryOutcome::Delivered)
            }
            Err(DeliveryError::Permanent(detail)) => {
                mark_dead(&self.pool, table, row, &detail).await?;
                Ok(DeliveryOutcome::Dead)
            }
            Err(DeliveryError::Transient(detail)) => {
                if row.attempts >= MAX_ATTEMPTS {
                    mark_dead(&self.pool, table, row, &detail).await?;
                    return Ok(DeliveryOutcome::Dead);
                }
                let next_attempt = now + chrono::Duration::seconds(backoff_seconds(row));
                sqlx::query(&format!(
                    "UPDATE {table}
                     SET next_attempt_at = $2, last_error = $3
                     WHERE id = $1"
                ))
                .bind(row.id)
                .bind(next_attempt)
                .bind(truncate(&detail, 300))
                .execute(&self.pool)
                .await?;
                Ok(DeliveryOutcome::Retryable)
            }
        }
    }

    async fn post(&self, row: &OutboxRow) -> Result<(), DeliveryError> {
        let url = format!("{}/v1/events", self.audit_url);
        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.service_token)
            .header("idempotency-key", &row.idempotency_key)
            .json(&row.payload)
            .send()
            .await;
        match response {
            Ok(response) if response.status().is_success() => Ok(()),
            Ok(response) => {
                let status = response.status().as_u16();
                if is_transient(status) {
                    Err(DeliveryError::Transient(format!(
                        "the audit service answered {status}"
                    )))
                } else {
                    Err(DeliveryError::Permanent(format!(
                        "the audit service refused the event with {status}"
                    )))
                }
            }
            Err(error) => Err(DeliveryError::Transient(format!(
                "the audit service is unreachable: {error}"
            ))),
        }
    }

    async fn cleanup(&self, now: DateTime<Utc>) -> Result<(), sqlx::Error> {
        for table in [CONSENT_OUTBOX, STATUS_OUTBOX] {
            sqlx::query(&format!(
                "DELETE FROM {table}
                 WHERE state = 'delivered' AND delivered_at < $1 - $2 * interval '1 hour'"
            ))
            .bind(now)
            .bind(DELIVERED_RETENTION_HOURS)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn backlog(&self, now: DateTime<Utc>) -> Result<(u64, i64), sqlx::Error> {
        backlog_for(&self.pool, CONSENT_OUTBOX, now).await
    }

    async fn status_backlog(&self, now: DateTime<Utc>) -> Result<(u64, i64), sqlx::Error> {
        backlog_for(&self.pool, STATUS_OUTBOX, now).await
    }
}

async fn backlog_for(
    pool: &PgPool,
    table: &'static str,
    now: DateTime<Utc>,
) -> Result<(u64, i64), sqlx::Error> {
    let (pending, oldest_age): (i64, i64) = sqlx::query_as(&format!(
        "SELECT count(*)::bigint,
                COALESCE(EXTRACT(EPOCH FROM ($1 - MIN(created_at)))::bigint, 0)
         FROM {table} WHERE state = 'pending'"
    ))
    .bind(now)
    .fetch_one(pool)
    .await?;
    Ok((pending.max(0) as u64, oldest_age.max(0)))
}

async fn mark_dead(
    pool: &PgPool,
    table: &'static str,
    row: &OutboxRow,
    detail: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(&format!(
        "UPDATE {table}
         SET state = 'dead', last_error = $2
         WHERE id = $1"
    ))
    .bind(row.id)
    .bind(truncate(detail, 300))
    .execute(pool)
    .await?;
    Ok(())
}

/// 30 s, 60 s, 120 s: exponential and bounded, with a per-row jitter derived
/// from the identifier so a batch does not retry in lockstep.
fn backoff_seconds(row: &OutboxRow) -> i64 {
    let base = 30i64 << row.attempts.clamp(0, 5);
    let jitter = i64::from(row.id.as_bytes()[15] % 10);
    base + jitter
}

fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

/// Whether a response status is worth retrying. A 4xx is not.
fn is_transient(status: u16) -> bool {
    status == 429 || status >= 500
}

/// The deny-list backstop (security design): a token-shaped or wallet-shaped
/// string must never leave the process, so the row is marked dead unsent.
fn payload_is_safe(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(text) => !is_token_shaped(text) && !is_wallet_shaped(text),
        serde_json::Value::Array(items) => items.iter().all(payload_is_safe),
        serde_json::Value::Object(fields) => fields.values().all(payload_is_safe),
        _ => true,
    }
}

fn is_token_shaped(value: &str) -> bool {
    value.starts_with("eyJ")
}

fn is_wallet_shaped(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("0x") else {
        return false;
    };
    hex.len() == 40 && hex.chars().all(|character| character.is_ascii_hexdigit())
}
