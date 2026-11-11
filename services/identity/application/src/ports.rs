//! Ports defined inward and implemented outward by `identity-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use identity_domain::{
    AuditRecord, ConsentPurpose, ConsentRecord, Contributor, DeletionRequest, DeviceSession,
    ExportRequest, JurisdictionRule, RetainedCategory, ShreddingKey, StatusTransition,
    SupportAccessGrant, User, UserId, UserStatus, VerifiedClaims,
};

/// One status change the lifecycle store applies atomically: the `users` status
/// column, the append-only transition, and the audit intent (BR5.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusChange {
    /// The account whose status changes.
    pub user_id: UserId,
    /// The status the caller observed.
    pub from: UserStatus,
    /// The status the edge moves to.
    pub to: UserStatus,
    /// The owner id, the admin id, or `system`.
    pub actor: String,
    /// `owner`, `admin`, or `system`; the audit payload carries it.
    pub actor_kind: &'static str,
    /// The admin reason, when the rule requires one.
    pub reason: Option<String>,
    /// When the change was requested.
    pub occurred_at: DateTime<Utc>,
}

/// Applies status changes, transitions, and audit intents in one transaction.
#[async_trait]
pub trait StatusTransitionStore: Send + Sync {
    /// Writes `users.status`, appends the transition, and enqueues its audit
    /// row. A stored row whose status no longer equals `change.from` is refused
    /// with [`RepositoryError::Conflict`].
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the transaction cannot commit.
    async fn apply(&self, change: &StatusChange) -> Result<StatusTransition, RepositoryError>;

    /// Applies a status change and inserts the open deletion request in one
    /// transaction (BR5.3).
    ///
    /// # Errors
    /// Returns [`RepositoryError::Conflict`] when another open request exists.
    async fn open_deletion(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, RepositoryError>;

    /// Applies a status change and records the cancellation in one transaction
    /// (BR5.4).
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the request cannot be updated.
    async fn cancel_deletion(
        &self,
        change: &StatusChange,
        request: &DeletionRequest,
    ) -> Result<StatusTransition, RepositoryError>;

    /// Reads the newest transition for one user, the evidence of its current
    /// status.
    ///
    /// # Errors
    /// Returns [`RepositoryError`] when the store cannot answer.
    async fn newest_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<StatusTransition>, RepositoryError>;
}

/// A persistence or external failure surfaced without leaking driver types
/// inward. The variants let the HTTP boundary choose 409/503 without parsing an
/// error string.
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

    /// A stored value is inconsistent with the presented one; the action fails
    /// closed rather than returning a record the caller does not own.
    #[error("refused: {0}")]
    Validation(String),
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

/// One active user and the timezone its reporting period is measured in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DueCandidate {
    /// The user identifier.
    pub user_id: UserId,
    /// The stored IANA timezone name.
    pub timezone: String,
}

/// Enumerates active users for the C3 reporting read.
#[async_trait]
pub trait DueUserRepository: Send + Sync {
    /// Lists one page of active, non-anonymised users ordered by id.
    async fn list_active(
        &self,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<DueCandidate>, RepositoryError>;
}

/// Reads and writes the internal user records.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Finds one user by internal id.
    async fn find_by_id(&self, id: UserId) -> Result<Option<User>, RepositoryError>;

    /// Finds the user whose encrypted Supabase subject matches `auth_subject`.
    /// A hash hit whose decrypted ciphertext names another subject is refused
    /// with [`RepositoryError::Validation`] (fail closed).
    async fn find_by_auth_subject(
        &self,
        auth_subject: &str,
    ) -> Result<Option<User>, RepositoryError>;

    /// Finds one user by the lower-case hex SHA-256 of the provider subject,
    /// for the C3 lookup. The caller already validated the subject.
    async fn find_by_subject_hash(
        &self,
        subject_hash: &str,
    ) -> Result<Option<User>, RepositoryError>;

    /// Finds one user by exact email address, for the admin lookup.
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, RepositoryError>;

    /// Creates the shredding-key reference, the user, and the protected subject
    /// in one transaction. Returns the key reference that now protects the user;
    /// a failure leaves no partial record.
    async fn provision(
        &self,
        user: &User,
        auth_subject: &str,
        now: DateTime<Utc>,
    ) -> Result<ShreddingKey, RepositoryError>;

    /// Inserts the user and its protected subject. The repository calls
    /// `identity.protect`; it never handles key material.
    async fn insert(
        &self,
        user: &User,
        auth_subject: &str,
        key: &ShreddingKey,
    ) -> Result<(), RepositoryError>;

    /// Persists a role, status, or preference change.
    async fn update(&self, user: &User) -> Result<(), RepositoryError>;

    /// Decrypts the stored subject through `identity.unprotect`, for the Auth
    /// admin delete and session revoke.
    async fn reveal_auth_subject(&self, user_id: UserId)
        -> Result<Option<String>, RepositoryError>;

    /// Anonymises the user's own fields and removes the encrypted subject.
    async fn anonymise(&self, user: &User) -> Result<(), RepositoryError>;
}

/// Why the jurisdiction gate did not clear a provider (C8).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GateRefusal {
    /// The declared or observed country is restricted.
    #[error("{0}")]
    Denied(String),
    /// The declaration or the observed address is malformed.
    #[error("{0}")]
    Invalid(String),
    /// The list, provider, or evidence store could not answer.
    #[error("{0}")]
    Unavailable(String),
}

/// Clears a provider for provisioning (C8).
///
/// The C8 HTTP endpoint remains for consumers in other services; the registry
/// calls this port in-process inside the Identity service.
#[async_trait]
pub trait JurisdictionGate: Send + Sync {
    /// Returns `Ok(())` only when both the self-declared country and the
    /// IP-resolved country are allowed.
    ///
    /// # Errors
    /// Returns [`GateRefusal`] when the provider is restricted, the declaration
    /// is malformed, or the decision cannot be made.
    async fn clear(
        &self,
        country_code: &str,
        ip: &str,
        correlation_id: &str,
    ) -> Result<(), GateRefusal>;
}

/// Reads and writes the per-subject shredding-key references.
#[async_trait]
pub trait ShreddingKeyRepository: Send + Sync {
    /// Creates a pgsodium key and stores its reference.
    async fn create_for_user(
        &self,
        user_id: UserId,
        now: DateTime<Utc>,
    ) -> Result<ShreddingKey, RepositoryError>;

    /// Finds the active key reference for one user.
    async fn active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<ShreddingKey>, RepositoryError>;

    /// Destroys the key material and records the terminal state.
    async fn destroy(
        &self,
        key: identity_domain::ShreddingKeyId,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
}

/// Reads and appends consent history.
#[async_trait]
pub trait ConsentRepository: Send + Sync {
    /// Appends one decision. Earlier records are never changed.
    async fn append(&self, record: &ConsentRecord) -> Result<(), RepositoryError>;

    /// Returns the newest record for one purpose.
    async fn latest(
        &self,
        user_id: UserId,
        purpose: ConsentPurpose,
    ) -> Result<Option<ConsentRecord>, RepositoryError>;

    /// Removes the user's consent history during anonymisation. Returns the
    /// number of records removed.
    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError>;
}

/// Removes a user's erasure-scoped rows through the dedicated erasure role.
///
/// The application connects as `identity_app`, which is append-only on consent
/// (BR4.4). The erasure job connects as `identity_retention`, so its deletes go
/// through this separate port and connection pool (FR17.3, NFR22.3).
#[async_trait]
pub trait ErasureStore: Send + Sync {
    /// Deletes the user's consent events, preferences, and archived events.
    /// Returns how many consent events were removed.
    async fn erase_consent(&self, user_id: UserId) -> Result<u64, RepositoryError>;
}

/// Reads and writes support-access grants.
#[async_trait]
pub trait SupportGrantRepository: Send + Sync {
    /// Inserts a requested grant.
    async fn insert(&self, grant: &SupportAccessGrant) -> Result<(), RepositoryError>;

    /// Finds one grant by identifier.
    async fn find_by_id(
        &self,
        id: identity_domain::SupportGrantId,
    ) -> Result<Option<SupportAccessGrant>, RepositoryError>;

    /// Persists a state change.
    async fn update(&self, grant: &SupportAccessGrant) -> Result<(), RepositoryError>;
}

/// Reads and writes the device-session view.
#[async_trait]
pub trait DeviceSessionRepository: Send + Sync {
    /// Inserts or updates the row for `(user_id, auth_session_id)`.
    async fn upsert(&self, session: &DeviceSession) -> Result<(), RepositoryError>;

    /// Lists the user's sessions, newest activity first.
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<DeviceSession>, RepositoryError>;

    /// Finds one session row.
    async fn find(
        &self,
        id: identity_domain::DeviceSessionId,
    ) -> Result<Option<DeviceSession>, RepositoryError>;

    /// Persists a revocation.
    async fn update(&self, session: &DeviceSession) -> Result<(), RepositoryError>;
}

/// Reads and writes export requests.
#[async_trait]
pub trait ExportRepository: Send + Sync {
    /// Inserts a requested export.
    async fn insert(&self, export: &ExportRequest) -> Result<(), RepositoryError>;

    /// Finds one export by identifier.
    async fn find_by_id(
        &self,
        id: identity_domain::ExportRequestId,
    ) -> Result<Option<ExportRequest>, RepositoryError>;

    /// Persists progress, completion, or failure.
    async fn update(&self, export: &ExportRequest) -> Result<(), RepositoryError>;

    /// Finds the user's current request, if one is pending, building, or ready.
    async fn find_active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<ExportRequest>, RepositoryError>;

    /// Atomically claims the oldest claimable request and sets its lease.
    async fn claim_next(
        &self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<Option<ExportRequest>, RepositoryError>;

    /// Lists ready requests whose download window has passed, for archive
    /// cleanup.
    async fn list_expired_ready(
        &self,
        now: DateTime<Utc>,
    ) -> Result<Vec<ExportRequest>, RepositoryError>;
}

/// Reads and writes deletion requests.
#[async_trait]
pub trait DeletionRepository: Send + Sync {
    /// Inserts a requested deletion.
    async fn insert(&self, deletion: &DeletionRequest) -> Result<(), RepositoryError>;

    /// Finds one deletion by identifier.
    async fn find_by_id(
        &self,
        id: identity_domain::DeletionRequestId,
    ) -> Result<Option<DeletionRequest>, RepositoryError>;

    /// Persists progress, the window, completion, or the retry bookkeeping.
    async fn update(&self, deletion: &DeletionRequest) -> Result<(), RepositoryError>;

    /// Finds the user's open request (neither cancelled nor erased), if any.
    async fn find_active_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Option<DeletionRequest>, RepositoryError>;

    /// Atomically claims the oldest due request: open, past its recovery
    /// window, due for a retry, and not under a live lease (W4).
    async fn claim_next(
        &self,
        worker: &str,
        now: DateTime<Utc>,
        lease: Duration,
    ) -> Result<Option<DeletionRequest>, RepositoryError>;
}

/// Reads the jurisdiction configuration.
#[async_trait]
pub trait JurisdictionRepository: Send + Sync {
    /// Finds one rule, or `None` when the country is not configured.
    async fn find_by_code(&self, code: &str) -> Result<Option<JurisdictionRule>, RepositoryError>;
}

/// Appends security and privacy events through C10.
#[async_trait]
pub trait AuditClient: Send + Sync {
    /// Appends one event; the caller decides whether a failure refuses the
    /// action (support reads, privacy events) or is only logged.
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError>;
}

/// Revokes sessions and deletes the Supabase Auth identity.
#[async_trait]
pub trait AuthAdmin: Send + Sync {
    /// Ends one Supabase Auth session.
    async fn revoke_session(
        &self,
        auth_subject: &str,
        session_id: &str,
    ) -> Result<(), ExternalError>;

    /// Deletes the Auth identity, its password, factors, and recovery codes.
    async fn delete_user(&self, auth_subject: &str) -> Result<(), ExternalError>;
}

/// Fetches one contributing service's export data and sends its anonymise call.
#[async_trait]
pub trait ContributorGateway: Send + Sync {
    /// Returns the contributor's user data as JSON.
    async fn export(
        &self,
        contributor: Contributor,
        user_id: UserId,
    ) -> Result<serde_json::Value, ExternalError>;

    /// Anonymises the contributor's personal fields and returns the categories it
    /// keeps under a legal basis. The idempotency key makes a retry apply once.
    async fn anonymise(
        &self,
        contributor: Contributor,
        user_id: UserId,
        idempotency_key: &str,
    ) -> Result<Vec<RetainedCategory>, ExternalError>;
}

/// Reads and writes the private export archive.
#[async_trait]
pub trait ExportStore: Send + Sync {
    /// Stores the archive under `key`.
    async fn put(&self, key: &str, body: Vec<u8>) -> Result<(), ExternalError>;

    /// Returns an owner-only signed URL that expires after `valid_for`.
    async fn signed_url(&self, key: &str, valid_for: Duration) -> Result<String, ExternalError>;

    /// Deletes the archive object.
    async fn delete(&self, key: &str) -> Result<(), ExternalError>;
}

/// Wakes the privacy worker after a request commits.
#[async_trait]
pub trait WakeTransport: Send + Sync {
    /// Sends the wake call. A failure degrades to the periodic sweep.
    async fn wake(&self) -> Result<(), String>;
}

/// A verified pre-sign-up gate grant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateGrant {
    /// The jurisdiction the visitor declared.
    pub jurisdiction: String,
    /// When the grant was issued.
    pub issued_at: DateTime<Utc>,
    /// When the grant expires.
    pub expires_at: DateTime<Utc>,
}

/// Signs and verifies the short-lived pre-sign-up gate token.
pub trait GateTokenIssuer: Send + Sync {
    /// Issues a signed, expiring gate token.
    ///
    /// # Errors
    /// Returns a human-readable reason when signing is misconfigured.
    fn issue(&self, jurisdiction: &str, now: DateTime<Utc>) -> Result<String, String>;

    /// Verifies a token's signature and expiry.
    ///
    /// # Errors
    /// Returns a human-readable reason for a forged or expired token.
    fn verify(&self, token: &str, now: DateTime<Utc>) -> Result<GateGrant, String>;
}

/// An injectable clock so tests control time.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Validates a Supabase JWT against JWKS (signature, issuer, audience, expiry).
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the facts Identity derives from it.
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

/// Resolves or issues internal service tokens.
pub trait ServiceTokenRegistry: Send + Sync {
    /// Returns the service a presented token was issued to, or `None` when the
    /// token is unknown.
    fn service_for(&self, token: &str) -> Option<String>;

    /// Returns the token issued to one service, for outbound calls.
    fn token_for(&self, service: &str) -> Option<String>;
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
