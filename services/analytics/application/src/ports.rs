//! Ports defined inward and implemented outward by `analytics-infrastructure`.

use std::collections::BTreeMap;

use analytics_domain::{
    AnalyticsEvent, ConsentCheck, FeatureFlag, Metric, MetricAggregate, Period,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// A persistence failure surfaced without leaking driver types inward.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RepositoryError {
    /// The store is unavailable or the query failed.
    #[error("repository unavailable: {0}")]
    Unavailable(String),
    /// The store returned data that violates a domain invariant.
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
    /// The dependency refused the request.
    #[error("dependency refused the request: {0}")]
    Refused(String),
}

/// An injectable clock so expiry, retention, and cache tests are deterministic.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// An injectable pause, so retention tests never sleep.
#[async_trait]
pub trait Pause: Send + Sync {
    /// Waits the configured pause.
    async fn pause(&self);
}

/// One salt version and its bytes. The bytes never leave the process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaltVersion {
    /// The Vault version label.
    pub version: String,
    /// The 32-byte salt.
    pub salt: Vec<u8>,
}

/// Reads the current salt and every retained version from Vault.
#[async_trait]
pub trait SaltProvider: Send + Sync {
    /// The version new pseudonyms are hashed under.
    async fn current(&self) -> Result<SaltVersion, ExternalError>;

    /// Every retained version, including the current one, for erasure.
    async fn retained(&self) -> Result<Vec<SaltVersion>, ExternalError>;
}

/// Checks analytics consent through C3.
#[async_trait]
pub trait ConsentChecker: Send + Sync {
    /// Whether analytics consent is granted for `user_id`.
    async fn analytics_granted(&self, user_id: &str) -> Result<bool, ExternalError>;
}

/// One C10 audit record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditRecord {
    /// The acting user, when there is one.
    pub actor_id: Option<String>,
    /// `user` or `system`.
    pub actor_kind: String,
    /// The dotted action name.
    pub action: String,
    /// The logical target, never a property value or full user reference.
    pub target: String,
    /// When the action happened.
    pub occurred_at: DateTime<Utc>,
    /// Identifier metadata only.
    pub context: BTreeMap<String, String>,
    /// `success`, `denied`, or `failed`.
    pub outcome: String,
    /// `routine` or `security`.
    pub sensitivity: String,
}

/// Writes C10 audit events.
#[async_trait]
pub trait AuditClient: Send + Sync {
    /// Appends one record.
    async fn append(&self, record: &AuditRecord) -> Result<(), ExternalError>;
}

/// The service a token was issued to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceCaller {
    /// The registered service name.
    pub service: String,
    /// The operations this caller may invoke.
    pub operations: Vec<String>,
}

impl ServiceCaller {
    /// Whether the caller may invoke `operation`.
    #[must_use]
    pub fn allows(&self, operation: &str) -> bool {
        self.operations.iter().any(|allowed| allowed == operation)
    }
}

/// Resolves an internal service token.
#[async_trait]
pub trait ServiceTokenRegistry: Send + Sync {
    /// Returns the caller for a known token hash, or `None`.
    async fn resolve(&self, token: &str) -> Result<Option<ServiceCaller>, RepositoryError>;
}

/// Reads and writes stored events.
#[async_trait]
pub trait EventRepository: Send + Sync {
    /// Inserts one event and, when present, its consent-check row atomically.
    async fn insert(
        &self,
        event: &AnalyticsEvent,
        consent: Option<&ConsentCheck>,
    ) -> Result<(), RepositoryError>;

    /// Events in `[from, to)`, capped at `limit`.
    async fn events_between(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<AnalyticsEvent>, RepositoryError>;

    /// Deletes one batch of events older than `cutoff` (BR5.1).
    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError>;

    /// Deletes every event whose `user_ref` matches any supplied hash (BR6.1).
    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError>;
}

/// Reads and writes consent-check rows.
#[async_trait]
pub trait ConsentRepository: Send + Sync {
    /// Inserts one check outcome.
    async fn insert(&self, check: &ConsentCheck) -> Result<(), RepositoryError>;

    /// Deletes one batch of checks older than `cutoff` (BR5.1).
    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError>;

    /// Deletes every check whose `user_ref` matches any supplied hash (BR6.1).
    async fn delete_by_user_refs(&self, user_refs: &[String]) -> Result<u64, RepositoryError>;
}

/// Reads and writes aggregate figures.
#[async_trait]
pub trait AggregateRepository: Send + Sync {
    /// Upserts one figure keyed by `(metric, period, cohort, period_start)`.
    async fn upsert(&self, aggregate: &MetricAggregate) -> Result<(), RepositoryError>;

    /// Lists figures, newest first.
    async fn list(
        &self,
        metric: Option<Metric>,
        period: Option<Period>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<MetricAggregate>, RepositoryError>;
}

/// Reads and writes feature flags.
#[async_trait]
pub trait FlagRepository: Send + Sync {
    /// Inserts a new flag.
    async fn insert(&self, flag: &FeatureFlag) -> Result<(), RepositoryError>;

    /// Persists a changed flag.
    async fn update(&self, flag: &FeatureFlag) -> Result<(), RepositoryError>;

    /// Deletes a flag by key. Returns whether a row was removed.
    async fn delete(&self, key: &str) -> Result<bool, RepositoryError>;

    /// Finds one flag by key.
    async fn find_by_key(&self, key: &str) -> Result<Option<FeatureFlag>, RepositoryError>;

    /// Every flag.
    async fn list(&self) -> Result<Vec<FeatureFlag>, RepositoryError>;
}

/// Reports whether the service can serve traffic.
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks the critical dependencies.
    ///
    /// # Errors
    /// Returns a human-readable reason when the service is not ready.
    async fn check(&self) -> Result<(), String>;
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

/// The authenticated caller.
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
    /// An administrator.
    Admin,
    /// A support operator.
    Support,
}

/// Validates a Supabase JWT against JWKS.
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the caller.
    async fn validate(&self, token: &str) -> Result<Principal, AuthError>;
}
