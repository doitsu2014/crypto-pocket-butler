//! The account-deletion request and its execution state machine (WF9,
//! BR6.1–BR6.7).
//!
//! Completed destruction is never repeated: a retry resumes the remaining steps.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::error::DomainError;
pub use crate::ids::{DeletionRequestId, UserId};

use crate::export::Contributor;

/// The longest retention category or basis label.
pub const MAX_RETENTION_LABEL_LEN: usize = 120;

/// One category kept under a legal basis after deletion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedCategory {
    category: String,
    basis: String,
}

impl RetainedCategory {
    /// Creates a retained category.
    ///
    /// # Errors
    /// Returns [`DomainError`] when either label is blank or too long.
    pub fn new(category: impl Into<String>, basis: impl Into<String>) -> Result<Self, DomainError> {
        let category = category.into();
        let basis = basis.into();
        let category = category.trim();
        let basis = basis.trim();
        if category.is_empty() {
            return Err(DomainError::Required {
                field: "retainedCategories.category",
            });
        }
        if basis.is_empty() {
            return Err(DomainError::Required {
                field: "retainedCategories.basis",
            });
        }
        if category.len() > MAX_RETENTION_LABEL_LEN || basis.len() > MAX_RETENTION_LABEL_LEN {
            return Err(DomainError::TooLarge {
                field: "retainedCategories",
                limit: "120 characters per label",
            });
        }
        Ok(Self {
            category: category.to_string(),
            basis: basis.to_string(),
        })
    }

    /// The category label.
    #[must_use]
    pub fn category(&self) -> &str {
        &self.category
    }

    /// The legal basis label.
    #[must_use]
    pub fn basis(&self) -> &str {
        &self.basis
    }
}

/// The deletion lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DeletionStatus {
    /// The user confirmed; waiting for a worker.
    Requested,
    /// A worker holds the lease and is executing.
    InProgress,
    /// Every step finished; terminal.
    Completed,
    /// A step failed; retryable.
    Failed,
}

impl DeletionStatus {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "requested" => Ok(Self::Requested),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be requested, in_progress, completed, or failed".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Requested => "requested",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    /// Whether no further work happens.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        self == Self::Completed
    }
}

/// The per-step progress that makes a retry resume rather than repeat.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeletionProgress {
    anonymised: BTreeMap<Contributor, bool>,
    auth_deleted: bool,
    identity_anonymised: bool,
    key_destroyed: bool,
    retained: Vec<RetainedCategory>,
}

impl DeletionProgress {
    /// Records one contributor's anonymisation.
    pub fn record_anonymised(&mut self, contributor: Contributor) {
        self.anonymised.insert(contributor, true);
    }

    /// Accumulates the labelled retention categories a contributor returned, so
    /// a retry keeps them without repeating the anonymise call.
    pub fn record_retained(&mut self, categories: Vec<RetainedCategory>) {
        for category in categories {
            if !self.retained.iter().any(|existing| {
                existing.category() == category.category() && existing.basis() == category.basis()
            }) {
                self.retained.push(category);
            }
        }
    }

    /// The labelled retention categories recorded so far.
    #[must_use]
    pub fn retained(&self) -> &[RetainedCategory] {
        &self.retained
    }

    /// Records that the Supabase Auth identity was removed.
    pub fn record_auth_deleted(&mut self) {
        self.auth_deleted = true;
    }

    /// Records that Identity's own fields were anonymised.
    pub fn record_identity_anonymised(&mut self) {
        self.identity_anonymised = true;
    }

    /// Records that the shredding key was destroyed, once every earlier step is
    /// recorded.
    ///
    /// # Errors
    /// Returns [`DomainError::Conflict`] when an earlier step is incomplete.
    pub fn record_key_destroyed(&mut self) -> Result<(), DomainError> {
        if !self.all_anonymised() || !self.auth_deleted || !self.identity_anonymised {
            return Err(DomainError::Conflict {
                reason: "the shredding key can only be destroyed after every other step"
                    .to_string(),
            });
        }
        self.key_destroyed = true;
        Ok(())
    }

    /// Whether one contributor was anonymised.
    #[must_use]
    pub fn is_anonymised(&self, contributor: Contributor) -> bool {
        self.anonymised.get(&contributor).copied().unwrap_or(false)
    }

    /// Whether every contributor was anonymised.
    #[must_use]
    pub fn all_anonymised(&self) -> bool {
        Contributor::ALL
            .into_iter()
            .all(|contributor| self.is_anonymised(contributor))
    }

    /// Whether the Supabase Auth identity was removed.
    #[must_use]
    pub fn auth_deleted(&self) -> bool {
        self.auth_deleted
    }

    /// Whether Identity's own fields were anonymised.
    #[must_use]
    pub fn identity_anonymised(&self) -> bool {
        self.identity_anonymised
    }

    /// Whether the shredding key was destroyed.
    #[must_use]
    pub fn key_destroyed(&self) -> bool {
        self.key_destroyed
    }
}

/// One account-deletion request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeletionRequest {
    id: DeletionRequestId,
    user_id: UserId,
    status: DeletionStatus,
    progress: DeletionProgress,
    requested_at: DateTime<Utc>,
    retained_categories: Vec<RetainedCategory>,
    completed_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    lease_owner: Option<String>,
    lease_until: Option<DateTime<Utc>>,
}

impl DeletionRequest {
    /// Creates a requested deletion (WF9 step 2).
    #[must_use]
    pub fn request(id: DeletionRequestId, user_id: UserId, now: DateTime<Utc>) -> Self {
        Self {
            id,
            user_id,
            status: DeletionStatus::Requested,
            progress: DeletionProgress::default(),
            requested_at: now,
            retained_categories: Vec::new(),
            completed_at: None,
            failure_reason: None,
            lease_owner: None,
            lease_until: None,
        }
    }

    /// Claims the request for one worker, including a retry of a failed one.
    ///
    /// # Errors
    /// Returns [`DomainError::Conflict`] when another worker holds a live lease
    /// or the request is completed.
    pub fn claim(
        &mut self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<(), DomainError> {
        if !self.is_claimable(now) {
            return Err(DomainError::Conflict {
                reason: "the deletion is already claimed or is completed".to_string(),
            });
        }
        self.status = DeletionStatus::InProgress;
        self.lease_owner = Some(worker.to_string());
        self.lease_until = Some(now + lease);
        Ok(())
    }

    /// Whether a worker may claim the request at `now`.
    #[must_use]
    pub fn is_claimable(&self, now: DateTime<Utc>) -> bool {
        matches!(
            self.status,
            DeletionStatus::Requested | DeletionStatus::InProgress | DeletionStatus::Failed
        ) && self
            .lease_until
            .is_none_or(|lease_until| now >= lease_until)
    }

    /// Records that one contributing service anonymised its data.
    pub fn record_anonymised(&mut self, contributor: Contributor) {
        self.progress.anonymised.insert(contributor, true);
    }

    /// Records the retention categories a contributor keeps under a legal basis.
    pub fn record_retained(&mut self, categories: Vec<RetainedCategory>) {
        self.progress.record_retained(categories);
    }

    /// Records that the Supabase Auth identity was removed (BR6.7).
    pub fn record_auth_deleted(&mut self) {
        self.progress.auth_deleted = true;
    }

    /// Records that Identity's own fields and consent history were anonymised.
    pub fn record_identity_anonymised(&mut self, _now: DateTime<Utc>) {
        self.progress.identity_anonymised = true;
    }

    /// Records that the shredding key was destroyed. This is the last step and
    /// requires every preceding step (BR6.3).
    ///
    /// # Errors
    /// Returns [`DomainError::Conflict`] when an earlier step is incomplete.
    pub fn record_key_destroyed(&mut self, _now: DateTime<Utc>) -> Result<(), DomainError> {
        if !self.progress.all_anonymised()
            || !self.progress.auth_deleted
            || !self.progress.identity_anonymised
        {
            return Err(DomainError::Conflict {
                reason: "the shredding key can only be destroyed after every other step"
                    .to_string(),
            });
        }
        self.progress.key_destroyed = true;
        Ok(())
    }

    /// Completes the request (WF9 step 5).
    ///
    /// # Errors
    /// Returns [`DomainError::Conflict`] unless every step is recorded.
    pub fn complete(
        &mut self,
        retained_categories: Vec<RetainedCategory>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status == DeletionStatus::Completed {
            return Err(DomainError::InvalidTransition {
                entity: "deletion request",
                from: self.status.as_str(),
                to: "completed",
            });
        }
        if !self.progress.all_anonymised()
            || !self.progress.auth_deleted
            || !self.progress.identity_anonymised
            || !self.progress.key_destroyed
        {
            return Err(DomainError::Conflict {
                reason: "the deletion cannot complete with an unrecorded step".to_string(),
            });
        }
        self.status = DeletionStatus::Completed;
        self.retained_categories = retained_categories;
        self.completed_at = Some(now);
        self.failure_reason = None;
        Ok(())
    }

    /// Records a failed step and leaves the request retryable (BR6.5).
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] for a completed request.
    pub fn fail(
        &mut self,
        reason: impl Into<String>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        if self.status == DeletionStatus::Completed {
            return Err(DomainError::InvalidTransition {
                entity: "deletion request",
                from: self.status.as_str(),
                to: "failed",
            });
        }
        self.status = DeletionStatus::Failed;
        self.failure_reason = Some(reason.into());
        self.lease_until = Some(now);
        Ok(())
    }

    /// Rebuilds a request from stored columns.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: DeletionRequestId,
        user_id: UserId,
        status: DeletionStatus,
        progress: DeletionProgress,
        requested_at: DateTime<Utc>,
        retained_categories: Vec<RetainedCategory>,
        completed_at: Option<DateTime<Utc>>,
        failure_reason: Option<String>,
        lease_owner: Option<String>,
        lease_until: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            status,
            progress,
            requested_at,
            retained_categories,
            completed_at,
            failure_reason,
            lease_owner,
            lease_until,
        }
    }

    /// The request identifier.
    #[must_use]
    pub fn id(&self) -> DeletionRequestId {
        self.id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> DeletionStatus {
        self.status
    }

    /// The per-step progress.
    #[must_use]
    pub fn progress(&self) -> &DeletionProgress {
        &self.progress
    }

    /// When the deletion was requested.
    #[must_use]
    pub fn requested_at(&self) -> DateTime<Utc> {
        self.requested_at
    }

    /// The labelled retention categories.
    #[must_use]
    pub fn retained_categories(&self) -> &[RetainedCategory] {
        &self.retained_categories
    }

    /// When the deletion completed.
    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
    }

    /// Why the last attempt failed.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// The worker holding the lease.
    #[must_use]
    pub fn lease_owner(&self) -> Option<&str> {
        self.lease_owner.as_deref()
    }

    /// When the lease frees.
    #[must_use]
    pub fn lease_until(&self) -> Option<DateTime<Utc>> {
        self.lease_until
    }
}
