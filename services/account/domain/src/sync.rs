//! Sync runs and the single-flight lease (BR4.1–BR4.5, NFR reliability).

use chrono::{DateTime, Duration, Utc};

use crate::error::DomainError;
use crate::ids::{AccountId, SyncRunId};
use crate::secrets::looks_like_secret;

/// A run holds ownership of an account for this many seconds.
pub const LEASE_SECONDS: i64 = 60;
/// The worker renews the lease at this cadence.
pub const RENEW_INTERVAL_SECONDS: i64 = 20;
/// The sweep marks expired runs failed at this cadence.
pub const SWEEP_INTERVAL_SECONDS: i64 = 60;
/// The longest non-secret failure summary a run stores.
pub const MAX_ERROR_MESSAGE_CHARS: usize = 500;

/// The reason class recorded on a failed run (never a provider body).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// The credential or RPC authenticated incorrectly.
    Auth,
    /// The venue returned a rate-limit response.
    RateLimit,
    /// The provider answered with an error.
    Provider,
    /// The provider or network was unreachable.
    Network,
    /// The provider returned data that could not be used.
    Data,
}

impl ErrorCategory {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auth => "auth",
            Self::RateLimit => "rate_limit",
            Self::Provider => "provider",
            Self::Network => "network",
            Self::Data => "data",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown category.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "auth" => Ok(Self::Auth),
            "rate_limit" => Ok(Self::RateLimit),
            "provider" => Ok(Self::Provider),
            "network" => Ok(Self::Network),
            "data" => Ok(Self::Data),
            _ => Err(DomainError::Invalid {
                field: "errorCategory",
                reason: "must be auth, rate_limit, provider, network, or data".to_string(),
            }),
        }
    }
}

/// The run lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncRunStatus {
    /// The run owns the lease.
    Running,
    /// The run finished with a snapshot.
    Succeeded,
    /// The run failed or its lease expired.
    Failed,
}

impl SyncRunStatus {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown status.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(DomainError::Invalid {
                field: "status",
                reason: "must be running, succeeded, or failed".to_string(),
            }),
        }
    }
}

/// One sync attempt for one account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRun {
    id: SyncRunId,
    account_id: AccountId,
    started_at: DateTime<Utc>,
    lease_until: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    status: SyncRunStatus,
    error_category: Option<ErrorCategory>,
    error_message: Option<String>,
    retries: u32,
}

impl SyncRun {
    /// Claims a new running run with a live lease.
    #[must_use]
    pub fn claim(
        id: SyncRunId,
        account_id: AccountId,
        now: DateTime<Utc>,
        lease_seconds: i64,
    ) -> Self {
        Self {
            id,
            account_id,
            started_at: now,
            lease_until: now + Duration::seconds(lease_seconds),
            finished_at: None,
            status: SyncRunStatus::Running,
            error_category: None,
            error_message: None,
            retries: 0,
        }
    }

    /// Rehydrates a stored run.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: SyncRunId,
        account_id: AccountId,
        started_at: DateTime<Utc>,
        lease_until: DateTime<Utc>,
        finished_at: Option<DateTime<Utc>>,
        status: SyncRunStatus,
        error_category: Option<ErrorCategory>,
        error_message: Option<String>,
        retries: u32,
    ) -> Self {
        Self {
            id,
            account_id,
            started_at,
            lease_until,
            finished_at,
            status,
            error_category,
            error_message,
            retries,
        }
    }

    /// The run identifier.
    #[must_use]
    pub fn id(&self) -> SyncRunId {
        self.id
    }

    /// The account the run belongs to.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// When the run started.
    #[must_use]
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// The lease expiry.
    #[must_use]
    pub fn lease_until(&self) -> DateTime<Utc> {
        self.lease_until
    }

    /// When the run finished.
    #[must_use]
    pub fn finished_at(&self) -> Option<DateTime<Utc>> {
        self.finished_at
    }

    /// The lifecycle state.
    #[must_use]
    pub fn status(&self) -> SyncRunStatus {
        self.status
    }

    /// The failure class.
    #[must_use]
    pub fn error_category(&self) -> Option<ErrorCategory> {
        self.error_category
    }

    /// The non-secret failure summary.
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        self.error_message.as_deref()
    }

    /// How many retries the run has used.
    #[must_use]
    pub fn retries(&self) -> u32 {
        self.retries
    }

    /// Whether the lease is still live.
    #[must_use]
    pub fn is_live(&self, now: DateTime<Utc>) -> bool {
        self.status == SyncRunStatus::Running && self.lease_until > now
    }

    /// Extends the lease by `lease_seconds` from `now`. Returns `false` when the
    /// run no longer holds a live lease.
    pub fn renew(&mut self, now: DateTime<Utc>, lease_seconds: i64) -> bool {
        if !self.is_live(now) {
            return false;
        }
        self.lease_until = now + Duration::seconds(lease_seconds);
        true
    }

    /// Marks the run succeeded.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] outside the running state.
    pub fn succeed(&mut self, at: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != SyncRunStatus::Running {
            return Err(DomainError::InvalidTransition {
                entity: "sync run",
                from: self.status.as_str(),
                to: SyncRunStatus::Succeeded.as_str(),
            });
        }
        self.status = SyncRunStatus::Succeeded;
        self.finished_at = Some(at);
        Ok(())
    }

    /// Marks the run failed with a non-secret summary.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] outside the running state.
    pub fn fail(
        &mut self,
        at: DateTime<Utc>,
        category: ErrorCategory,
        message: &str,
    ) -> Result<(), DomainError> {
        if self.status != SyncRunStatus::Running {
            return Err(DomainError::InvalidTransition {
                entity: "sync run",
                from: self.status.as_str(),
                to: SyncRunStatus::Failed.as_str(),
            });
        }
        self.status = SyncRunStatus::Failed;
        self.finished_at = Some(at);
        self.error_category = Some(category);
        self.error_message = Some(sanitise_message(message));
        Ok(())
    }

    /// Marks a running run failed when its lease has expired (BR4.2).
    /// Returns `true` when the sweep changed the run.
    pub fn expire_if_due(&mut self, now: DateTime<Utc>) -> bool {
        if self.status == SyncRunStatus::Running && self.lease_until <= now {
            self.status = SyncRunStatus::Failed;
            self.finished_at = Some(now);
            self.error_category = Some(ErrorCategory::Data);
            self.error_message = Some("lease expired".to_string());
            return true;
        }
        false
    }
}

fn sanitise_message(message: &str) -> String {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return "sync failed".to_string();
    }
    if looks_like_secret(trimmed) {
        return "sync failed".to_string();
    }
    trimmed.chars().take(MAX_ERROR_MESSAGE_CHARS).collect()
}
