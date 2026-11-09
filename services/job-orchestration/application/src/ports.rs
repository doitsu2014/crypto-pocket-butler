//! Ports defined inward and implemented outward by `job-orchestration-infrastructure`.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use job_orchestration_domain::{JobRun, RunId, RunStatus, UserId};

/// A persistence or external failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// The store is unavailable or the query failed.
    #[error("repository unavailable: {0}")]
    Unavailable(String),
    /// The store returned data that violates the domain invariants.
    #[error("stored data is invalid: {0}")]
    InvalidData(String),
    /// A uniqueness or state rule was violated.
    #[error("conflict: {0}")]
    Conflict(String),
    /// The role lacks the grant.
    #[error("refused: {0}")]
    Forbidden(String),
}

/// A failed call to a provider.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ExternalError {
    /// The provider could not be reached.
    #[error("provider unavailable: {0}")]
    Unavailable(String),
}

/// What the provider answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAnswer {
    /// 200 with status succeeded (or a bodiless 200/204).
    Succeeded,
    /// 200 with status failed.
    Failed,
    /// 200 with status skipped.
    Skipped,
    /// 202 with a run id; the provider owns the window.
    AlreadyRunning,
    /// 409; a per-item lease is held.
    LeaseHeld,
    /// Any other 4xx.
    Refused(u16),
    /// 5xx, timeout, or unreachable.
    Unavailable,
}

/// Calls a provider's C11 run endpoint (BR6.2).
#[async_trait]
pub trait TargetClient: Send + Sync {
    /// Runs one window. `idempotency_key` is `job:{job}:{window}`.
    async fn run(
        &self,
        provider: &str,
        job: &str,
        scheduled_for: DateTime<Utc>,
        owner: &str,
        idempotency_key: &str,
        timeout_seconds: u64,
    ) -> Result<TargetAnswer, ExternalError>;
}

/// A job definition row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JobDefinition {
    /// The job name.
    pub name: String,
    /// The provider service key.
    pub provider: String,
    /// The parsed schedule text.
    pub schedule: String,
    /// Per-call timeout.
    pub timeout_seconds: u64,
    /// Bounded attempts.
    pub max_attempts: u32,
    /// Whether the job is enabled.
    pub enabled: bool,
    /// The newest window already scheduled.
    pub last_scheduled_for: Option<DateTime<Utc>>,
}

/// Reads and writes jobs and runs.
#[async_trait]
pub trait RunRepository: Send + Sync {
    /// Lists the enabled job definitions.
    async fn definitions(&self) -> Result<Vec<JobDefinition>, RepositoryError>;
    /// Inserts a queued attempt, doing nothing when `(job, window, attempt)`
    /// already exists (BR1.2).
    async fn insert(&self, run: &JobRun) -> Result<(), RepositoryError>;
    /// The newest attempt for a window.
    async fn latest_for_window(
        &self,
        job: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<Option<JobRun>, RepositoryError>;
    /// Whether any attempt for the window exists.
    async fn window_exists(
        &self,
        job: &str,
        scheduled_for: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
    /// Lists up to `limit` pending rows, oldest window first.
    async fn pending(&self, limit: i64) -> Result<Vec<JobRun>, RepositoryError>;
    /// Moves a pending row to running under `owner`; false when it was not pending.
    async fn claim(
        &self,
        id: RunId,
        owner: &str,
        lease_until: DateTime<Utc>,
        started_at: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
    /// Records an outcome only while `owner` still holds the lease; false otherwise.
    async fn finish(
        &self,
        run: &JobRun,
        owner: &str,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
    /// Marks running rows with an expired lease failed and returns them.
    async fn reap_expired(&self, now: DateTime<Utc>) -> Result<Vec<JobRun>, RepositoryError>;
    /// Failed rows whose retry is due.
    async fn due_retries(&self, now: DateTime<Utc>) -> Result<Vec<JobRun>, RepositoryError>;
    /// Lists runs for a job between two instants, newest first, bounded.
    async fn list_for_job(
        &self,
        job: &str,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
    ) -> Result<Vec<JobRun>, RepositoryError>;
    /// The newest attempt per job, for the operator list.
    async fn latest_per_job(&self) -> Result<Vec<JobRun>, RepositoryError>;
    /// Updates a job's watermark.
    async fn set_watermark(&self, job: &str, window: DateTime<Utc>) -> Result<(), RepositoryError>;
    /// Deletes runs whose window is older than the cutoff, in bounded batches.
    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError>;
}

/// The orchestrator lease (BR1.3).
#[async_trait]
pub trait LeaseRepository: Send + Sync {
    /// Claims or renews the lease for `owner`. Returns `false` when another owner
    /// holds a live lease.
    async fn claim_or_renew(
        &self,
        owner: &str,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
    /// Whether `owner` still holds a live lease.
    async fn owned(&self, owner: &str, now: DateTime<Utc>) -> Result<bool, RepositoryError>;
}

/// The heartbeat row (BR4).
#[async_trait]
pub trait HeartbeatRepository: Send + Sync {
    /// Writes the heartbeat when `owner` holds the lease.
    async fn write(&self, owner: &str, now: DateTime<Utc>) -> Result<(), RepositoryError>;
    /// The heartbeat age, shared by every replica.
    async fn age_seconds(&self, now: DateTime<Utc>) -> Result<Option<i64>, RepositoryError>;
}

/// Appends C10 events.
#[async_trait]
pub trait AuditClient: Send + Sync {
    /// Appends one event; a refusal is logged by the caller and never blocks.
    async fn append(&self, event: &AuditEvent) -> Result<(), RepositoryError>;
}

/// One audit event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// The actor, absent for system events.
    pub actor_id: Option<UserId>,
    /// The allow-listed action.
    pub action: String,
    /// The target (`job:{job}:{window}`).
    pub target: String,
    /// The outcome label.
    pub outcome: String,
    /// When it happened.
    pub occurred_at: DateTime<Utc>,
}

/// An injectable clock.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Generates identifiers.
pub trait IdGenerator: Send + Sync {
    /// The next identifier.
    fn next(&self) -> uuid::Uuid;
}

/// Validates a Supabase JWT and returns its claims.
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token.
    async fn validate(&self, token: &str) -> Result<VerifiedClaims, AuthError>;
}

/// The facts derived from a validated JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedClaims {
    /// The user id.
    pub user_id: UserId,
    /// The application role from `app_metadata.role`.
    pub role: String,
}

/// A token-validation failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    /// No token was presented.
    #[error("missing bearer token")]
    Missing,
    /// The token is invalid.
    #[error("invalid token: {0}")]
    Invalid(String),
    /// The identity provider is unavailable.
    #[error("identity provider unavailable: {0}")]
    Unavailable(String),
}

/// A registered caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredService {
    /// The caller name.
    pub name: String,
    /// The operations it may call.
    pub operations: Vec<job_orchestration_domain::Operation>,
}

/// Resolves inbound service tokens.
#[async_trait]
pub trait ServiceTokenRegistry: Send + Sync {
    /// Returns the registered caller for a token.
    async fn service_for(&self, token: &str) -> Result<Option<RegisteredService>, RepositoryError>;
}

/// Reports readiness.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks the service's own database.
    async fn check(&self) -> Result<(), String>;
}

/// No-op status used by tests and the placeholder wiring.
pub fn status_label(status: RunStatus) -> &'static str {
    status.as_str()
}
