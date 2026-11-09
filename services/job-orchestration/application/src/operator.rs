//! WF3: operator reads, manual runs and backfill.

use std::sync::Arc;

use chrono::{DateTime, Duration, Utc};
use job_orchestration_domain::{JobRun, RunId, Schedule, Trigger, UserId};

use crate::common::{AppError, Caller};
use crate::ports::{AuditClient, AuditEvent, Clock, IdGenerator, RunRepository};

/// The backfill width cap.
pub const MAX_BACKFILL_DAYS: i64 = 7;
/// The most pages one backfill walks.
pub const MAX_BACKFILL_WINDOWS: usize = 5_000;
/// The default run page size.
pub const DEFAULT_PAGE: i64 = 100;
/// The largest run page.
pub const MAX_PAGE: i64 = 500;

/// One job with its latest attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobView {
    /// The job name.
    pub job: String,
    /// The latest attempt, when one exists.
    pub latest: Option<JobRun>,
}

/// The result of a manual run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedRun {
    /// The queued attempt.
    pub run: JobRun,
}

/// What a backfill queued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackfillResult {
    /// Windows accepted as new pending runs.
    pub accepted: usize,
    /// Windows already queued or covered.
    pub covered: usize,
}

/// Operator reads and triggers.
#[derive(Clone)]
pub struct OperatorService {
    runs: Arc<dyn RunRepository>,
    audit: Arc<dyn AuditClient>,
    clock: Arc<dyn Clock>,
    ids: Arc<dyn IdGenerator>,
}

impl OperatorService {
    /// Wires the ports.
    #[must_use]
    pub fn new(
        runs: Arc<dyn RunRepository>,
        audit: Arc<dyn AuditClient>,
        clock: Arc<dyn Clock>,
        ids: Arc<dyn IdGenerator>,
    ) -> Self {
        Self {
            runs,
            audit,
            clock,
            ids,
        }
    }

    /// The catalogue with each job's latest attempt.
    ///
    /// # Errors
    /// Returns [`AppError::Forbidden`] for a non-operator and
    /// [`AppError::Unavailable`] when the store cannot answer.
    pub async fn jobs(&self, caller: &Caller) -> Result<Vec<JobView>, AppError> {
        caller.require_operator()?;
        let latest = self
            .runs
            .latest_per_job()
            .await
            .map_err(AppError::from_repository)?;
        let mut views: Vec<JobView> = job_orchestration_domain::CATALOGUE
            .iter()
            .map(|(job, _)| JobView {
                job: (*job).to_string(),
                latest: latest.iter().find(|run| run.job == *job).cloned(),
            })
            .collect();
        views.sort_by(|left, right| left.job.cmp(&right.job));
        Ok(views)
    }

    /// One bounded run page, newest first.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for an unknown job or a bad page, and
    /// [`AppError::Forbidden`] for a non-operator.
    pub async fn runs(
        &self,
        caller: &Caller,
        job: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: Option<i64>,
    ) -> Result<Vec<JobRun>, AppError> {
        caller.require_operator()?;
        if !job_orchestration_domain::is_known_job(job) {
            return Err(AppError::NotFound("job"));
        }
        let limit = limit.unwrap_or(DEFAULT_PAGE);
        if limit <= 0 || limit > MAX_PAGE {
            return Err(AppError::Validation(format!(
                "limit must be between 1 and {MAX_PAGE}"
            )));
        }
        self.runs
            .list_for_job(job, from, to, limit)
            .await
            .map_err(AppError::from_repository)
    }

    /// Queues the current window as a manual run.
    ///
    /// # Errors
    /// Returns [`AppError::Conflict`] when the window is already queued or
    /// covered, and [`AppError::Validation`] for a missing reason.
    pub async fn run_now(
        &self,
        caller: &Caller,
        job: &str,
        reason: &str,
        schedule_text: &str,
    ) -> Result<QueuedRun, AppError> {
        caller.require_operator()?;
        let now = self.clock.now();
        let schedule = Schedule::parse(schedule_text)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let window = schedule.window(now);
        if let Some(existing) = self
            .runs
            .latest_for_window(job, window)
            .await
            .map_err(AppError::from_repository)?
        {
            return Err(AppError::Conflict(format!(
                "the window is already covered by run {}",
                existing.id
            )));
        }
        let run = JobRun::queued(
            RunId::new(self.ids.next()),
            job,
            window,
            1,
            Trigger::Manual,
            Some(caller.user_id),
            Some(reason),
        )
        .map_err(AppError::from_domain)?;
        self.runs
            .insert(&run)
            .await
            .map_err(AppError::from_repository)?;
        self.audit(caller, &run, "job.run.manual").await;
        Ok(QueuedRun { run })
    }

    /// Queues every uncovered window between `from` and `to`.
    ///
    /// # Errors
    /// Returns [`AppError::Validation`] for an inverted range or a width above
    /// seven days, and [`AppError::Forbidden`] for a non-operator.
    pub async fn backfill(
        &self,
        caller: &Caller,
        job: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        reason: &str,
        schedule_text: &str,
    ) -> Result<BackfillResult, AppError> {
        caller.require_operator()?;
        if to < from {
            return Err(AppError::Validation(
                "to must not be before from".to_string(),
            ));
        }
        if to - from > Duration::days(MAX_BACKFILL_DAYS) {
            return Err(AppError::Validation(
                "a backfill may span at most 7 days".to_string(),
            ));
        }
        let schedule = Schedule::parse(schedule_text)
            .map_err(|error| AppError::Internal(error.to_string()))?;
        let mut result = BackfillResult {
            accepted: 0,
            covered: 0,
        };
        let mut window = schedule.window(from);
        let mut walked = 0;
        while window <= to && walked < MAX_BACKFILL_WINDOWS {
            walked += 1;
            if self
                .runs
                .window_exists(job, window)
                .await
                .map_err(AppError::from_repository)?
            {
                result.covered += 1;
            } else {
                let run = JobRun::queued(
                    RunId::new(self.ids.next()),
                    job,
                    window,
                    1,
                    Trigger::Backfill,
                    Some(caller.user_id),
                    Some(reason),
                )
                .map_err(AppError::from_domain)?;
                self.runs
                    .insert(&run)
                    .await
                    .map_err(AppError::from_repository)?;
                self.audit(caller, &run, "job.backfill").await;
                result.accepted += 1;
            }
            window = schedule.next_window(window);
        }
        Ok(result)
    }

    async fn audit(&self, caller: &Caller, run: &JobRun, action: &str) {
        let event = AuditEvent {
            actor_id: Some(caller.user_id as UserId),
            action: action.to_string(),
            target: format!("job:{}:{}", run.job, run.scheduled_for.to_rfc3339()),
            outcome: "success".to_string(),
            occurred_at: self.clock.now(),
        };
        if let Err(error) = self.audit.append(&event).await {
            tracing::warn!(%error, "the operator event was not audited");
        }
    }
}
