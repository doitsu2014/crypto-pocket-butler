//! Ports defined inward and implemented outward by `reporting-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reporting_domain::{
    AuditRecord, Brief, BriefDelivery, BriefId, Period, PnlRead, UserId, ValuationRead,
};
use serde_json::Value;
use uuid::Uuid;

use crate::common::VerifiedClaims;

/// A persistence failure surfaced without leaking driver types inward.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// The store is unavailable or the query failed.
    #[error("repository unavailable: {0}")]
    Unavailable(String),

    /// The store returned data that does not satisfy the domain invariants.
    #[error("stored data is invalid: {0}")]
    InvalidData(String),

    /// A uniqueness or state rule was violated.
    #[error("conflict: {0}")]
    Conflict(String),

    /// The database refused the statement because the role lacks the grant.
    #[error("refused: {0}")]
    Forbidden(String),
}

/// A failed call to an external dependency.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExternalError {
    /// The dependency could not be reached or timed out.
    #[error("dependency unavailable: {0}")]
    Unavailable(String),

    /// The dependency refused the request; a retry will not help.
    #[error("dependency refused the request: {0}")]
    Refused(String),
}

/// A stored brief and the JSON body of its sections.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredBrief {
    /// The brief record.
    pub brief: Brief,
    /// The composed sections, absent until the brief is generated.
    pub sections: Option<Value>,
}

/// Reads and writes briefs.
#[async_trait]
pub trait BriefRepository: Send + Sync {
    /// Inserts or replaces a generated brief in its window. An existing generated
    /// brief for the same window is returned unchanged (BR1.1: one brief per
    /// window, never regenerated).
    async fn upsert_generated(
        &self,
        brief: &Brief,
        sections: &Value,
    ) -> Result<Brief, RepositoryError>;

    /// Inserts or replaces a failed brief in its window. A generated brief is
    /// never replaced by a failure.
    async fn upsert_failed(&self, brief: &Brief) -> Result<Brief, RepositoryError>;

    /// Finds one brief by identifier.
    async fn find_by_id(&self, id: BriefId) -> Result<Option<StoredBrief>, RepositoryError>;

    /// Finds the brief for one user's period window, when one exists.
    async fn find_window(
        &self,
        user_id: UserId,
        period: Period,
        period_start: DateTime<Utc>,
    ) -> Result<Option<StoredBrief>, RepositoryError>;

    /// The user's newest generated brief.
    async fn latest_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<StoredBrief>, RepositoryError>;

    /// Lists one page of the user's briefs, newest first, using an `updated_at`
    /// cursor. A period narrows the list to daily or weekly.
    async fn list_for_user(
        &self,
        user_id: UserId,
        period: Option<Period>,
        limit: i64,
        before: Option<DateTime<Utc>>,
    ) -> Result<Vec<Brief>, RepositoryError>;

    /// Lists every brief of the user for a privacy export.
    async fn list_for_user_export(
        &self,
        user_id: UserId,
    ) -> Result<Vec<StoredBrief>, RepositoryError>;

    /// Replaces the user id on the user's briefs with a random opaque reference
    /// (privacy anonymise). Returns the number of rows changed.
    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError>;

    /// Deletes generated briefs older than the retention cutoff. This runs only
    /// through the migrator CLI: the runtime role has no DELETE grant.
    async fn delete_generated_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError>;
}

/// Reads and writes brief deliveries.
#[async_trait]
pub trait DeliveryRepository: Send + Sync {
    /// Records a delivery attempt; a retry on the same channel and brief replaces
    /// the earlier attempt.
    async fn upsert(&self, delivery: &BriefDelivery) -> Result<(), RepositoryError>;

    /// Lists the delivery attempts for one brief, oldest first.
    async fn list_for_brief(
        &self,
        brief_id: BriefId,
    ) -> Result<Vec<BriefDelivery>, RepositoryError>;
}

/// One user C3 reports as due for a period.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueUser {
    /// The user identifier.
    pub user_id: UserId,
    /// The IANA timezone the period boundary was measured in.
    pub timezone: String,
}

/// Reads C3's due users.
#[async_trait]
pub trait DueUsersClient: Send + Sync {
    /// Returns the users whose local boundary for `period` has passed at `at`.
    async fn due(&self, period: Period, at: DateTime<Utc>) -> Result<Vec<DueUser>, ExternalError>;
}

/// Reads C8's valuation for a user.
#[async_trait]
pub trait ValuationClient: Send + Sync {
    /// Returns the user's current valuation with its lines and provenance.
    async fn valuation(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> Result<ValuationRead, ExternalError>;
}

/// Reads C7's realised performance for a window.
#[async_trait]
pub trait PerformanceClient: Send + Sync {
    /// Returns the user's realised performance between two instants.
    async fn performance(
        &self,
        user_id: UserId,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Result<PnlRead, ExternalError>;
}

/// Sends C9 alerts.
#[async_trait]
pub trait NotificationClient: Send + Sync {
    /// Submits an alert deduplicated by `idempotency_key` on one channel (BR2.2).
    /// Returns the alert identifier, or `None` when Notification suppressed it.
    async fn notify(
        &self,
        user_id: UserId,
        alert_type: &str,
        idempotency_key: &str,
        channel: &str,
        payload: Value,
    ) -> Result<Option<String>, ExternalError>;
}

/// Appends C10 events.
#[async_trait]
pub trait AuditClient: Send + Sync {
    /// Appends one event.
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError>;
}

/// A stored brief-generation run and its lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BriefRun {
    /// The run identifier.
    pub id: Uuid,
    /// The job name.
    pub job: String,
    /// The owner instance.
    pub owner: String,
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// The lease expiry.
    pub lease_until: DateTime<Utc>,
    /// When the run finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// The terminal status, when finished.
    pub status: RunStatus,
    /// A short non-secret detail.
    pub detail: Option<String>,
}

/// The status of a brief-generation run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunStatus {
    /// Holding the single-flight lease.
    Running,
    /// Finished successfully.
    Succeeded,
    /// Finished with a failure.
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
}

/// Reads and writes the single-flight brief-generation lease.
#[async_trait]
pub trait RunRepository: Send + Sync {
    /// Claims the job's lease. Returns `false` when a live lease already exists.
    async fn claim(&self, run: &BriefRun) -> Result<bool, RepositoryError>;

    /// Extends a live lease. Returns `false` when the run no longer owns it.
    async fn renew(
        &self,
        id: Uuid,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    /// Persists a terminal state.
    async fn finish(&self, run: &BriefRun) -> Result<(), RepositoryError>;

    /// Marks every running run with an expired lease failed and returns it.
    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<BriefRun>, RepositoryError>;
}

/// One recorded privacy anonymisation, used to make a retry apply once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymisationRecord {
    /// The random opaque reference that replaced the subject.
    pub anonymised_ref: UserId,
    /// The caller's idempotency key.
    pub idempotency_key: String,
    /// The categories kept under a legal basis.
    pub retained_categories: Vec<String>,
    /// When the anonymisation completed.
    pub completed_at: DateTime<Utc>,
}

/// Reads and writes the privacy anonymisation ledger.
#[async_trait]
pub trait PrivacyRepository: Send + Sync {
    /// Finds a completed anonymisation by the caller's idempotency key.
    async fn find_anonymisation(
        &self,
        idempotency_key: &str,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError>;

    /// Records a completed anonymisation.
    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError>;
}

/// An injectable clock so period and lease tests are deterministic.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Generates identifiers, so tests are deterministic.
pub trait IdGenerator: Send + Sync {
    /// The next identifier.
    fn next(&self) -> Uuid;
}

/// Validates a Supabase JWT against JWKS (signature, issuer, audience, expiry).
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the facts Reporting derives from it.
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError>;
}

/// A token-validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    /// No bearer token was presented.
    #[error("missing bearer token")]
    Missing,
    /// The token failed signature, issuer, audience, expiry, or shape checks.
    #[error("invalid token: {0}")]
    Invalid(String),
    /// JWKS could not be refreshed inside the staleness window.
    #[error("identity provider unavailable: {0}")]
    Unavailable(String),
}

/// A registered internal caller and its allow-list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredService {
    /// The caller name.
    pub name: String,
    /// The operations it may call.
    pub operations: Vec<reporting_domain::Operation>,
}

impl RegisteredService {
    /// Whether the caller may call the operation.
    #[must_use]
    pub fn allows(&self, operation: reporting_domain::Operation) -> bool {
        self.operations.contains(&operation)
    }
}

/// Resolves inbound service tokens from the registry and outbound tokens from
/// the secret file.
#[async_trait]
pub trait ServiceTokenRegistry: Send + Sync {
    /// Returns the registered caller for a presented token, or `None` when the
    /// token is unknown.
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError>;

    /// Returns the token issued to one service, for outbound calls.
    fn token_for(&self, service: &str) -> Option<String>;
}

/// Reports whether the service can serve traffic.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks the service's critical dependencies.
    async fn check(&self) -> Result<(), String>;
}
