//! Ports defined inward and implemented outward by
//! `notification-infrastructure`.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use notification_domain::{
    Alert, AlertId, AlertStatus, Channel, DeliveryAttempt, NotificationPreference, UserId,
};
use uuid::Uuid;

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
    /// The dependency could not be reached or timed out; retry.
    #[error("dependency unavailable: {0}")]
    Unavailable(String),
    /// The dependency refused the request; do not retry.
    #[error("dependency refused the request: {0}")]
    Refused(String),
}

/// An injectable clock so retry, retention, and lease tests are deterministic.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// An injectable identifier source so erasure references are testable.
pub trait IdGenerator: Send + Sync {
    /// A fresh random identifier.
    fn next(&self) -> Uuid;
}

/// An injectable pause so audit retry tests never sleep.
#[async_trait]
pub trait Pause: Send + Sync {
    /// Waits the configured duration.
    async fn pause_for(&self, duration: Duration);
}

/// What an insert did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertOutcome {
    /// A new alert was stored.
    Inserted(Alert),
    /// The idempotency key already existed; BR1.1 returns deduplicated.
    Duplicate(Alert),
}

/// How many rows one erasure rewrote.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnonymisedRows {
    /// Alerts rewritten.
    pub alerts: u64,
    /// Delivery attempts rewritten.
    pub attempts: u64,
}

/// Reads and writes stored alerts.
#[async_trait]
pub trait AlertRepository: Send + Sync {
    /// Inserts one alert, or reports the stored alert holding the same key.
    async fn insert(&self, alert: &Alert) -> Result<InsertOutcome, RepositoryError>;

    /// Finds one alert by its idempotency key.
    async fn find_by_idempotency_key(&self, key: &str) -> Result<Option<Alert>, RepositoryError>;

    /// One keyset page of a user's feed, newest first. A type whose in-app
    /// preference is disabled is omitted (BR2.1, WF4.2); a type with no row
    /// keeps the default and appears.
    async fn list_for_user(
        &self,
        user_id: UserId,
        before: Option<(DateTime<Utc>, AlertId)>,
        limit: u32,
    ) -> Result<Vec<Alert>, RepositoryError>;

    /// Alerts whose next attempt is due.
    async fn due(&self, now: DateTime<Utc>, limit: u32) -> Result<Vec<Alert>, RepositoryError>;

    /// Whether any alert is due. The worker polls this with the wake flag.
    async fn has_due(&self, now: DateTime<Utc>) -> Result<bool, RepositoryError>;

    /// Persists the status and the next due time.
    async fn set_status(
        &self,
        id: AlertId,
        status: AlertStatus,
        next_attempt_at: Option<DateTime<Utc>>,
    ) -> Result<(), RepositoryError>;

    /// NFR13.1: replaces the user reference on the user's alerts and delivery
    /// attempts with one non-reversible reference, atomically.
    async fn anonymise_user(
        &self,
        user_id: UserId,
        replacement: UserId,
    ) -> Result<AnonymisedRows, RepositoryError>;

    /// Deletes one batch of alerts older than `cutoff`.
    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError>;
}

/// Reads and writes stored preferences.
#[async_trait]
pub trait PreferenceRepository: Send + Sync {
    /// Every stored row for a user.
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<NotificationPreference>, RepositoryError>;

    /// Inserts or replaces one row keyed by `(user_id, type, channel)`.
    async fn upsert(&self, preference: &NotificationPreference) -> Result<(), RepositoryError>;

    /// Inserts or replaces every row in one transaction, so a multi-type save is
    /// all-or-nothing.
    async fn upsert_all(
        &self,
        preferences: &[NotificationPreference],
    ) -> Result<(), RepositoryError>;
}

/// Reads and writes delivery attempts.
#[async_trait]
pub trait AttemptRepository: Send + Sync {
    /// Records one attempt.
    async fn insert(&self, attempt: &DeliveryAttempt) -> Result<(), RepositoryError>;

    /// Every attempt for one alert.
    async fn list_for_alert(
        &self,
        alert_id: AlertId,
    ) -> Result<Vec<DeliveryAttempt>, RepositoryError>;

    /// Deletes one batch of attempts older than `cutoff`.
    async fn delete_before(
        &self,
        cutoff: DateTime<Utc>,
        limit: u32,
    ) -> Result<u64, RepositoryError>;
}

/// The durable wake flag and the single delivery lease.
#[async_trait]
pub trait DeliveryControl: Send + Sync {
    /// Sets the durable wake flag (BR3.1 handoff, NFR2.5).
    async fn request_wake(&self, at: DateTime<Utc>) -> Result<(), RepositoryError>;

    /// Whether the wake flag is set.
    async fn wake_requested(&self) -> Result<bool, RepositoryError>;

    /// Clears the wake flag after a drained run.
    async fn clear_wake(&self) -> Result<(), RepositoryError>;

    /// Takes the single-flight delivery lease, unless a live lease is held.
    /// A new lease also resets the run's provider-call allowance.
    async fn acquire_lease(
        &self,
        owner: Uuid,
        now: DateTime<Utc>,
        ttl_seconds: i64,
    ) -> Result<bool, RepositoryError>;

    /// Atomically reserves one provider call inside the current run. Returns
    /// `false` once the run has used its whole allowance (NFR20.4).
    async fn reserve_email(&self, owner: Uuid, cap: u32) -> Result<bool, RepositoryError>;

    /// Releases the lease when `owner` still holds it.
    async fn release_lease(&self, owner: Uuid) -> Result<(), RepositoryError>;
}

/// Reads the verified contact from Identity's C3 at delivery time (BR4.2).
#[async_trait]
pub trait ContactDirectory: Send + Sync {
    /// `Ok(Some(address))` when a verified address exists; `Ok(None)` when it
    /// is missing or unverified (permanent); `Err` when C3 could not answer.
    async fn verified_contact(&self, user_id: &str) -> Result<Option<String>, ExternalError>;
}

/// One message for the transactional provider.
///
/// Deliberately not `Debug`: the recipient must never reach a log line.
pub struct EmailMessage {
    /// The verified recipient address.
    pub to: String,
    /// The subject line.
    pub subject: String,
    /// The plain-text body built from allow-listed payload values only.
    pub body: String,
}

/// Sends one email through the transactional provider.
#[async_trait]
pub trait EmailSender: Send + Sync {
    /// Sends the message. A refusal is permanent; an unavailability is retried.
    async fn send(&self, message: &EmailMessage) -> Result<(), ExternalError>;
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
    /// The logical target, never a payload value or full user reference.
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

/// Reports whether the service can serve traffic. Postgres only: a provider or
/// C3 outage must not fail readiness (NFR1.7).
#[async_trait]
pub trait ReadinessProbe: Send + Sync {
    /// Checks the critical dependency.
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

/// The channel a preference row concerns.
pub fn channels() -> &'static [Channel] {
    &Channel::ALL
}
