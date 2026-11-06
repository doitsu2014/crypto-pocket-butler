//! Retention runs and the audit-log sweep decision (BR4.1, BR4.2).
//!
//! The audit service sweeps its own `audit_logs` only. The two-year financial
//! window belongs to Ledger, which executes it in its own schema and reports the
//! run here.

use chrono::{DateTime, Months, TimeZone, Utc};

use crate::error::DomainError;
use crate::ids::RetentionRunId;
use crate::secrets::redact;

/// The retention window, in calendar months.
pub const AUDIT_LOG_RETENTION_MONTHS: u32 = 6;

/// The role an audit-log sweep must run under.
pub const AUDIT_RETENTION_ROLE: &str = "audit_retention";

/// The longest privileged-role name stored on a run.
pub const MAX_ROLE_LEN: usize = 64;

/// The longest failure reason stored on a run.
pub const MAX_FAILURE_REASON_LEN: usize = 500;

/// Whose run it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetentionCategory {
    /// A sweep the audit service ran in its own schema.
    AuditLogs,
    /// A sweep another service ran in its own schema and reported.
    Reported,
}

impl RetentionCategory {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "audit_logs" => Ok(Self::AuditLogs),
            "reported" => Ok(Self::Reported),
            _ => Err(DomainError::Invalid {
                field: "category",
                reason: "must be audit_logs or reported".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AuditLogs => "audit_logs",
            Self::Reported => "reported",
        }
    }
}

/// The service that executed the sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetentionOwner {
    /// Audit itself.
    Audit,
    /// Ledger.
    Ledger,
    /// Account.
    Account,
    /// Portfolio.
    Portfolio,
    /// Reporting.
    Reporting,
    /// Notification.
    Notification,
    /// Analytics.
    Analytics,
    /// Job Orchestration.
    JobOrchestration,
    /// Market Data.
    MarketData,
    /// Asset Registry.
    AssetRegistry,
    /// Identity.
    Identity,
}

impl RetentionOwner {
    /// The services allowed to own a run.
    pub const ALL: [RetentionOwner; 11] = [
        Self::Audit,
        Self::Ledger,
        Self::Account,
        Self::Portfolio,
        Self::Reporting,
        Self::Notification,
        Self::Analytics,
        Self::JobOrchestration,
        Self::MarketData,
        Self::AssetRegistry,
        Self::Identity,
    ];

    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "audit" => Ok(Self::Audit),
            "ledger" => Ok(Self::Ledger),
            "account" => Ok(Self::Account),
            "portfolio" => Ok(Self::Portfolio),
            "reporting" => Ok(Self::Reporting),
            "notification" => Ok(Self::Notification),
            "analytics" => Ok(Self::Analytics),
            "job_orchestration" => Ok(Self::JobOrchestration),
            "market_data" => Ok(Self::MarketData),
            "asset_registry" => Ok(Self::AssetRegistry),
            "identity" => Ok(Self::Identity),
            _ => Err(DomainError::Invalid {
                field: "owner",
                reason: "unknown retaining service".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Audit => "audit",
            Self::Ledger => "ledger",
            Self::Account => "account",
            Self::Portfolio => "portfolio",
            Self::Reporting => "reporting",
            Self::Notification => "notification",
            Self::Analytics => "analytics",
            Self::JobOrchestration => "job_orchestration",
            Self::MarketData => "market_data",
            Self::AssetRegistry => "asset_registry",
            Self::Identity => "identity",
        }
    }
}

/// Where a run is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetentionState {
    /// The sweep is running.
    Running,
    /// The sweep completed.
    Success,
    /// The sweep failed.
    Failed,
}

impl RetentionState {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "running" => Ok(Self::Running),
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "state",
                reason: "unknown retention state".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }
}

/// The terminal outcome of a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RetentionOutcome {
    /// The sweep completed.
    Success,
    /// The sweep failed.
    Failed,
}

impl RetentionOutcome {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "outcome",
                reason: "must be success or failed".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
        }
    }

    /// The state the outcome moves the run to.
    #[must_use]
    pub fn state(self) -> RetentionState {
        match self {
            Self::Success => RetentionState::Success,
            Self::Failed => RetentionState::Failed,
        }
    }
}

/// One retention run and what it deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionRun {
    id: RetentionRunId,
    category: RetentionCategory,
    owner: RetentionOwner,
    state: RetentionState,
    cutoff_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    deleted_count: i64,
    privileged_role: String,
    outcome: Option<RetentionOutcome>,
    failure_reason: Option<String>,
}

impl RetentionRun {
    /// Opens an audit-log sweep under the audited retention role.
    ///
    /// # Errors
    /// Returns [`DomainError`] when the role is blank or too long.
    pub fn start(
        id: RetentionRunId,
        cutoff_at: DateTime<Utc>,
        started_at: DateTime<Utc>,
        privileged_role: &str,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id,
            category: RetentionCategory::AuditLogs,
            owner: RetentionOwner::Audit,
            state: RetentionState::Running,
            cutoff_at,
            started_at,
            finished_at: None,
            deleted_count: 0,
            privileged_role: validate_role(privileged_role)?,
            outcome: None,
            failure_reason: None,
        })
    }

    /// Records a completed run another service reported (BR4.2).
    ///
    /// # Errors
    /// Returns [`DomainError`] when the report names no role, no outcome, or a
    /// category other than `reported`.
    #[allow(clippy::too_many_arguments)]
    pub fn reported(
        id: RetentionRunId,
        owner: RetentionOwner,
        cutoff_at: DateTime<Utc>,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        deleted_count: i64,
        privileged_role: &str,
        outcome: RetentionOutcome,
        failure_reason: Option<&str>,
    ) -> Result<Self, DomainError> {
        if owner == RetentionOwner::Audit {
            return Err(DomainError::Contradiction {
                field: "owner",
                reason: "a reported run belongs to another service".to_string(),
            });
        }
        if deleted_count < 0 {
            return Err(DomainError::Invalid {
                field: "deletedCount",
                reason: "must not be negative".to_string(),
            });
        }
        if finished_at < started_at {
            return Err(DomainError::Contradiction {
                field: "finishedAt",
                reason: "must not precede startedAt".to_string(),
            });
        }
        Ok(Self {
            id,
            category: RetentionCategory::Reported,
            owner,
            state: outcome.state(),
            cutoff_at,
            started_at,
            finished_at: Some(finished_at),
            deleted_count,
            privileged_role: validate_role(privileged_role)?,
            outcome: Some(outcome),
            failure_reason: failure_reason.map(sanitize_failure_reason),
        })
    }

    /// Finishes a running sweep successfully.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the run is not running,
    /// or [`DomainError::Invalid`] for a negative count.
    pub fn finish_success(
        mut self,
        deleted_count: i64,
        finished_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        self.ensure_running("success")?;
        if deleted_count < 0 {
            return Err(DomainError::Invalid {
                field: "deletedCount",
                reason: "must not be negative".to_string(),
            });
        }
        self.state = RetentionState::Success;
        self.outcome = Some(RetentionOutcome::Success);
        self.deleted_count = deleted_count;
        self.finished_at = Some(finished_at);
        Ok(self)
    }

    /// Finishes a running sweep as failed. The reason is redacted, never refused,
    /// because the run still has to record that it failed.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the run is not running.
    pub fn finish_failed(
        mut self,
        reason: &str,
        finished_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        self.ensure_running("failed")?;
        self.state = RetentionState::Failed;
        self.outcome = Some(RetentionOutcome::Failed);
        self.failure_reason = Some(sanitize_failure_reason(reason));
        self.finished_at = Some(finished_at);
        Ok(self)
    }

    fn ensure_running(&self, to: &'static str) -> Result<(), DomainError> {
        if self.state == RetentionState::Running {
            Ok(())
        } else {
            Err(DomainError::InvalidTransition {
                entity: "retention run",
                from: self.state.as_str(),
                to,
            })
        }
    }

    /// The run identifier.
    #[must_use]
    pub fn id(&self) -> RetentionRunId {
        self.id
    }

    /// Whose run it is.
    #[must_use]
    pub fn category(&self) -> RetentionCategory {
        self.category
    }

    /// The service that executed it.
    #[must_use]
    pub fn owner(&self) -> RetentionOwner {
        self.owner
    }

    /// Where it is in its lifecycle.
    #[must_use]
    pub fn state(&self) -> RetentionState {
        self.state
    }

    /// Records strictly older than this were in scope.
    #[must_use]
    pub fn cutoff_at(&self) -> DateTime<Utc> {
        self.cutoff_at
    }

    /// When the sweep started.
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// When the sweep finished.
    #[must_use]
    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }

    /// How many records were deleted.
    #[must_use]
    pub fn deleted_count(&self) -> i64 {
        self.deleted_count
    }

    /// The audited role the sweep ran under.
    #[must_use]
    pub fn privileged_role(&self) -> &str {
        &self.privileged_role
    }

    /// The terminal outcome.
    #[must_use]
    pub fn outcome(&self) -> Option<RetentionOutcome> {
        self.outcome
    }

    /// Why the run failed.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// Rebuilds a run from storage.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: RetentionRunId,
        category: RetentionCategory,
        owner: RetentionOwner,
        state: RetentionState,
        cutoff_at: DateTime<Utc>,
        started_at: DateTime<Utc>,
        finished_at: Option<DateTime<Utc>>,
        deleted_count: i64,
        privileged_role: String,
        outcome: Option<RetentionOutcome>,
        failure_reason: Option<String>,
    ) -> Self {
        Self {
            id,
            category,
            owner,
            state,
            cutoff_at,
            started_at,
            finished_at,
            deleted_count,
            privileged_role,
            outcome,
            failure_reason,
        }
    }
}

/// The cutoff for a run: the same day six calendar months earlier, clamped to the
/// last day of the month.
#[must_use]
pub fn cutoff_six_calendar_months_before(now: DateTime<Utc>) -> DateTime<Utc> {
    let date = now.date_naive();
    let shifted = date
        .checked_sub_months(Months::new(AUDIT_LOG_RETENTION_MONTHS))
        .unwrap_or(date);
    Utc.from_utc_datetime(&shifted.and_time(now.time()))
}

/// One partition of the audit trail, as the sweep sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionSummary {
    /// The partition table name.
    pub name: String,
    /// The first instant the partition holds.
    pub from: DateTime<Utc>,
    /// The exclusive end of the partition range.
    pub to: DateTime<Utc>,
    /// Whether any row in the partition carries a legal basis.
    pub holds_legal_hold: bool,
    /// How many rows the partition holds.
    pub row_count: i64,
}

/// What the sweep does with one partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionAction {
    /// The whole partition is past the window and holds nothing legally held.
    Drop,
    /// Part of the partition is past the window, or it holds a legally held row.
    BatchDelete,
    /// Nothing in the partition is past the window.
    Keep,
}

/// BR4.1: the drop-versus-batch rule.
///
/// A partition entirely older than the cutoff and free of legal holds is dropped
/// whole. A partition straddling the cutoff, or holding a legally held record, is
/// swept row by row so the held rows survive.
#[must_use]
pub fn partition_action(summary: &PartitionSummary, cutoff: DateTime<Utc>) -> PartitionAction {
    if summary.to <= cutoff {
        if summary.holds_legal_hold {
            PartitionAction::BatchDelete
        } else {
            PartitionAction::Drop
        }
    } else if summary.from < cutoff {
        PartitionAction::BatchDelete
    } else {
        PartitionAction::Keep
    }
}

fn validate_role(raw: &str) -> Result<String, DomainError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(DomainError::Required {
            field: "privilegedRole",
        });
    }
    if value.len() > MAX_ROLE_LEN {
        return Err(DomainError::TooLarge {
            field: "privilegedRole",
            limit: "64 characters",
        });
    }
    Ok(value.to_string())
}

fn sanitize_failure_reason(reason: &str) -> String {
    let trimmed = reason.trim();
    if trimmed.is_empty() {
        return "unspecified failure".to_string();
    }
    let (safe, _) = redact(trimmed);
    safe.chars().take(MAX_FAILURE_REASON_LEN).collect()
}
