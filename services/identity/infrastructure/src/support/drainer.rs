//! The support audit-outbox drainer (BR3.13, contract C5).
//!
//! Claims due rows under `FOR UPDATE SKIP LOCKED`, posts each with its
//! idempotency key, and applies the retry/dead bookkeeping. It runs inside the
//! `identity-api` process, never blocks a grant change, and a C10 outage only
//! grows the outbox. The pass reports its backlog to the monitoring sink after
//! every attempt, so the heartbeat advances even when the drainer is stuck.

use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use identity_application::ports::{SupportDrainDelivery, SupportDrainObservation};
use sqlx::PgPool;

use super::backlog::SupportOutboxBacklog;
use super::outbox::{SupportOutboxRow, SupportOutboxStore};

/// How many due rows one pass takes.
const DEFAULT_BATCH: i64 = 20;

/// One pass's counts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SupportDrainerReport {
    /// Rows C10 accepted.
    pub delivered: u32,
    /// Rows scheduled for another attempt.
    pub retryable: u32,
    /// Rows that will not be retried.
    pub dead: u32,
}

enum DeliveryError {
    Transient(String),
    Permanent(String),
}

/// Drains `support_audit_outbox` into C10.
pub struct SupportOutboxDrainer {
    store: SupportOutboxStore,
    backlog: SupportOutboxBacklog,
    client: reqwest::Client,
    audit_url: String,
    service_token: String,
    observation: Arc<dyn SupportDrainObservation>,
    interval: Duration,
    batch: i64,
}

impl SupportOutboxDrainer {
    /// Creates the drainer. The client carries the reliability design's 5 s
    /// connect / 10 s read budget.
    #[must_use]
    pub fn new(
        pool: PgPool,
        audit_url: impl Into<String>,
        service_token: impl Into<String>,
        observation: Arc<dyn SupportDrainObservation>,
        interval: Duration,
    ) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            store: SupportOutboxStore::new(pool.clone()),
            backlog: SupportOutboxBacklog::new(pool),
            client,
            audit_url: audit_url.into().trim_end_matches('/').to_string(),
            service_token: service_token.into(),
            observation,
            interval,
            batch: DEFAULT_BATCH,
        }
    }

    /// Runs one pass and records the backlog gauges. A drain failure is logged,
    /// never propagated to a caller.
    pub async fn run_once(&self, now: DateTime<Utc>) -> SupportDrainerReport {
        if let Err(error) = self.store.cleanup(now).await {
            tracing::warn!(%error, "support outbox cleanup failed");
        }
        let report = match self.drain(now).await {
            Ok(report) => report,
            Err(error) => {
                tracing::warn!(%error, "support outbox drain failed");
                SupportDrainerReport::default()
            }
        };
        match self.backlog.read(now).await {
            Ok((pending, oldest_age_seconds)) => {
                self.observation.record_pass(pending, oldest_age_seconds);
            }
            Err(error) => tracing::warn!(%error, "support outbox backlog read failed"),
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
                    "support audit outbox pass delivered"
                );
            }
        }
    }

    async fn drain(&self, now: DateTime<Utc>) -> Result<SupportDrainerReport, sqlx::Error> {
        let claimed = self.store.claim(now, self.batch).await?;
        let mut report = SupportDrainerReport::default();
        for row in claimed {
            let outcome = self.deliver(&row).await?;
            match outcome {
                SupportDrainDelivery::Delivered => report.delivered += 1,
                SupportDrainDelivery::Retryable => report.retryable += 1,
                SupportDrainDelivery::Dead => report.dead += 1,
            }
            self.observation.record_delivery(outcome);
        }
        Ok(report)
    }

    async fn deliver(&self, row: &SupportOutboxRow) -> Result<SupportDrainDelivery, sqlx::Error> {
        let now = Utc::now();
        match self.post(row).await {
            Ok(()) => {
                self.store.mark_delivered(row.id, now).await?;
                Ok(SupportDrainDelivery::Delivered)
            }
            Err(DeliveryError::Permanent(detail)) => {
                self.store.mark_dead(row.id, &detail).await?;
                Ok(SupportDrainDelivery::Dead)
            }
            Err(DeliveryError::Transient(detail)) => self
                .store
                .record_failure(row, now, &detail, true)
                .await
                .map(delivery_to_application),
        }
    }

    async fn post(&self, row: &SupportOutboxRow) -> Result<(), DeliveryError> {
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
}

fn delivery_to_application(outcome: super::outbox::SupportDelivery) -> SupportDrainDelivery {
    match outcome {
        super::outbox::SupportDelivery::Delivered => SupportDrainDelivery::Delivered,
        super::outbox::SupportDelivery::Retryable => SupportDrainDelivery::Retryable,
        super::outbox::SupportDelivery::Dead => SupportDrainDelivery::Dead,
    }
}

/// Whether a response status is worth retrying. A 4xx other than 429 is not.
fn is_transient(status: u16) -> bool {
    status == 429 || status >= 500
}
