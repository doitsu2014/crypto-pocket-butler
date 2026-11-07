//! Refresh jobs and their single-flight leases (BR4.1–BR4.5).

use chrono::{DateTime, Duration, Utc};

use crate::constants::RUN_LEASE_SECONDS;
use crate::error::DomainError;
use crate::ids::RefreshRunId;
use crate::secrets::sanitise_message;

/// The two refresh jobs. Each holds its own lease and may run concurrently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshJob {
    /// Ranks 1–100, every five minutes, deadline 180 seconds.
    Top100,
    /// Ranks 101+, every ten minutes, deadline 600 seconds.
    Tail,
}

impl RefreshJob {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Top100 => "top100",
            Self::Tail => "tail",
        }
    }

    /// The C11 job name.
    #[must_use]
    pub fn job_name(self) -> &'static str {
        match self {
            Self::Top100 => "price-refresh-top100",
            Self::Tail => "price-refresh-tail",
        }
    }

    /// Parses a stored label.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown job.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "top100" => Ok(Self::Top100),
            "tail" => Ok(Self::Tail),
            _ => Err(DomainError::Invalid {
                field: "job",
                reason: "must be top100 or tail".to_string(),
            }),
        }
    }

    /// Parses the C11 path's job name.
    #[must_use]
    pub fn parse_job_name(raw: &str) -> Option<Self> {
        match raw.trim() {
            "price-refresh-top100" => Some(Self::Top100),
            "price-refresh-tail" => Some(Self::Tail),
            _ => None,
        }
    }

    /// The hard deadline of one run.
    #[must_use]
    pub fn deadline_seconds(self) -> i64 {
        match self {
            Self::Top100 => 180,
            Self::Tail => 600,
        }
    }

    /// The canonical rank slice this job prices.
    #[must_use]
    pub fn selects_rank(self, rank: u32) -> bool {
        match self {
            Self::Top100 => rank <= 100,
            Self::Tail => rank > 100,
        }
    }
}

/// Why a run was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// Job Orchestration's scheduled call.
    Scheduled,
    /// An operator's manual call.
    Manual,
}

impl Trigger {
    /// The stored and audit label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scheduled => "scheduled",
            Self::Manual => "manual",
        }
    }

    /// Parses the `X-Run-Trigger` value.
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] for an unknown trigger.
    pub fn parse(raw: &str) -> Result<Self, DomainError> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "scheduled" => Ok(Self::Scheduled),
            "manual" => Ok(Self::Manual),
            _ => Err(DomainError::Invalid {
                field: "trigger",
                reason: "must be scheduled or manual".to_string(),
            }),
        }
    }
}

/// The run lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// The run owns the lease.
    Running,
    /// The run finished and its counts reconcile.
    Succeeded,
    /// The run failed or its lease expired.
    Failed,
}

impl RunStatus {
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

/// The failure class recorded on a failed run (never a provider body).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// A source rate limit deferred the work.
    RateLimit,
    /// A provider answered with an error.
    Provider,
    /// A provider or the network was unreachable.
    Network,
    /// A reading or a dependency's data could not be used.
    Data,
}

impl ErrorCategory {
    /// The stored label.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
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
            "rate_limit" => Ok(Self::RateLimit),
            "provider" => Ok(Self::Provider),
            "network" => Ok(Self::Network),
            "data" => Ok(Self::Data),
            _ => Err(DomainError::Invalid {
                field: "errorCategory",
                reason: "must be rate_limit, provider, network, or data".to_string(),
            }),
        }
    }
}

/// One scheduled or manual price-and-FX refresh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshRun {
    id: RefreshRunId,
    job: RefreshJob,
    started_at: DateTime<Utc>,
    lease_until: DateTime<Utc>,
    finished_at: Option<DateTime<Utc>>,
    status: RunStatus,
    requested_count: u32,
    priced_count: u32,
    unpriced_count: u32,
    error_category: Option<ErrorCategory>,
    error_message: Option<String>,
}

impl RefreshRun {
    /// Claims a new run with a live lease.
    #[must_use]
    pub fn claim(id: RefreshRunId, job: RefreshJob, now: DateTime<Utc>) -> Self {
        Self {
            id,
            job,
            started_at: now,
            lease_until: now + Duration::seconds(RUN_LEASE_SECONDS),
            finished_at: None,
            status: RunStatus::Running,
            requested_count: 0,
            priced_count: 0,
            unpriced_count: 0,
            error_category: None,
            error_message: None,
        }
    }

    /// Rehydrates a stored run.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn from_stored(
        id: RefreshRunId,
        job: RefreshJob,
        started_at: DateTime<Utc>,
        lease_until: DateTime<Utc>,
        finished_at: Option<DateTime<Utc>>,
        status: RunStatus,
        requested_count: u32,
        priced_count: u32,
        unpriced_count: u32,
        error_category: Option<ErrorCategory>,
        error_message: Option<String>,
    ) -> Self {
        Self {
            id,
            job,
            started_at,
            lease_until,
            finished_at,
            status,
            requested_count,
            priced_count,
            unpriced_count,
            error_category,
            error_message,
        }
    }

    /// The run identifier.
    #[must_use]
    pub fn id(&self) -> RefreshRunId {
        self.id
    }

    /// The job the run belongs to.
    #[must_use]
    pub fn job(&self) -> RefreshJob {
        self.job
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
    pub fn status(&self) -> RunStatus {
        self.status
    }

    /// How many assets the run attempted.
    #[must_use]
    pub fn requested_count(&self) -> u32 {
        self.requested_count
    }

    /// How many assets were priced.
    #[must_use]
    pub fn priced_count(&self) -> u32 {
        self.priced_count
    }

    /// How many assets remained unpriced.
    #[must_use]
    pub fn unpriced_count(&self) -> u32 {
        self.unpriced_count
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

    /// Whether the lease is still live.
    #[must_use]
    pub fn is_live(&self, now: DateTime<Utc>) -> bool {
        self.status == RunStatus::Running && self.lease_until > now
    }

    /// Extends the lease by [`RUN_LEASE_SECONDS`] from `now`. Returns `false`
    /// when the run no longer holds a live lease; the caller must then stop.
    pub fn renew(&mut self, now: DateTime<Utc>) -> bool {
        if !self.is_live(now) {
            return false;
        }
        self.lease_until = now + Duration::seconds(RUN_LEASE_SECONDS);
        true
    }

    /// Records the reconciling counts (BR4.5).
    ///
    /// # Errors
    /// Returns [`DomainError::Invalid`] when the counts do not add up and
    /// [`DomainError::InvalidTransition`] outside the running state.
    pub fn record_counts(
        &mut self,
        requested: u32,
        priced: u32,
        unpriced: u32,
    ) -> Result<(), DomainError> {
        if self.status != RunStatus::Running {
            return Err(DomainError::InvalidTransition {
                entity: "refresh run",
                from: self.status.as_str(),
                to: "running",
            });
        }
        if requested != priced + unpriced {
            return Err(DomainError::Invalid {
                field: "requestedCount",
                reason: "must equal pricedCount plus unpricedCount".to_string(),
            });
        }
        self.requested_count = requested;
        self.priced_count = priced;
        self.unpriced_count = unpriced;
        Ok(())
    }

    /// Marks the run succeeded.
    ///
    /// # Errors
    /// Returns [`DomainError::InvalidTransition`] outside the running state and
    /// [`DomainError::Invalid`] when the counts do not reconcile.
    pub fn succeed(&mut self, at: DateTime<Utc>) -> Result<(), DomainError> {
        if self.status != RunStatus::Running {
            return Err(DomainError::InvalidTransition {
                entity: "refresh run",
                from: self.status.as_str(),
                to: RunStatus::Succeeded.as_str(),
            });
        }
        if self.requested_count != self.priced_count + self.unpriced_count {
            return Err(DomainError::Invalid {
                field: "requestedCount",
                reason: "must equal pricedCount plus unpricedCount".to_string(),
            });
        }
        self.status = RunStatus::Succeeded;
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
        if self.status != RunStatus::Running {
            return Err(DomainError::InvalidTransition {
                entity: "refresh run",
                from: self.status.as_str(),
                to: RunStatus::Failed.as_str(),
            });
        }
        self.status = RunStatus::Failed;
        self.finished_at = Some(at);
        self.error_category = Some(category);
        self.error_message = Some(sanitise_message(message, "refresh failed"));
        Ok(())
    }

    /// Marks a running run failed when its lease has expired. Returns `true`
    /// when the sweep changed the run.
    pub fn expire_if_due(&mut self, now: DateTime<Utc>) -> bool {
        if self.status == RunStatus::Running && self.lease_until <= now {
            self.status = RunStatus::Failed;
            self.finished_at = Some(now);
            self.error_category = Some(ErrorCategory::Data);
            self.error_message = Some("lease expired".to_string());
            return true;
        }
        false
    }
}
