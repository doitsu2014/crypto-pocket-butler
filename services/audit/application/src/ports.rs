//! Ports defined inward and implemented outward by `audit-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use async_trait::async_trait;
use audit_domain::{
    AuditEvent, AuditEventId, AuditExport, DeletionRequestId, EventLabel, PartitionSummary,
    PrivacyDeletion, RetentionRun, Sensitivity, SupportSession, SupportSessionId, UserId,
};
use chrono::{DateTime, Utc};

pub use audit_domain::ExportFilter;

/// A persistence failure surfaced without leaking driver types inward.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// The store is unavailable or the query failed.
    #[error("repository unavailable: {0}")]
    Unavailable(String),

    /// The store returned data that does not satisfy the domain invariants.
    #[error("stored data is invalid: {0}")]
    InvalidData(String),

    /// A uniqueness rule was violated.
    #[error("conflict: {0}")]
    Conflict(String),

    /// The database refused the statement because the role lacks the grant.
    #[error("refused: {0}")]
    Forbidden(String),
}

/// The position of the last row returned by a cursor-paginated read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    /// The event's `occurredAt`.
    pub occurred_at: DateTime<Utc>,
    /// The event identifier, breaking ties at the same instant.
    pub id: AuditEventId,
}

/// One admin query over the trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventQuery {
    /// A dotted action prefix.
    pub action_prefix: Option<String>,
    /// An exact target.
    pub target: Option<String>,
    /// Only events at this sensitivity.
    pub sensitivity: Option<Sensitivity>,
    /// The lower bound on `occurredAt`.
    pub occurred_from: Option<DateTime<Utc>>,
    /// The upper bound on `occurredAt`.
    pub occurred_to: Option<DateTime<Utc>>,
    /// The mandatory lower bound on `recordedAt` (the partition key).
    pub recorded_from: DateTime<Utc>,
    /// The mandatory upper bound on `recordedAt`.
    pub recorded_to: DateTime<Utc>,
    /// Where to continue from.
    pub cursor: Option<Cursor>,
    /// The page size.
    pub limit: u32,
}

/// The result of an append attempt (BR2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppendOutcome {
    /// The event was inserted.
    Appended,
    /// The idempotency key already exists; carries the original event and
    /// whether the retried payload matched it.
    Duplicate {
        /// The event stored by the first attempt.
        event_id: AuditEventId,
        /// Whether the retried payload hashes to the same value.
        same_payload: bool,
    },
}

/// Reads and appends audit events.
#[async_trait]
pub trait AuditEventRepository: Send + Sync {
    /// Inserts one event, and its idempotency row when the emitter sent a key.
    async fn append(
        &self,
        event: &AuditEvent,
        emitting_service: &str,
        payload_sha256: &str,
    ) -> Result<AppendOutcome, RepositoryError>;

    /// Finds one event by identifier.
    async fn find_by_id(&self, id: AuditEventId) -> Result<Option<AuditEvent>, RepositoryError>;

    /// Returns one page of events, newest first.
    async fn query(&self, query: &EventQuery) -> Result<Vec<AuditEvent>, RepositoryError>;

    /// Counts matching rows, stopping at `cap`.
    async fn count_for_export(
        &self,
        filter: &ExportFilter,
        cap: u64,
    ) -> Result<u64, RepositoryError>;

    /// Returns one export batch, oldest first, after `after`.
    async fn export_batch(
        &self,
        filter: &ExportFilter,
        after: Option<Cursor>,
        limit: u32,
    ) -> Result<Vec<AuditEvent>, RepositoryError>;
}

/// Reads and writes support sessions.
#[async_trait]
pub trait SupportSessionRepository: Send + Sync {
    /// Inserts a new session.
    async fn insert(&self, session: &SupportSession) -> Result<(), RepositoryError>;

    /// Finds a session by identifier.
    async fn find_by_id(
        &self,
        id: SupportSessionId,
    ) -> Result<Option<SupportSession>, RepositoryError>;

    /// Persists a state change (column-scoped update).
    async fn update(&self, session: &SupportSession) -> Result<(), RepositoryError>;
}

/// Reads and writes export requests.
#[async_trait]
pub trait ExportRepository: Send + Sync {
    /// Inserts a new export request.
    async fn insert(&self, export: &AuditExport) -> Result<(), RepositoryError>;

    /// Persists a terminal state (column-scoped update).
    async fn update(&self, export: &AuditExport) -> Result<(), RepositoryError>;

    /// Marks `requested` rows older than `before` as failed (recovery on start).
    async fn mark_stale_failed(
        &self,
        before: DateTime<Utc>,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError>;
}

/// Runs and records retention sweeps.
#[async_trait]
pub trait RetentionRepository: Send + Sync {
    /// Inserts a new run.
    async fn insert_run(&self, run: &RetentionRun) -> Result<(), RepositoryError>;

    /// Persists a terminal state (column-scoped update).
    async fn update_run(&self, run: &RetentionRun) -> Result<(), RepositoryError>;

    /// Finds a run by identifier.
    async fn find_run(
        &self,
        id: audit_domain::RetentionRunId,
    ) -> Result<Option<RetentionRun>, RepositoryError>;

    /// Summarises every partition of the trail.
    async fn partition_summaries(&self) -> Result<Vec<PartitionSummary>, RepositoryError>;

    /// Drops one whole partition, re-checking the window and legal holds inside
    /// the database before the destructive statement.
    async fn drop_partition(
        &self,
        name: &str,
        cutoff: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    /// Deletes one batch of unheld rows strictly older than `cutoff`.
    async fn delete_batch(&self, cutoff: DateTime<Utc>, limit: u32)
        -> Result<u64, RepositoryError>;

    /// Deletes idempotency rows whose event can no longer exist because the
    /// retention window has passed. Returns the number removed.
    async fn delete_expired_idempotency(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, RepositoryError>;

    /// Creates partitions for the current month and the next `months_ahead`.
    async fn ensure_partitions_ahead(
        &self,
        months_ahead: u32,
    ) -> Result<Vec<String>, RepositoryError>;
}

/// Reads and writes privacy deletions, and executes them under the retention
/// credential.
#[async_trait]
pub trait PrivacyDeletionRepository: Send + Sync {
    /// Inserts a request.
    async fn insert(&self, deletion: &PrivacyDeletion) -> Result<(), RepositoryError>;

    /// Persists a terminal state.
    async fn update(&self, deletion: &PrivacyDeletion) -> Result<(), RepositoryError>;

    /// Finds a request by Identity's deletion request id.
    async fn find_by_request(
        &self,
        request: DeletionRequestId,
    ) -> Result<Option<PrivacyDeletion>, RepositoryError>;

    /// Returns up to `limit` requests still waiting to be executed.
    async fn next_requested(&self, limit: u32) -> Result<Vec<PrivacyDeletion>, RepositoryError>;

    /// Examines up to `limit` of the subject's events.
    async fn labels_for_subject(
        &self,
        subject: UserId,
        limit: u32,
    ) -> Result<Vec<EventLabel>, RepositoryError>;

    /// Deletes exactly these identifiers, which must belong to `subject` and
    /// carry no legal basis. Returns the number removed.
    async fn delete_for_subject(
        &self,
        subject: UserId,
        ids: &[AuditEventId],
    ) -> Result<u64, RepositoryError>;
}

/// An injectable clock so tests control time.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// An injectable pause between retention batches, so tests never sleep.
#[async_trait]
pub trait Pause: Send + Sync {
    /// Waits the configured pause.
    async fn pause(&self);
}

/// The authenticated operator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// The Supabase Auth subject.
    pub subject: String,
    /// The role carried in `app_metadata.role`.
    pub role: Role,
}

/// Application roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// A regular user.
    User,
    /// An audit administrator.
    Admin,
    /// A support operator.
    Support,
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

/// Validates a Supabase JWT against JWKS (signature, issuer, audience, expiry).
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the caller.
    async fn validate(&self, token: &str) -> Result<Principal, AuthError>;
}

/// Resolves an internal service token to the emitting service.
pub trait ServiceTokenRegistry: Send + Sync {
    /// Returns the service a token was issued to, or `None` when the token is
    /// unknown. Unknown tokens never append.
    fn service_for(&self, token: &str) -> Option<String>;
}

/// Reports whether the service can serve traffic.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks the service's critical dependencies.
    ///
    /// # Errors
    /// Returns a human-readable reason when the service is not ready.
    async fn check(&self) -> Result<(), String>;
}
