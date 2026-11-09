//! WF1–WF4, BR1.1–BR1.3, BR2.1–BR2.3 — the brief-generation run.
//!
//! One run covers every user C3 reports due: it derives each user's local window,
//! composes the brief from C8 and C7, stores it once per window, and submits an
//! email alert through C9. A brief that cannot be produced is recorded as failed
//! with an explicit reason; a brief whose figures are degraded is still produced
//! and marked partial or unavailable.

use std::sync::Arc;
use std::time::{Duration as StdDuration, Instant};

use chrono::{DateTime, Duration, Utc};
use futures::stream::{self, StreamExt};
use reporting_domain::{
    AuditOutcome, AuditRecord, Brief, BriefDelivery, DataStatus, DeliveryChannel, DeliveryStatus,
    Period, UserId,
};
use serde_json::Value;
use uuid::Uuid;

use crate::common::{AppError, ServiceCaller};
use crate::ports::{
    AuditClient, BriefRepository, BriefRun, Clock, DeliveryRepository, DueUsersClient, IdGenerator,
    NotificationClient, PerformanceClient, RepositoryError, RunRepository, RunStatus,
    ValuationClient,
};
use crate::sections::{email_payload, sections_json};
use crate::window::window;

/// The C11 job name.
pub const RUN_JOB: &str = "brief-generation";
/// The lease length, renewed while the run is in flight.
pub const LEASE_SECONDS: i64 = 1_800;
/// How often the lease is renewed.
pub const RENEW_SECONDS: u64 = 60;
/// How many briefs generation keeps in flight.
pub const CONCURRENCY: usize = 32;
/// The C9 alert type for a ready brief. It is exactly `brief`, the name
/// Notification's closed type set accepts; any other value is refused 422.
pub const ALERT_TYPE: &str = "brief";
/// A brief is announced by email only, so it does not add a second in-app entry.
pub const EMAIL_CHANNEL: &str = "email";
/// A failed brief is retried once it is this old; a younger failure is not
/// hammered (BR1.1).
pub const RETRY_AFTER_MINUTES: i64 = 60;

/// What one run did.
#[derive(Debug, Clone, PartialEq)]
pub struct RunReport {
    /// How many (user, period) pairs were due.
    pub due: usize,
    /// How many briefs were produced.
    pub generated: usize,
    /// How many briefs could not be produced.
    pub failed: usize,
    /// How many due windows already had a brief.
    pub skipped: usize,
    /// How many email alerts Notification accepted.
    pub email_submitted: usize,
    /// How many email alerts could not be delivered.
    pub email_failed: usize,
    /// How long the run took.
    pub duration_seconds: f64,
}

impl RunReport {
    fn empty() -> Self {
        Self {
            due: 0,
            generated: 0,
            failed: 0,
            skipped: 0,
            email_submitted: 0,
            email_failed: 0,
            duration_seconds: 0.0,
        }
    }
}

/// One (user, period) window the run must produce.
#[derive(Debug, Clone)]
struct Job {
    user_id: UserId,
    period: Period,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
}

/// One job's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Outcome {
    Generated,
    Failed,
    Skipped,
    EmailSubmitted,
    EmailFailed,
}

/// Produces the scheduled briefs.
#[derive(Clone)]
pub struct GenerationService {
    briefs: Arc<dyn BriefRepository>,
    deliveries: Arc<dyn DeliveryRepository>,
    due_users: Arc<dyn DueUsersClient>,
    valuation: Arc<dyn ValuationClient>,
    performance: Arc<dyn PerformanceClient>,
    notifications: Arc<dyn NotificationClient>,
    audit: Arc<dyn AuditClient>,
    runs: Arc<dyn RunRepository>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl GenerationService {
    /// Wires the ports.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        briefs: Arc<dyn BriefRepository>,
        deliveries: Arc<dyn DeliveryRepository>,
        due_users: Arc<dyn DueUsersClient>,
        valuation: Arc<dyn ValuationClient>,
        performance: Arc<dyn PerformanceClient>,
        notifications: Arc<dyn NotificationClient>,
        audit: Arc<dyn AuditClient>,
        runs: Arc<dyn RunRepository>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            briefs,
            deliveries,
            due_users,
            valuation,
            performance,
            notifications,
            audit,
            runs,
            clock,
            ids,
        }
    }

    /// Runs the C11 job on behalf of an allow-listed caller.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] when the caller may not run it and
    /// [`AppError::Conflict`] when another run holds the lease.
    pub async fn run(&self, caller: &ServiceCaller, owner: &str) -> Result<RunReport, AppError> {
        caller.require_operation(reporting_domain::Operation::RunBriefGeneration)?;
        self.execute(owner).await
    }

    /// Runs the job from the worker's own schedule.
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] when another run holds the lease.
    pub async fn run_scheduled(&self, owner: &str) -> Result<RunReport, AppError> {
        self.execute(owner).await
    }

    async fn execute(&self, owner: &str) -> Result<RunReport, AppError> {
        let started = Instant::now();
        let now = self.clock.now();
        let run = BriefRun {
            id: self.ids.next(),
            job: RUN_JOB.to_string(),
            owner: owner.to_string(),
            started_at: now,
            lease_until: now + Duration::seconds(LEASE_SECONDS),
            finished_at: None,
            status: RunStatus::Running,
            detail: None,
        };
        let claimed = self
            .runs
            .claim(&run)
            .await
            .map_err(AppError::from_repository)?;
        if !claimed {
            return Err(AppError::Conflict(format!(
                "another {RUN_JOB} run holds the lease"
            )));
        }
        let sweeper = self.renewal(run.id);
        let result = self.execute_claimed(&run, now).await;
        if let Some(handle) = sweeper {
            handle.abort();
        }
        let mut finished = run.clone();
        finished.finished_at = Some(self.clock.now());
        finished.status = if result.is_ok() {
            RunStatus::Succeeded
        } else {
            RunStatus::Failed
        };
        finished.detail = result.as_ref().err().map(|error| match error {
            AppError::Conflict(detail)
            | AppError::Forbidden(detail)
            | AppError::Unavailable(detail)
            | AppError::Internal(detail) => detail.clone(),
            other => other.to_string(),
        });
        if let Err(error) = self.runs.finish(&finished).await {
            tracing::error!(%error, "the brief-generation run could not record its outcome");
        }
        let mut report = result?;
        report.duration_seconds = started.elapsed().as_secs_f64();
        Ok(report)
    }

    fn renewal(&self, run_id: Uuid) -> Option<tokio::task::JoinHandle<()>> {
        let runs = self.runs.clone();
        let clock = self.clock.clone();
        Some(tokio::spawn(async move {
            let mut ticker = tokio::time::interval(StdDuration::from_secs(RENEW_SECONDS));
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                ticker.tick().await;
                let now = clock.now();
                match runs
                    .renew(run_id, now + Duration::seconds(LEASE_SECONDS), now)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => break,
                    Err(error) => {
                        tracing::warn!(%error, "the brief-generation lease could not be renewed");
                    }
                }
            }
        }))
    }

    async fn execute_claimed(
        &self,
        run: &BriefRun,
        now: DateTime<Utc>,
    ) -> Result<RunReport, AppError> {
        if let Err(error) = self.runs.sweep_expired(now).await {
            tracing::warn!(%error, "the expired-run sweep failed");
        }
        let mut report = RunReport::empty();
        let jobs = self.collect_jobs(now, &mut report).await?;
        report.due = jobs.len();

        let briefs = self.briefs.clone();
        let deliveries = self.deliveries.clone();
        let valuation = self.valuation.clone();
        let performance = self.performance.clone();
        let notifications = self.notifications.clone();
        let audit = self.audit.clone();
        let clock = self.clock.clone();
        let ids = self.ids.clone();
        let cutoff = now - Duration::minutes(RETRY_AFTER_MINUTES);

        let outcomes: Vec<Vec<Outcome>> = stream::iter(jobs.into_iter().map(|job| {
            let briefs = briefs.clone();
            let deliveries = deliveries.clone();
            let valuation = valuation.clone();
            let performance = performance.clone();
            let notifications = notifications.clone();
            let audit = audit.clone();
            let clock = clock.clone();
            let ids = ids.clone();
            async move {
                produce(
                    job,
                    cutoff,
                    briefs,
                    deliveries,
                    valuation,
                    performance,
                    notifications,
                    audit,
                    clock,
                    ids,
                )
                .await
            }
        }))
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;

        for group in outcomes {
            for outcome in group {
                match outcome {
                    Outcome::Generated => report.generated += 1,
                    Outcome::Failed => report.failed += 1,
                    Outcome::Skipped => report.skipped += 1,
                    Outcome::EmailSubmitted => report.email_submitted += 1,
                    Outcome::EmailFailed => report.email_failed += 1,
                }
            }
        }
        tracing::info!(
            run = %run.id,
            due = report.due,
            generated = report.generated,
            failed = report.failed,
            skipped = report.skipped,
            "brief generation finished"
        );
        Ok(report)
    }

    async fn collect_jobs(
        &self,
        now: DateTime<Utc>,
        report: &mut RunReport,
    ) -> Result<Vec<Job>, AppError> {
        let mut jobs: Vec<Job> = Vec::new();
        for period in [Period::Daily, Period::Weekly] {
            let due = self
                .due_users
                .due(period, now)
                .await
                .map_err(AppError::from_external)?;
            for user in due {
                match window(period, &user.timezone, now) {
                    Some((period_start, period_end)) => jobs.push(Job {
                        user_id: user.user_id,
                        period,
                        period_start,
                        period_end,
                    }),
                    None => {
                        // A window that cannot be resolved is a failed brief,
                        // not a silently skipped user (BR2.1).
                        if let Err(error) =
                            self.record_unresolvable(&user.user_id, period, now).await
                        {
                            tracing::warn!(%error, "an unresolvable window could not be recorded");
                        }
                        report.failed += 1;
                    }
                }
            }
        }
        Ok(jobs)
    }

    async fn record_unresolvable(
        &self,
        user_id: &UserId,
        period: Period,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        let mut brief = Brief::new(
            reporting_domain::BriefId::new(self.ids.next()),
            *user_id,
            period,
            now - period_length(period),
            now,
            now,
        )
        .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        brief
            .mark_failed("the local period window could not be resolved", now)
            .map_err(|error| RepositoryError::InvalidData(error.to_string()))?;
        self.briefs.upsert_failed(&brief).await?;
        Ok(())
    }
}

fn period_length(period: Period) -> Duration {
    match period {
        Period::Daily => Duration::days(1),
        Period::Weekly => Duration::days(7),
    }
}

/// Produces one brief and its delivery attempt.
#[allow(clippy::too_many_arguments)]
async fn produce(
    job: Job,
    retry_cutoff: DateTime<Utc>,
    briefs: Arc<dyn BriefRepository>,
    deliveries: Arc<dyn DeliveryRepository>,
    valuation: Arc<dyn ValuationClient>,
    performance: Arc<dyn PerformanceClient>,
    notifications: Arc<dyn NotificationClient>,
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
) -> Vec<Outcome> {
    let now = clock.now();
    match briefs
        .find_window(job.user_id, job.period, job.period_start)
        .await
    {
        Ok(Some(existing)) => {
            // BR1.1: a failed or hour-old pending brief is retried; a generated
            // one is never regenerated.
            let stale = existing.brief.status() != reporting_domain::BriefStatus::Generated;
            if !stale || existing.brief.updated_at() > retry_cutoff {
                return vec![Outcome::Skipped];
            }
        }
        Ok(None) => {}
        Err(error) => {
            tracing::warn!(%error, "the brief lookup failed");
            return vec![Outcome::Failed];
        }
    }

    let mut brief = match Brief::new(
        reporting_domain::BriefId::new(ids.next()),
        job.user_id,
        job.period,
        job.period_start,
        job.period_end,
        now,
    ) {
        Ok(brief) => brief,
        Err(error) => {
            tracing::warn!(%error, "the brief window was refused");
            return vec![Outcome::Failed];
        }
    };

    let valuation = match valuation.valuation(job.user_id, now).await {
        Ok(read) => read,
        Err(error) => {
            record_failure(
                &mut brief,
                now,
                "the portfolio valuation was unavailable",
                &briefs,
                &audit,
            )
            .await;
            tracing::warn!(%error, "the portfolio valuation read failed");
            return vec![Outcome::Failed];
        }
    };
    let performance = match performance
        .performance(job.user_id, job.period_start, job.period_end)
        .await
    {
        Ok(read) => read,
        Err(error) => {
            tracing::warn!(%error, "the performance read failed; the brief is degraded");
            reporting_domain::PnlRead {
                realized: None,
                net_invested: None,
                unrealized: None,
                coverage: None,
                currency: None,
                fx_as_of: None,
                fx_estimated: false,
                as_of: now,
                status: DataStatus::Unavailable,
            }
        }
    };

    let composed = reporting_domain::compose_sections(job.period, &valuation, &performance);
    let sections = sections_json(&composed);
    let has_unpriced = composed.net_worth.unpriced_count > 0;
    if let Err(error) = brief.mark_generated(valuation.as_of, composed.data_status, now) {
        tracing::warn!(%error, "the brief could not be marked generated");
        return vec![Outcome::Failed];
    }
    let stored = match briefs.upsert_generated(&brief, &sections).await {
        Ok(stored) => stored,
        Err(error) => {
            tracing::warn!(%error, "the brief could not be stored");
            return vec![Outcome::Failed];
        }
    };
    if stored.status() != reporting_domain::BriefStatus::Generated {
        // A concurrent run stored the window first.
        return vec![Outcome::Skipped];
    }
    let mut outcomes = vec![Outcome::Generated];

    // BR2.2: every generated brief submits an email alert; Notification decides
    // whether to suppress it.
    let idempotency_key = format!("brief:{}:email", stored.id());
    let payload = email_payload(
        &stored.id().to_string(),
        stored.period().as_str(),
        &valuation.as_of.to_rfc3339(),
        composed.data_status.as_str(),
        has_unpriced,
    );
    let delivery = match notifications
        .notify(
            job.user_id,
            ALERT_TYPE,
            &idempotency_key,
            EMAIL_CHANNEL,
            payload,
        )
        .await
    {
        Ok(Some(alert_id)) => {
            outcomes.push(Outcome::EmailSubmitted);
            BriefDelivery::new(
                reporting_domain::BriefDeliveryId::new(ids.next()),
                stored.id(),
                DeliveryChannel::Email,
                Some(alert_id),
                DeliveryStatus::Submitted,
                now,
            )
        }
        Ok(None) => BriefDelivery::new(
            reporting_domain::BriefDeliveryId::new(ids.next()),
            stored.id(),
            DeliveryChannel::Email,
            None,
            DeliveryStatus::Suppressed,
            now,
        ),
        Err(error) => {
            outcomes.push(Outcome::EmailFailed);
            tracing::warn!(%error, "the brief email alert was not delivered");
            BriefDelivery::new(
                reporting_domain::BriefDeliveryId::new(ids.next()),
                stored.id(),
                DeliveryChannel::Email,
                None,
                DeliveryStatus::Failed,
                now,
            )
        }
    };
    if let Ok(delivery) = delivery {
        if let Err(error) = deliveries.upsert(&delivery).await {
            tracing::warn!(%error, "the delivery attempt could not be stored");
        }
        let (action, outcome) = match delivery.status() {
            DeliveryStatus::Failed => ("reporting.brief.failed", AuditOutcome::Failed),
            DeliveryStatus::Submitted | DeliveryStatus::Suppressed => {
                ("reporting.brief.email_submitted", AuditOutcome::Success)
            }
        };
        if let Ok(record) = AuditRecord::new(
            None,
            action,
            format!("brief:{}", stored.id()),
            outcome,
            now,
            std::collections::BTreeMap::from([(
                "delivery".to_string(),
                delivery.status().as_str().to_string(),
            )]),
        ) {
            audit_with_fallback(&audit, &record, "brief-generation").await;
        }
    }
    outcomes
}

/// Appends a reporting event, falling back to the one allow-listed umbrella
/// action when C10 refuses the finer action.
async fn audit_with_fallback(
    audit: &Arc<dyn AuditClient>,
    record: &AuditRecord,
    correlation: &str,
) {
    if audit.append(record, correlation).await.is_ok() {
        return;
    }
    let fallback = AuditRecord::new(
        record.actor_id(),
        "system.reporting.brief",
        record.target().to_string(),
        record.outcome(),
        record.occurred_at(),
        record.context().clone(),
    );
    match fallback {
        Ok(fallback) => {
            if let Err(error) = audit.append(&fallback, correlation).await {
                tracing::warn!(%error, "the fallback brief event was not audited");
            }
        }
        Err(error) => tracing::warn!(%error, "the fallback audit record was refused"),
    }
}

async fn record_failure(
    brief: &mut Brief,
    now: DateTime<Utc>,
    reason: &str,
    briefs: &Arc<dyn BriefRepository>,
    audit: &Arc<dyn AuditClient>,
) {
    if let Err(error) = brief.mark_failed(reason, now) {
        tracing::warn!(%error, "the brief could not be marked failed");
        return;
    }
    if let Err(error) = briefs.upsert_failed(brief).await {
        tracing::warn!(%error, "the failed brief could not be stored");
        return;
    }
    let record = AuditRecord::new(
        None,
        "reporting.brief.failed",
        format!("brief:{}", brief.id()),
        AuditOutcome::Failed,
        now,
        std::collections::BTreeMap::from([
            ("period".to_string(), brief.period().as_str().to_string()),
            ("periodStart".to_string(), brief.period_start().to_rfc3339()),
        ]),
    );
    if let Ok(record) = record {
        audit_with_fallback(audit, &record, "brief-generation").await;
    }
}

/// Reads a value the C8 adapter projected, exposed for a focused test.
#[must_use]
pub fn is_partial(value: &Value) -> bool {
    value.get("dataStatus").and_then(Value::as_str) == Some("partial")
}
