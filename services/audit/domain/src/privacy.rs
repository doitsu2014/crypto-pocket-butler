//! Privacy deletion (BR4.3, WF7). The audit service records the request; the
//! retention service — the only holder of a `DELETE` grant — executes it.

use chrono::{DateTime, Utc};

use crate::error::DomainError;
use crate::ids::{AuditEventId, DeletionRequestId, PrivacyDeletionId, UserId};
use crate::secrets::redact;

/// The longest failure reason stored on a deletion.
pub const MAX_FAILURE_REASON_LEN: usize = 500;

/// Where a deletion request is in its lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PrivacyDeletionState {
    /// Recorded; the retention service has not executed it.
    Requested,
    /// Executed.
    Completed,
    /// Execution failed.
    Failed,
}

impl PrivacyDeletionState {
    /// Parses the stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "requested" => Ok(Self::Requested),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "state",
                reason: "unknown privacy-deletion state".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }
}

/// One event the deletion examined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventLabel {
    /// The event identifier.
    pub id: AuditEventId,
    /// The legal basis stamped at append time, if any.
    pub legal_basis: Option<String>,
}

/// What a deletion did with the records it examined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DeletionReport {
    /// Records removed.
    pub deleted: i64,
    /// Records kept under a stated legal basis.
    pub kept_with_legal_basis: i64,
    /// Kept records carrying no legal basis — a defect, reported not repaired.
    pub defects: i64,
}

/// BR4.3: decides per record. A record with a legal basis is kept; everything
/// else is deleted. A kept record without a basis is a defect the report carries.
#[must_use]
pub fn classify(labels: &[EventLabel]) -> DeletionReport {
    let mut report = DeletionReport::default();
    for label in labels {
        match label.legal_basis.as_deref() {
            None => report.deleted += 1,
            Some(basis) if basis.trim().is_empty() => {
                report.kept_with_legal_basis += 1;
                report.defects += 1;
            }
            Some(_) => report.kept_with_legal_basis += 1,
        }
    }
    report
}

/// One recorded privacy-deletion request and its result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivacyDeletion {
    id: PrivacyDeletionId,
    deletion_request_id: DeletionRequestId,
    subject_user_id: UserId,
    requested_at: DateTime<Utc>,
    state: PrivacyDeletionState,
    completed_at: Option<DateTime<Utc>>,
    deleted_count: i64,
    kept_count: i64,
    kept_with_legal_basis: i64,
    defects: i64,
    failure_reason: Option<String>,
}

impl PrivacyDeletion {
    /// Records a request in the `requested` state.
    #[must_use]
    pub fn requested(
        id: PrivacyDeletionId,
        deletion_request_id: DeletionRequestId,
        subject_user_id: UserId,
        at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            deletion_request_id,
            subject_user_id,
            requested_at: at,
            state: PrivacyDeletionState::Requested,
            completed_at: None,
            deleted_count: 0,
            kept_count: 0,
            kept_with_legal_basis: 0,
            defects: 0,
            failure_reason: None,
        }
    }

    /// Records the result of an executed deletion.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the request is not
    /// requested, or [`DomainError::Invalid`] for negative counts.
    pub fn complete(
        mut self,
        report: DeletionReport,
        at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        self.ensure_requested("completed")?;
        if report.deleted < 0 || report.kept_with_legal_basis < 0 || report.defects < 0 {
            return Err(DomainError::Invalid {
                field: "counts",
                reason: "counts must not be negative".to_string(),
            });
        }
        self.state = PrivacyDeletionState::Completed;
        self.completed_at = Some(at);
        self.deleted_count = report.deleted;
        self.kept_with_legal_basis = report.kept_with_legal_basis;
        self.kept_count = report.kept_with_legal_basis;
        self.defects = report.defects;
        Ok(self)
    }

    /// Records a failed deletion. The reason is redacted, not refused.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] when the request is not
    /// requested.
    pub fn fail(mut self, reason: &str, at: DateTime<Utc>) -> Result<Self, DomainError> {
        self.ensure_requested("failed")?;
        self.state = PrivacyDeletionState::Failed;
        self.completed_at = Some(at);
        let trimmed = reason.trim();
        let (safe, _) = redact(if trimmed.is_empty() {
            "unspecified failure"
        } else {
            trimmed
        });
        self.failure_reason = Some(safe.chars().take(MAX_FAILURE_REASON_LEN).collect());
        Ok(self)
    }

    fn ensure_requested(&self, to: &'static str) -> Result<(), DomainError> {
        if self.state == PrivacyDeletionState::Requested {
            Ok(())
        } else {
            Err(DomainError::InvalidTransition {
                entity: "privacy deletion",
                from: self.state.as_str(),
                to,
            })
        }
    }

    /// The deletion row identifier.
    #[must_use]
    pub fn id(&self) -> PrivacyDeletionId {
        self.id
    }

    /// Identity's deletion request identifier.
    #[must_use]
    pub fn deletion_request_id(&self) -> DeletionRequestId {
        self.deletion_request_id
    }

    /// The user being deleted.
    #[must_use]
    pub fn subject_user_id(&self) -> UserId {
        self.subject_user_id
    }

    /// When the request was recorded.
    #[must_use]
    pub fn requested_at(&self) -> DateTime<Utc> {
        self.requested_at
    }

    /// Where the request is in its lifecycle.
    #[must_use]
    pub fn state(&self) -> PrivacyDeletionState {
        self.state
    }

    /// When the request reached a terminal state.
    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// Records removed.
    #[must_use]
    pub fn deleted_count(&self) -> i64 {
        self.deleted_count
    }

    /// Records kept.
    #[must_use]
    pub fn kept_count(&self) -> i64 {
        self.kept_count
    }

    /// Kept records carrying a legal basis.
    #[must_use]
    pub fn kept_with_legal_basis(&self) -> i64 {
        self.kept_with_legal_basis
    }

    /// Kept records carrying no legal basis.
    #[must_use]
    pub fn defects(&self) -> i64 {
        self.defects
    }

    /// Why the deletion failed.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// Rebuilds a deletion from storage.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: PrivacyDeletionId,
        deletion_request_id: DeletionRequestId,
        subject_user_id: UserId,
        requested_at: DateTime<Utc>,
        state: PrivacyDeletionState,
        completed_at: Option<DateTime<Utc>>,
        deleted_count: i64,
        kept_count: i64,
        kept_with_legal_basis: i64,
        defects: i64,
        failure_reason: Option<String>,
    ) -> Self {
        Self {
            id,
            deletion_request_id,
            subject_user_id,
            requested_at,
            state,
            completed_at,
            deleted_count,
            kept_count,
            kept_with_legal_basis,
            defects,
            failure_reason,
        }
    }
}
