//! The data-export request and its assembly state machine (WF8, BR5.1–BR5.4).
//!
//! Progress is monotonic; a partial archive is never presented as complete.

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};

use crate::error::DomainError;
pub use crate::ids::{ExportRequestId, UserId};

/// How long the worker may spend assembling an export.
pub const DEFAULT_ASSEMBLY_MINUTES: i64 = 15;
/// How long a signed download stays valid.
pub const DOWNLOAD_WINDOW_HOURS: i64 = 24;
/// How long a worker holds a claimed record.
pub const LEASE_SECONDS: i64 = 60;

/// One service that contributes data to the export and receives an anonymise
/// call on deletion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Contributor {
    /// The Account service.
    Account,
    /// The Ledger service.
    Ledger,
    /// The Portfolio service.
    Portfolio,
}

impl Contributor {
    /// Every contributor, in the fixed call order.
    pub const ALL: [Self; 3] = [Self::Account, Self::Ledger, Self::Portfolio];

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Account => "account",
            Self::Ledger => "ledger",
            Self::Portfolio => "portfolio",
        }
    }

    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "account" => Ok(Self::Account),
            "ledger" => Ok(Self::Ledger),
            "portfolio" => Ok(Self::Portfolio),
            _ => Err(DomainError::Invalid {
                field: "contributor",
                reason: "must be account, ledger, or portfolio".to_string(),
            }),
        }
    }
}

/// The state of one contribution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContributionState {
    /// Not answered yet.
    Pending,
    /// Answered successfully.
    Succeeded,
    /// Answered with a failure.
    Failed,
}

impl ContributionState {
    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "pending" => Ok(Self::Pending),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "contribution",
                reason: "must be pending, succeeded, or failed".to_string(),
            }),
        }
    }
}

/// The contribution states keyed by contributor.
pub type Contributions = BTreeMap<Contributor, ContributionState>;

/// A fresh map with every contributor pending.
#[must_use]
pub fn pending_contributions() -> Contributions {
    Contributor::ALL
        .into_iter()
        .map(|contributor| (contributor, ContributionState::Pending))
        .collect()
}

/// The export lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExportStatus {
    /// Waiting for a worker.
    Pending,
    /// A worker holds the lease and is assembling.
    Building,
    /// The archive exists and can be downloaded.
    Ready,
    /// A contribution failed; terminal for this request.
    Failed,
    /// The assembly deadline or download window passed; terminal.
    Expired,
}

impl ExportStatus {
    /// Parses a stored value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for anything else.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim() {
            "pending" => Ok(Self::Pending),
            "building" => Ok(Self::Building),
            "ready" => Ok(Self::Ready),
            "failed" => Ok(Self::Failed),
            "expired" => Ok(Self::Expired),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be pending, building, ready, failed, or expired".to_string(),
            }),
        }
    }

    /// The stored value.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Building => "building",
            Self::Ready => "ready",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    /// Whether no further work happens for this request.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Failed | Self::Expired)
    }
}

/// One export request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportRequest {
    id: ExportRequestId,
    user_id: UserId,
    status: ExportStatus,
    progress: u8,
    contributions: Contributions,
    requested_at: DateTime<Utc>,
    assembly_deadline: DateTime<Utc>,
    file_reference: Option<String>,
    download_expires_at: Option<DateTime<Utc>>,
    failure_reason: Option<String>,
    completed_at: Option<DateTime<Utc>>,
    lease_owner: Option<String>,
    lease_until: Option<DateTime<Utc>>,
}

impl ExportRequest {
    /// Creates a pending request (WF8 step 2).
    #[must_use]
    pub fn request(
        id: ExportRequestId,
        user_id: UserId,
        now: DateTime<Utc>,
        assembly_deadline: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_id,
            status: ExportStatus::Pending,
            progress: 0,
            contributions: pending_contributions(),
            requested_at: now,
            assembly_deadline,
            file_reference: None,
            download_expires_at: None,
            failure_reason: None,
            completed_at: None,
            lease_owner: None,
            lease_until: None,
        }
    }

    /// Claims the request for one worker (WF8 step 3).
    ///
    /// # Errors
    /// Returns [`DomainError::Conflict`] when another worker holds a live lease
    /// or the request is terminal.
    pub fn claim(
        &mut self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<(), DomainError> {
        if !self.is_claimable(now) {
            return Err(DomainError::Conflict {
                reason: "the export is already claimed or is not claimable".to_string(),
            });
        }
        self.status = ExportStatus::Building;
        self.lease_owner = Some(worker.to_string());
        self.lease_until = Some(now + lease);
        Ok(())
    }

    /// Whether a worker may claim the request at `now`.
    #[must_use]
    pub fn is_claimable(&self, now: DateTime<Utc>) -> bool {
        matches!(self.status, ExportStatus::Pending | ExportStatus::Building)
            && self
                .lease_until
                .is_none_or(|lease_until| now >= lease_until)
    }

    /// Records one successful contribution and advances progress.
    pub fn record_success(&mut self, contributor: Contributor, _now: DateTime<Utc>) {
        if self.status != ExportStatus::Building {
            return;
        }
        self.contributions
            .insert(contributor, ContributionState::Succeeded);
        let answered = self
            .contributions
            .values()
            .filter(|state| **state == ContributionState::Succeeded)
            .count();
        let computed = (answered * 100 / Contributor::ALL.len()) as u8;
        self.progress = self.progress.max(computed);
    }

    /// Records one failed contribution and fails the whole request.
    pub fn record_failure(
        &mut self,
        contributor: Contributor,
        reason: impl Into<String>,
        _now: DateTime<Utc>,
    ) {
        if self.status != ExportStatus::Building {
            return;
        }
        self.contributions
            .insert(contributor, ContributionState::Failed);
        self.failure_reason = Some(reason.into());
        self.status = ExportStatus::Failed;
    }

    /// Fails the request without naming a contributor, used when the assembled
    /// archive itself is refused (a credential-shaped field or a storage error).
    pub fn fail(&mut self, reason: impl Into<String>, _now: DateTime<Utc>) {
        if self.status != ExportStatus::Building {
            return;
        }
        self.failure_reason = Some(reason.into());
        self.status = ExportStatus::Failed;
    }

    /// Marks the assembled archive ready (WF8 step 4).
    ///
    /// # Errors
    /// Returns [`DomainError::Conflict`] unless every contributor succeeded and
    /// the request is building.
    pub fn mark_ready(
        &mut self,
        file_reference: impl Into<String>,
        download_expires_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), DomainError> {
        let all_succeeded = self
            .contributions
            .values()
            .all(|state| *state == ContributionState::Succeeded);
        if self.status != ExportStatus::Building || !all_succeeded {
            return Err(DomainError::Conflict {
                reason: "the export is incomplete and cannot be marked ready".to_string(),
            });
        }
        self.status = ExportStatus::Ready;
        self.progress = 100;
        self.file_reference = Some(file_reference.into());
        self.download_expires_at = Some(download_expires_at);
        self.completed_at = Some(now);
        Ok(())
    }

    /// Expires the request when its assembly deadline or download window passed.
    /// Returns whether the state changed.
    pub fn expire_if_due(&mut self, now: DateTime<Utc>) -> bool {
        let due = match self.status {
            ExportStatus::Pending | ExportStatus::Building => now >= self.assembly_deadline,
            ExportStatus::Ready => self
                .download_expires_at
                .is_some_and(|download_expires_at| now >= download_expires_at),
            ExportStatus::Failed | ExportStatus::Expired => false,
        };
        if due {
            self.status = ExportStatus::Expired;
        }
        due
    }

    /// Rebuilds a request from stored columns.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: ExportRequestId,
        user_id: UserId,
        status: ExportStatus,
        progress: u8,
        contributions: Contributions,
        requested_at: DateTime<Utc>,
        assembly_deadline: DateTime<Utc>,
        file_reference: Option<String>,
        download_expires_at: Option<DateTime<Utc>>,
        failure_reason: Option<String>,
        completed_at: Option<DateTime<Utc>>,
        lease_owner: Option<String>,
        lease_until: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id,
            user_id,
            status,
            progress,
            contributions,
            requested_at,
            assembly_deadline,
            file_reference,
            download_expires_at,
            failure_reason,
            completed_at,
            lease_owner,
            lease_until,
        }
    }

    /// The request identifier.
    #[must_use]
    pub fn id(&self) -> ExportRequestId {
        self.id
    }

    /// The owner.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> ExportStatus {
        self.status
    }

    /// Percentage of contributors that answered.
    #[must_use]
    pub fn progress(&self) -> u8 {
        self.progress
    }

    /// The contribution states.
    #[must_use]
    pub fn contributions(&self) -> &Contributions {
        &self.contributions
    }

    /// When the request arrived.
    #[must_use]
    pub fn requested_at(&self) -> DateTime<Utc> {
        self.requested_at
    }

    /// When an unfinished request expires.
    #[must_use]
    pub fn assembly_deadline(&self) -> DateTime<Utc> {
        self.assembly_deadline
    }

    /// The archive reference in the private bucket.
    #[must_use]
    pub fn file_reference(&self) -> Option<&str> {
        self.file_reference.as_deref()
    }

    /// When the signed download stops working.
    #[must_use]
    pub fn download_expires_at(&self) -> Option<DateTime<Utc>> {
        self.download_expires_at
    }

    /// Why the request failed.
    #[must_use]
    pub fn failure_reason(&self) -> Option<&str> {
        self.failure_reason.as_deref()
    }

    /// When the archive became ready.
    #[must_use]
    pub fn completed_at(&self) -> Option<DateTime<Utc>> {
        self.completed_at
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

    /// Whether the request is ready.
    #[must_use]
    pub fn is_ready(&self) -> bool {
        self.status == ExportStatus::Ready
    }
}
