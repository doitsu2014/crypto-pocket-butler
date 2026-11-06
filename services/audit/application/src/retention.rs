//! WF6: run and record retention (BR4.1, BR4.2).

use std::sync::Arc;

use audit_domain::{
    cutoff_six_calendar_months_before, partition_action, Context, Outcome, PartitionAction,
    RetentionCategory, RetentionOutcome, RetentionOwner, RetentionRun, RetentionRunId,
    RetentionState, Sensitivity, AUDIT_RETENTION_ROLE,
};

use crate::common::FieldError;
use crate::ports::{Clock, Pause, RetentionRepository};
use crate::trail::Trail;

/// The default batch size for a partial-partition sweep.
pub const DEFAULT_BATCH_SIZE: u32 = 10_000;
/// The default maximum batches per sweep.
pub const DEFAULT_MAX_BATCHES: u32 = 1_000;

/// Sweep sizing, held in one place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SweepConfig {
    /// Rows deleted per batch.
    pub batch_size: u32,
    /// The maximum batches one sweep may run.
    pub max_batches: u32,
}

impl Default for SweepConfig {
    fn default() -> Self {
        Self {
            batch_size: DEFAULT_BATCH_SIZE,
            max_batches: DEFAULT_MAX_BATCHES,
        }
    }
}

/// A report another service sends about its own completed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCommand {
    /// Must be `reported`.
    pub category: String,
    /// The service that ran the sweep.
    pub owner: String,
    /// The cutoff it used.
    pub cutoff_at: chrono::DateTime<chrono::Utc>,
    /// When its sweep started.
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// When its sweep finished.
    pub finished_at: chrono::DateTime<chrono::Utc>,
    /// How many records it deleted.
    pub deleted_count: i64,
    /// The audited role it ran under.
    pub privileged_role: String,
    /// `success` or `failed`.
    pub outcome: String,
    /// Why it failed, when it did.
    pub failure_reason: Option<String>,
}

/// Why a retention operation was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RetentionError {
    /// The report failed validation.
    #[error("the retention report is invalid")]
    Rejected(Vec<FieldError>),
    /// The report named a service other than the caller.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// The run could not open under the privileged role.
    #[error("the run could not start: {0}")]
    CouldNotStart(String),
    /// The sweep failed after it started.
    #[error("the sweep failed: {0}")]
    RunFailed(String),
    /// Storage failed.
    #[error("the run could not be stored: {0}")]
    Repository(String),
}

/// Runs the audit-log sweep and records reported runs.
pub struct RetentionService {
    retention: Arc<dyn RetentionRepository>,
    clock: Arc<dyn Clock>,
    pause: Arc<dyn Pause>,
    trail: Trail,
    config: SweepConfig,
}

impl RetentionService {
    /// Creates the service.
    #[must_use]
    pub fn new(
        retention: Arc<dyn RetentionRepository>,
        clock: Arc<dyn Clock>,
        pause: Arc<dyn Pause>,
        trail: Trail,
        config: SweepConfig,
    ) -> Self {
        Self {
            retention,
            clock,
            pause,
            trail,
            config,
        }
    }

    /// Creates the current month's partition and the next `months_ahead`.
    ///
    /// # Errors
    /// Returns [`RetentionError::Repository`] when the partitions cannot be made.
    pub async fn ensure_partitions(
        &self,
        months_ahead: u32,
    ) -> Result<Vec<String>, RetentionError> {
        self.retention
            .ensure_partitions_ahead(months_ahead)
            .await
            .map_err(|error| RetentionError::Repository(error.to_string()))
    }

    /// Runs one audit-log sweep under the audited retention role.
    ///
    /// # Errors
    /// Returns [`RetentionError::CouldNotStart`] when the run cannot be opened,
    /// and [`RetentionError::RunFailed`] when the sweep fails after starting. A
    /// failed run is still recorded.
    pub async fn sweep(&self) -> Result<RetentionRun, RetentionError> {
        let now = self.clock.now();
        let cutoff = cutoff_six_calendar_months_before(now);
        let run = RetentionRun::start(
            RetentionRunId::generate(),
            cutoff,
            now,
            AUDIT_RETENTION_ROLE,
        )
        .map_err(|error| RetentionError::Rejected(vec![FieldError::from(error)]))?;

        if let Err(error) = self.retention.insert_run(&run).await {
            let _ = self
                .trail
                .record_system(
                    "system.retention.run",
                    "audit_logs",
                    Outcome::Failed,
                    Sensitivity::Security,
                    rule_context("run-not-started"),
                )
                .await;
            return Err(RetentionError::CouldNotStart(error.to_string()));
        }

        let mut deleted: i64 = 0;
        match self.execute_partitions(cutoff, &mut deleted).await {
            Ok(()) => {
                let finished = run
                    .finish_success(deleted, self.clock.now())
                    .map_err(|error| RetentionError::Repository(error.to_string()))?;
                self.retention
                    .update_run(&finished)
                    .await
                    .map_err(|error| RetentionError::Repository(error.to_string()))?;
                let _ = self
                    .trail
                    .record_system(
                        "system.retention.run",
                        &finished.id().to_string(),
                        Outcome::Success,
                        Sensitivity::Routine,
                        run_context(finished.id()),
                    )
                    .await;
                Ok(finished)
            }
            Err(reason) => {
                let finished = run
                    .finish_failed(&reason, self.clock.now())
                    .map_err(|error| RetentionError::Repository(error.to_string()))?;
                let _ = self.retention.update_run(&finished).await;
                let _ = self
                    .trail
                    .record_system(
                        "system.retention.run",
                        &finished.id().to_string(),
                        Outcome::Failed,
                        Sensitivity::Security,
                        run_context(finished.id()),
                    )
                    .await;
                Err(RetentionError::RunFailed(reason))
            }
        }
    }

    async fn execute_partitions(
        &self,
        cutoff: chrono::DateTime<chrono::Utc>,
        deleted: &mut i64,
    ) -> Result<(), String> {
        let summaries = self
            .retention
            .partition_summaries()
            .await
            .map_err(|error| error.to_string())?;
        let mut needs_batch = false;
        for summary in summaries {
            match partition_action(&summary, cutoff) {
                PartitionAction::Drop => {
                    self.retention
                        .drop_partition(&summary.name)
                        .await
                        .map_err(|error| error.to_string())?;
                    *deleted += summary.row_count;
                }
                PartitionAction::BatchDelete => needs_batch = true,
                PartitionAction::Keep => {}
            }
        }

        if needs_batch {
            for _ in 0..self.config.max_batches {
                let removed = self
                    .retention
                    .delete_batch(cutoff, self.config.batch_size)
                    .await
                    .map_err(|error| error.to_string())?;
                *deleted += removed as i64;
                if removed < u64::from(self.config.batch_size) {
                    break;
                }
                self.pause.pause().await;
            }
        }
        Ok(())
    }

    /// Records a completed run another service reports (BR4.2).
    ///
    /// # Errors
    /// Returns [`RetentionError::Rejected`] when the report names no role or
    /// outcome, [`RetentionError::Forbidden`] when it names another service, and
    /// [`RetentionError::Repository`] on a storage failure.
    pub async fn report(
        &self,
        caller_service: &str,
        command: ReportCommand,
    ) -> Result<RetentionRun, RetentionError> {
        let (owner, outcome) = match self.validate_report(caller_service, &command) {
            Ok(values) => values,
            Err(error) => {
                let _ = self
                    .trail
                    .record_system(
                        "system.retention.reported",
                        caller_service,
                        Outcome::Denied,
                        Sensitivity::Security,
                        rule_context("report-invalid"),
                    )
                    .await;
                return Err(error);
            }
        };

        let run = RetentionRun::reported(
            RetentionRunId::generate(),
            owner,
            command.cutoff_at,
            command.started_at,
            command.finished_at,
            command.deleted_count,
            &command.privileged_role,
            outcome,
            command.failure_reason.as_deref(),
        )
        .map_err(|error| RetentionError::Rejected(vec![FieldError::from(error)]))?;

        self.retention
            .insert_run(&run)
            .await
            .map_err(|error| RetentionError::Repository(error.to_string()))?;
        let _ = self
            .trail
            .record_system(
                "system.retention.reported",
                &run.id().to_string(),
                Outcome::Success,
                Sensitivity::Routine,
                run_context(run.id()),
            )
            .await;
        Ok(run)
    }

    fn validate_report(
        &self,
        caller_service: &str,
        command: &ReportCommand,
    ) -> Result<(RetentionOwner, RetentionOutcome), RetentionError> {
        let category = RetentionCategory::parse(&command.category)
            .map_err(|error| RetentionError::Rejected(vec![FieldError::from(error)]))?;
        if category != RetentionCategory::Reported {
            return Err(RetentionError::Rejected(vec![FieldError::new(
                "category",
                "Audit runs its own audit_logs sweep; reports must be category reported",
            )]));
        }
        let owner = RetentionOwner::parse(&command.owner)
            .map_err(|error| RetentionError::Rejected(vec![FieldError::from(error)]))?;
        let normalised = caller_service.replace('-', "_");
        if owner.as_str() != normalised {
            return Err(RetentionError::Forbidden(format!(
                "the caller may report only its own run, not '{}'",
                owner.as_str()
            )));
        }
        let outcome = RetentionOutcome::parse(&command.outcome)
            .map_err(|error| RetentionError::Rejected(vec![FieldError::from(error)]))?;
        if command.privileged_role.trim().is_empty() {
            return Err(RetentionError::Rejected(vec![FieldError::new(
                "privilegedRole",
                "a run without a privileged role is refused",
            )]));
        }
        Ok((owner, outcome))
    }

    /// A run is never resumed after a failure: this is what the next scheduled
    /// run does instead.
    #[must_use]
    pub fn state_of(run: &RetentionRun) -> RetentionState {
        run.state()
    }
}

fn rule_context(rule: &str) -> Context {
    let mut context = Context::empty();
    let _ = context.insert("ruleId", rule);
    context
}

fn run_context(run_id: RetentionRunId) -> Context {
    let mut context = Context::empty();
    let _ = context.insert("runId", &run_id.to_string());
    context
}
