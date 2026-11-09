//! WF1–WF2: the tick planner, the reaper and the call semantics.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use futures::stream::{self, StreamExt};
use job_orchestration_domain::{provider_for, JobRun, RunId, RunStatus, Schedule, Trigger};

use crate::common::AppError;
use crate::ports::{
    AuditClient, AuditEvent, Clock, HeartbeatRepository, IdGenerator, JobDefinition,
    LeaseRepository, RunRepository, TargetAnswer, TargetClient,
};

/// The lease TTL.
pub const LEASE_SECONDS: i64 = 30;
/// The most pending rows one tick drains.
pub const PENDING_PER_TICK: i64 = 200;
/// Missed windows are recovered only inside this lookback.
pub const LOOKBACK_HOURS: i64 = 24;
/// Outbound calls in flight.
pub const CONCURRENCY: usize = 32;

/// What one tick did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TickReport {
    /// Whether this instance held the lease.
    pub leased: bool,
    /// Windows or attempts enqueued.
    pub queued: usize,
    /// Calls attempted.
    pub executed: usize,
    /// Succeeded calls.
    pub succeeded: usize,
    /// Failed calls.
    pub failed: usize,
    /// Skipped calls.
    pub skipped: usize,
    /// Abandoned calls.
    pub abandoned: usize,
    /// Runs reaped from an expired lease.
    pub reaped: usize,
}

/// The tick planner.
#[derive(Clone)]
pub struct Scheduler {
    runs: Arc<dyn RunRepository>,
    lease: Arc<dyn LeaseRepository>,
    heartbeat: Arc<dyn HeartbeatRepository>,
    target: Arc<dyn TargetClient>,
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl Scheduler {
    /// Wires the ports.
    #[must_use]
    pub fn new(
        runs: Arc<dyn RunRepository>,
        lease: Arc<dyn LeaseRepository>,
        heartbeat: Arc<dyn HeartbeatRepository>,
        target: Arc<dyn TargetClient>,
        audit: Arc<dyn AuditClient>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            runs,
            lease,
            heartbeat,
            target,
            audit,
            clock,
            ids,
        }
    }

    /// Runs one tick for `owner`.
    ///
    /// # Errors
    /// Returns [`AppError::Unavailable`] when the store cannot answer.
    pub async fn tick(&self, owner: &str) -> Result<TickReport, AppError> {
        let now = self.clock.now();
        let mut report = TickReport::default();
        let claimed = self
            .lease
            .claim_or_renew(owner, now + Duration::seconds(LEASE_SECONDS), now)
            .await
            .map_err(AppError::from_repository)?;
        if !claimed {
            return Ok(report);
        }
        report.leased = true;
        self.heartbeat
            .write(owner, now)
            .await
            .map_err(AppError::from_repository)?;
        let definitions: BTreeMap<String, JobDefinition> = self
            .runs
            .definitions()
            .await
            .map_err(AppError::from_repository)?
            .into_iter()
            .map(|definition| (definition.name.clone(), definition))
            .collect();
        report.reaped = self
            .runs
            .reap_expired(now)
            .await
            .map_err(AppError::from_repository)?
            .len();
        report.queued += self.schedule_due(now, &definitions).await?;
        report.queued += self.queue_retries(now, &definitions).await?;
        let pending = self
            .runs
            .pending(PENDING_PER_TICK)
            .await
            .map_err(AppError::from_repository)?;
        let refresher = self.refresh_lease(owner);
        let outcomes: Vec<Option<RunStatus>> = stream::iter(pending.into_iter().map(|run| {
            let target = self.target.clone();
            let runs = self.runs.clone();
            let audit = self.audit.clone();
            let definition = definitions.get(&run.job).cloned();
            let owner = owner.to_string();
            async move { execute(run, definition, &owner, target, runs, audit, now).await }
        }))
        .buffer_unordered(CONCURRENCY)
        .collect()
        .await;
        refresher.abort();
        for outcome in outcomes {
            report.executed += 1;
            match outcome {
                Some(RunStatus::Succeeded) => report.succeeded += 1,
                Some(RunStatus::Failed) => report.failed += 1,
                Some(RunStatus::Skipped) => report.skipped += 1,
                Some(RunStatus::Abandoned) => report.abandoned += 1,
                _ => {}
            }
        }
        self.heartbeat
            .write(owner, self.clock.now())
            .await
            .map_err(AppError::from_repository)?;
        Ok(report)
    }

    fn refresh_lease(&self, owner: &str) -> tokio::task::JoinHandle<()> {
        let lease = self.lease.clone();
        let heartbeat = self.heartbeat.clone();
        let clock = self.clock.clone();
        let owner = owner.to_string();
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                ticker.tick().await;
                let now = clock.now();
                if let Ok(true) = lease
                    .claim_or_renew(&owner, now + Duration::seconds(LEASE_SECONDS), now)
                    .await
                {
                    if let Err(error) = heartbeat.write(&owner, now).await {
                        tracing::warn!(%error, "the heartbeat refresh failed");
                    }
                }
            }
        })
    }

    async fn schedule_due(
        &self,
        now: DateTime<Utc>,
        definitions: &BTreeMap<String, JobDefinition>,
    ) -> Result<usize, AppError> {
        let mut queued = 0;
        for definition in definitions.values() {
            if !definition.enabled {
                continue;
            }
            let schedule = Schedule::parse(&definition.schedule)
                .map_err(|error| AppError::Internal(error.to_string()))?;
            let window = schedule.window(now);
            if !self
                .runs
                .window_exists(&definition.name, window)
                .await
                .map_err(AppError::from_repository)?
            {
                queued += self
                    .enqueue(&definition.name, window, 1, Trigger::Scheduled, None, None)
                    .await?;
            }
            let previous = schedule.previous_window(window);
            if previous >= now - Duration::hours(LOOKBACK_HOURS)
                && !self
                    .runs
                    .window_exists(&definition.name, previous)
                    .await
                    .map_err(AppError::from_repository)?
            {
                queued += self
                    .enqueue(&definition.name, previous, 1, Trigger::Missed, None, None)
                    .await?;
            }
            self.runs
                .set_watermark(&definition.name, window)
                .await
                .map_err(AppError::from_repository)?;
        }
        Ok(queued)
    }

    async fn queue_retries(
        &self,
        now: DateTime<Utc>,
        definitions: &BTreeMap<String, JobDefinition>,
    ) -> Result<usize, AppError> {
        let mut queued = 0;
        for failed in self
            .runs
            .due_retries(now)
            .await
            .map_err(AppError::from_repository)?
        {
            let Some(definition) = definitions.get(&failed.job) else {
                continue;
            };
            if failed.attempt >= definition.max_attempts {
                continue;
            }
            queued += self
                .enqueue(
                    &failed.job,
                    failed.scheduled_for,
                    failed.attempt + 1,
                    failed.trigger,
                    failed.actor_id,
                    failed.reason.as_deref(),
                )
                .await?;
        }
        Ok(queued)
    }

    async fn enqueue(
        &self,
        job: &str,
        window: DateTime<Utc>,
        attempt: u32,
        trigger: Trigger,
        actor: Option<job_orchestration_domain::UserId>,
        reason: Option<&str>,
    ) -> Result<usize, AppError> {
        let run = JobRun::queued(
            RunId::new(self.ids.next()),
            job,
            window,
            attempt,
            trigger,
            actor,
            reason,
        )
        .map_err(AppError::from_domain)?;
        self.runs
            .insert(&run)
            .await
            .map_err(AppError::from_repository)?;
        Ok(1)
    }
}

/// The canonical idempotency key (BR1.4).
#[must_use]
pub fn idempotency_key(job: &str, window: DateTime<Utc>) -> String {
    format!("job:{job}:{}", window.format("%Y-%m-%dT%H:%M:%SZ"))
}

async fn execute(
    mut run: JobRun,
    definition: Option<JobDefinition>,
    owner: &str,
    target: Arc<dyn TargetClient>,
    runs: Arc<dyn RunRepository>,
    audit: Arc<dyn AuditClient>,
    now: DateTime<Utc>,
) -> Option<RunStatus> {
    let definition = definition?;
    let provider = provider_for(&run.job)?;
    let schedule = Schedule::parse(&definition.schedule).ok()?;
    let timeout = definition.timeout_seconds.clamp(1, 3_600);
    let lease_until =
        now + Duration::seconds(i64::try_from(timeout).unwrap_or(3_600) + LEASE_SECONDS);
    match runs.claim(run.id, owner, lease_until, now).await {
        Ok(true) => {}
        Ok(false) => return None,
        Err(error) => {
            tracing::warn!(%error, run = %run.id, "the pending run could not be claimed");
            return None;
        }
    }
    if let Err(error) = run.claim(owner, lease_until, now) {
        tracing::warn!(%error, run = %run.id, "the run could not move to running");
        return None;
    }
    let key = idempotency_key(&run.job, run.scheduled_for);
    let answer = target
        .run(provider, &run.job, run.scheduled_for, owner, &key, timeout)
        .await
        .unwrap_or(TargetAnswer::Unavailable);
    let failed_attempt = matches!(
        answer,
        TargetAnswer::Failed | TargetAnswer::LeaseHeld | TargetAnswer::Unavailable
    );
    let exhausted = failed_attempt && run.attempt >= definition.max_attempts;
    let (status, detail, response, next) = if exhausted {
        (
            RunStatus::Abandoned,
            "attempts exhausted".to_string(),
            None,
            None,
        )
    } else {
        classify(answer, &schedule, run.scheduled_for)
    };
    if let Err(error) = run.finish(status, response, Some(detail.as_str()), now, next) {
        tracing::warn!(%error, run = %run.id, "the run could not record its outcome");
        return None;
    }
    match runs.finish(&run, owner, now).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::debug!(run = %run.id, "the run outcome was discarded: the lease moved");
            return None;
        }
        Err(error) => {
            tracing::warn!(%error, run = %run.id, "the run outcome could not be stored");
            return None;
        }
    }
    let (action, outcome) = match (run.actor_id, status) {
        (None, _) => ("system.job.run".to_string(), "success".to_string()),
        (Some(_), RunStatus::Succeeded | RunStatus::Skipped) => {
            ("job.run".to_string(), "success".to_string())
        }
        (Some(_), _) => ("job.run".to_string(), "failed".to_string()),
    };
    let event = AuditEvent {
        actor_id: run.actor_id,
        action,
        target: format!("job:{}:{}", run.job, run.scheduled_for.to_rfc3339()),
        outcome,
        occurred_at: now,
    };
    if let Err(error) = audit.append(&event).await {
        tracing::warn!(%error, run = %run.id, "the run event was not audited");
    }
    Some(status)
}

fn classify(
    answer: TargetAnswer,
    schedule: &Schedule,
    window: DateTime<Utc>,
) -> (RunStatus, String, Option<u16>, Option<DateTime<Utc>>) {
    let retry = Some(schedule.retry_at(window));
    match answer {
        TargetAnswer::Succeeded => (
            RunStatus::Succeeded,
            "succeeded".to_string(),
            Some(200),
            None,
        ),
        TargetAnswer::Skipped => (
            RunStatus::Skipped,
            "target skipped".to_string(),
            Some(200),
            None,
        ),
        TargetAnswer::AlreadyRunning => (
            RunStatus::Skipped,
            "run active".to_string(),
            Some(202),
            None,
        ),
        TargetAnswer::LeaseHeld => (
            RunStatus::Failed,
            "lease held".to_string(),
            Some(409),
            retry,
        ),
        TargetAnswer::Failed => (
            RunStatus::Failed,
            "target reported failure".to_string(),
            Some(200),
            retry,
        ),
        TargetAnswer::Unavailable => (
            RunStatus::Failed,
            "target unavailable".to_string(),
            None,
            retry,
        ),
        TargetAnswer::Refused(status) => (
            RunStatus::Abandoned,
            "target refused the call".to_string(),
            Some(status),
            None,
        ),
    }
}
