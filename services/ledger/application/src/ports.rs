//! Ports defined inward and implemented outward by `ledger-infrastructure`.

use std::collections::BTreeMap;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ledger_domain::{
    Correction, CostBasisLot, IncomingTransaction, LedgerSubject, LedgerSubjectId, PnlSnapshot,
    Transaction, TransactionId, TransferPair,
};
use rust_decimal::Decimal;
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
    /// The dependency could not be reached or timed out.
    #[error("dependency unavailable: {0}")]
    Unavailable(String),
    /// The dependency refused the request.
    #[error("dependency refused the request: {0}")]
    Refused(String),
}

/// An injectable clock so freshness and expiry tests are deterministic.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// An injectable identifier source so tests are deterministic.
pub trait IdGenerator: Send + Sync {
    /// A fresh identifier.
    fn next(&self) -> Uuid;
}

/// An injectable pause so audit retry tests never sleep.
#[async_trait]
pub trait Pause: Send + Sync {
    /// Waits the configured duration.
    async fn pause_for(&self, duration: Duration);
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
    /// The logical target; never an amount or a credential.
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

/// A keyset cursor over `(executedAt, id)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionCursor {
    /// The last row's execution time.
    pub executed_at: DateTime<Utc>,
    /// The last row's identifier.
    pub id: TransactionId,
}

/// Reads and writes the append-only transaction log.
#[async_trait]
pub trait TransactionRepository: Send + Sync {
    /// Inserts one transaction; a canonical or external key conflict is a
    /// [`RepositoryError::Conflict`] (BR1.2).
    async fn insert(&self, transaction: &Transaction) -> Result<(), RepositoryError>;

    /// Finds one transaction.
    async fn find_by_id(&self, id: TransactionId) -> Result<Option<Transaction>, RepositoryError>;

    /// Whether a canonical key is already stored.
    async fn canonical_exists(&self, key: &str) -> Result<bool, RepositoryError>;

    /// Whether an external key is already stored.
    async fn external_exists(&self, key: &str) -> Result<bool, RepositoryError>;

    /// One keyset page, newest first (NFR2.2).
    async fn list_page(
        &self,
        subject: LedgerSubjectId,
        after: Option<TransactionCursor>,
        limit: u32,
    ) -> Result<Vec<Transaction>, RepositoryError>;

    /// Every transaction for a subject, in execution order, for replay.
    async fn list_all(&self, subject: LedgerSubjectId)
        -> Result<Vec<Transaction>, RepositoryError>;

    /// The latest transaction record time for a subject.
    async fn last_recorded_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError>;

    /// Every subject that owns a transaction.
    async fn list_subjects(&self) -> Result<Vec<LedgerSubjectId>, RepositoryError>;

    /// The transaction count for one subject, for reconcile batching.
    async fn count_for_subject(&self, subject: LedgerSubjectId) -> Result<u64, RepositoryError>;
}

/// Reads and writes the append-only correction log.
#[async_trait]
pub trait CorrectionRepository: Send + Sync {
    /// Inserts one correction.
    async fn insert(&self, correction: &Correction) -> Result<(), RepositoryError>;

    /// Corrections for one transaction, oldest first.
    async fn list_for_transaction(
        &self,
        transaction: TransactionId,
    ) -> Result<Vec<Correction>, RepositoryError>;

    /// Corrections for a subject, oldest first.
    async fn list_for_subject(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<Correction>, RepositoryError>;

    /// The latest correction time for a subject.
    async fn last_corrected_at(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError>;
}

/// Reads and writes the mutable subject rows.
#[async_trait]
pub trait SubjectRepository: Send + Sync {
    /// Finds the subject for a Supabase Auth user.
    async fn find_by_user(&self, user_id: &str) -> Result<Option<LedgerSubject>, RepositoryError>;

    /// Finds one subject by identifier.
    async fn find_by_id(
        &self,
        id: LedgerSubjectId,
    ) -> Result<Option<LedgerSubject>, RepositoryError>;

    /// Inserts a subject.
    async fn create(&self, subject: &LedgerSubject) -> Result<(), RepositoryError>;

    /// Every subject identifier, for reconcile.
    async fn list_ids(&self) -> Result<Vec<LedgerSubjectId>, RepositoryError>;
}

/// Replaces and reads the derived FIFO lots.
#[async_trait]
pub trait LotRepository: Send + Sync {
    /// Replaces every lot for a subject atomically after a rebuild.
    async fn replace_for_subject(
        &self,
        subject: LedgerSubjectId,
        lots: &[CostBasisLot],
    ) -> Result<(), RepositoryError>;

    /// The open lots for a subject.
    async fn list_open(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<CostBasisLot>, RepositoryError>;
}

/// Reads and writes the derived P&L snapshots.
#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Upserts the total snapshot for a subject and portfolio scope.
    async fn upsert(&self, snapshot: &PnlSnapshot) -> Result<(), RepositoryError>;

    /// The latest snapshot for a subject and portfolio scope.
    async fn find_latest(
        &self,
        subject: LedgerSubjectId,
        portfolio_id: Option<Uuid>,
    ) -> Result<Option<PnlSnapshot>, RepositoryError>;
}

/// Reads and writes transfer pairings.
#[async_trait]
pub trait TransferPairRepository: Send + Sync {
    /// Inserts a pending pair.
    async fn insert(&self, pair: &TransferPair) -> Result<(), RepositoryError>;

    /// Every pair for a subject.
    async fn list_for_subject(
        &self,
        subject: LedgerSubjectId,
    ) -> Result<Vec<TransferPair>, RepositoryError>;

    /// Marks an outgoing leg matched with its incoming leg.
    async fn mark_matched(
        &self,
        out_transaction: TransactionId,
        in_transaction: TransactionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    /// Marks a pending incoming leg matched when its outbound leg arrives.
    async fn match_pending_in(
        &self,
        in_transaction: TransactionId,
        out_transaction: TransactionId,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    /// Marks an outgoing leg rejected.
    async fn mark_rejected(&self, out_transaction: TransactionId) -> Result<(), RepositoryError>;
}

/// Reads Account's venue and imported transactions through C5, and the current
/// holdings that coverage is measured against.
#[async_trait]
pub trait AccountClient: Send + Sync {
    /// Every row for a user, including rows carrying a line error.
    async fn transactions_for_user(
        &self,
        user_id: &str,
    ) -> Result<Vec<IncomingTransaction>, ExternalError>;

    /// The user's current holdings across every account.
    async fn holdings_for_user(&self, user_id: &str) -> Result<Vec<Holding>, ExternalError>;
}

/// One held position from Account's C5 `getHoldings`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holding {
    /// The connected account.
    pub account_id: Uuid,
    /// The asset, absent for a spam token the amendment marks without an id.
    pub asset_id: Option<String>,
    /// The quantity held in that account.
    pub quantity: Decimal,
}

/// Resolves an asset through the Asset Registry (C4).
#[async_trait]
pub trait AssetResolver: Send + Sync {
    /// Whether the asset exists and is supported.
    async fn exists(&self, asset_id: &str) -> Result<bool, ExternalError>;
}

/// One C6 price observation.
#[derive(Debug, Clone, PartialEq)]
pub struct PriceQuote {
    /// The asset.
    pub asset_id: String,
    /// The unit price, absent when unpriced.
    pub price: Option<Decimal>,
    /// The currency the price is quoted in.
    pub currency: Option<String>,
    /// When the price was observed.
    pub as_of: Option<DateTime<Utc>>,
    /// `live`, `delayed`, `stale`, or `unpriced`.
    pub freshness: String,
}

/// One C6 FX observation.
#[derive(Debug, Clone, PartialEq)]
pub struct FxQuote {
    /// The base currency.
    pub base: String,
    /// The quote currency.
    pub quote: String,
    /// One unit of `quote` in `base`.
    pub rate: Decimal,
    /// When the rate was observed.
    pub as_of: Option<DateTime<Utc>>,
}

/// Reads current prices and FX from Market Data (C6).
#[async_trait]
pub trait MarketDataClient: Send + Sync {
    /// Prices for the requested assets.
    async fn prices(&self, asset_ids: &[String]) -> Result<Vec<PriceQuote>, ExternalError>;

    /// The rate from `quote` into `base`.
    async fn fx(&self, base: &str, quote: &str) -> Result<FxQuote, ExternalError>;
}

/// Applies the anonymisation under the `ledger_anonymiser` role. The adapter
/// opens a short-lived connection for this one operation only.
#[async_trait]
pub trait AnonymiserGateway: Send + Sync {
    /// Clears the user id and sets a non-reversible reference on the subject.
    async fn anonymise(
        &self,
        subject: LedgerSubjectId,
        anonymised_ref: &str,
    ) -> Result<(), RepositoryError>;
}

/// Records completed anonymisations by idempotency key, so a retry applies once
/// without keeping the user id.
#[async_trait]
pub trait AnonymisationRepository: Send + Sync {
    /// The anonymised reference recorded for a key, if the request completed.
    async fn find(&self, idempotency_key: &str) -> Result<Option<String>, RepositoryError>;

    /// Records one completed request.
    async fn record(
        &self,
        idempotency_key: &str,
        anonymised_ref: &str,
        at: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;
}

/// Holds and reads the reconcile lease so only one run rebuilds at a time.
#[async_trait]
pub trait ReconcileLease: Send + Sync {
    /// Tries to take the lease until `until`.
    async fn acquire(
        &self,
        now: DateTime<Utc>,
        until: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    /// Releases the lease.
    async fn release(&self, now: DateTime<Utc>) -> Result<(), RepositoryError>;

    /// When the last run completed, if ever.
    async fn last_completed(&self) -> Result<Option<DateTime<Utc>>, RepositoryError>;
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

/// A per-asset position used by the P&L view.
#[derive(Debug, Clone, PartialEq)]
pub struct Position {
    /// The asset.
    pub asset_id: String,
    /// The quantity still held.
    pub quantity: Decimal,
}
