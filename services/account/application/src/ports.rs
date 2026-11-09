//! Ports defined inward and implemented outward by `account-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value;

use account_domain::{
    Account, AccountId, AuditRecord, Credential, CredentialId, HoldingSnapshot, ImportBatch,
    ImportBatchId, ImportRowError, ImportedTransaction, Operation, SyncRun, SyncRunId, UserId,
    WalletConnection,
};

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

/// Reads and writes accounts.
#[async_trait]
pub trait AccountRepository: Send + Sync {
    /// Inserts a new account.
    async fn insert(&self, account: &Account) -> Result<(), RepositoryError>;

    /// Finds one account by identifier.
    async fn find_by_id(&self, id: AccountId) -> Result<Option<Account>, RepositoryError>;

    /// Lists the user's accounts, oldest first.
    async fn find_for_user(&self, user_id: UserId) -> Result<Vec<Account>, RepositoryError>;

    /// Whether another account of the user already uses the label, ignoring case
    /// and exclusing `except` (BR1.2).
    async fn label_exists(
        &self,
        user_id: UserId,
        label: &str,
        except: Option<AccountId>,
    ) -> Result<bool, RepositoryError>;

    /// Persists a status, label, or sync-time change.
    async fn update(&self, account: &Account) -> Result<(), RepositoryError>;

    /// Lists the accounts the worker may sync (excluding CSV and disconnected).
    async fn list_syncable(&self) -> Result<Vec<Account>, RepositoryError>;

    /// Removes the personal label and stamps `anonymisedAt` for every account of
    /// the user (BR8.2). Returns the number of rows changed.
    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError>;
}

/// Reads and writes exchange credentials.
#[async_trait]
pub trait CredentialRepository: Send + Sync {
    /// Inserts a credential.
    async fn insert(&self, credential: &Credential) -> Result<(), RepositoryError>;

    /// Finds one credential by identifier.
    async fn find_by_id(&self, id: CredentialId) -> Result<Option<Credential>, RepositoryError>;

    /// Finds the account's credential, when one exists.
    async fn find_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<Credential>, RepositoryError>;

    /// Persists a verification state or Vault reference change.
    async fn update(&self, credential: &Credential) -> Result<(), RepositoryError>;

    /// Lists every credential that is verified or stale, for periodic
    /// re-verification (WF7).
    async fn list_verified(&self) -> Result<Vec<Credential>, RepositoryError>;

    /// Lists the credentials of the user's accounts, for privacy export.
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Credential>, RepositoryError>;

    /// Removes every credential of the user's accounts after their Vault secrets
    /// are destroyed (BR8.2). Returns the number removed.
    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError>;
}

/// Reads and writes wallet connections.
#[async_trait]
pub trait WalletConnectionRepository: Send + Sync {
    /// Inserts a connection.
    async fn insert(&self, connection: &WalletConnection) -> Result<(), RepositoryError>;

    /// Finds the account's connection.
    async fn find_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<WalletConnection>, RepositoryError>;

    /// Lists the user's connections, for privacy export.
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<WalletConnection>, RepositoryError>;

    /// Removes the user's wallet addresses for privacy anonymisation (BR8.2).
    /// Returns the number removed.
    async fn delete_for_user(&self, user_id: UserId) -> Result<u64, RepositoryError>;
}

/// Reads and writes sync runs and performs the fenced snapshot write.
#[async_trait]
pub trait SyncRunRepository: Send + Sync {
    /// Claims a running run. Returns [`RepositoryError::Conflict`] when a run with
    /// a live lease already exists for the account (BR4.1).
    async fn claim(&self, run: &SyncRun) -> Result<(), RepositoryError>;

    /// Finds one run by identifier.
    async fn find_by_id(&self, id: SyncRunId) -> Result<Option<SyncRun>, RepositoryError>;

    /// Finds the account's currently running run, if any.
    async fn running_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Option<SyncRun>, RepositoryError>;

    /// Lists every running run whose lease is still live, for the worker.
    async fn list_running(&self, now: DateTime<Utc>) -> Result<Vec<SyncRun>, RepositoryError>;

    /// Extends a live lease. Returns `false` when the run no longer owns it.
    async fn renew(
        &self,
        id: SyncRunId,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    /// Persists a terminal run state. A run that is not running is left alone.
    async fn finish(&self, run: &SyncRun) -> Result<(), RepositoryError>;

    /// Writes the run's snapshots and the account's last-synced time in one
    /// transaction, fenced on `run.id = :run AND lease_until > now()` (NFR
    /// reliability). Returns [`RepositoryError::Conflict`] when the lease was
    /// superseded.
    async fn write_snapshot(
        &self,
        run: &SyncRun,
        holdings: &[HoldingSnapshot],
        last_synced_at: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<(), RepositoryError>;

    /// Marks every running run with an expired lease failed and returns it
    /// (BR4.2).
    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<SyncRun>, RepositoryError>;
}

/// Reads the holdings and imported transactions C5 serves.
#[async_trait]
pub trait HoldingRepository: Send + Sync {
    /// The latest successful run's snapshots for one account.
    async fn latest_for_account(
        &self,
        account_id: AccountId,
    ) -> Result<Vec<HoldingSnapshot>, RepositoryError>;

    /// Every snapshot of the user's accounts, for privacy export.
    async fn snapshots_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<HoldingSnapshot>, RepositoryError>;
}

/// Reads and writes CSV import batches, staged bytes, rows, and row errors.
#[async_trait]
pub trait ImportRepository: Send + Sync {
    /// Creates the batch and stages its bytes in one transaction.
    async fn create_batch(&self, batch: &ImportBatch, bytes: &[u8]) -> Result<(), RepositoryError>;

    /// Finds one batch.
    async fn find_batch(&self, id: ImportBatchId) -> Result<Option<ImportBatch>, RepositoryError>;

    /// Persists a batch status or count change.
    async fn update_batch(&self, batch: &ImportBatch) -> Result<(), RepositoryError>;

    /// Claims the oldest pending batch, marking it processing. Returns `None`
    /// when nothing is pending.
    async fn claim_pending(&self) -> Result<Option<ImportBatch>, RepositoryError>;

    /// Reads the staged bytes for a batch.
    async fn staged_bytes(&self, id: ImportBatchId) -> Result<Option<Vec<u8>>, RepositoryError>;

    /// Deletes the staging row once the batch completes (or after 90 days).
    async fn delete_staging(&self, id: ImportBatchId) -> Result<(), RepositoryError>;

    /// Whether the account already holds this dedupe key (BR5.2).
    async fn dedupe_exists(
        &self,
        account_id: AccountId,
        dedupe_key: &str,
    ) -> Result<bool, RepositoryError>;

    /// Inserts rows, skipping keys that already exist. Returns the number
    /// actually inserted.
    async fn insert_transactions(
        &self,
        rows: &[ImportedTransaction],
    ) -> Result<u64, RepositoryError>;

    /// Inserts the per-line errors.
    async fn insert_row_errors(&self, errors: &[ImportRowError]) -> Result<(), RepositoryError>;

    /// The account's imported transactions since `since`, oldest first.
    async fn transactions_for_account(
        &self,
        account_id: AccountId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<ImportedTransaction>, RepositoryError>;

    /// Every imported transaction of the user's accounts, for privacy export.
    async fn transactions_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ImportedTransaction>, RepositoryError>;
}

/// One recorded privacy anonymisation, used to make a retry apply once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnonymisationRecord {
    /// The user.
    pub user_id: UserId,
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
    /// Finds a completed anonymisation for the user.
    async fn find_anonymisation(
        &self,
        user_id: UserId,
    ) -> Result<Option<AnonymisationRecord>, RepositoryError>;

    /// Records a completed anonymisation.
    async fn record_anonymisation(
        &self,
        record: &AnonymisationRecord,
    ) -> Result<(), RepositoryError>;
}

/// Verifies a read-only exchange key and reads its balances.
#[async_trait]
pub trait ExchangeProvider: Send + Sync {
    /// Performs the read-only balances call and the permissions check (BR2.1).
    async fn verify_read_only(
        &self,
        venue: &str,
        credentials: &ExchangeCredentials,
    ) -> Result<VerifiedPermissions, ExternalError>;

    /// Reads the venue's balances.
    async fn balances(
        &self,
        venue: &str,
        credentials: &ExchangeCredentials,
    ) -> Result<Vec<ObservedBalance>, ExternalError>;
}

/// The key material for one exchange credential, held in memory only for the
/// provider call.
#[derive(Clone)]
pub struct ExchangeCredentials {
    /// The public key identifier.
    pub api_key: String,
    /// The private secret.
    pub api_secret: String,
    /// The OKX passphrase, when the venue requires one.
    pub passphrase: Option<String>,
}

impl ExchangeCredentials {
    /// Creates a key pair.
    #[must_use]
    pub fn new(api_key: impl Into<String>, api_secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            passphrase: None,
        }
    }

    /// Adds the OKX passphrase.
    #[must_use]
    pub fn with_passphrase(mut self, passphrase: impl Into<String>) -> Self {
        self.passphrase = Some(passphrase.into());
        self
    }
}

impl std::fmt::Debug for ExchangeCredentials {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ExchangeCredentials")
            .field("api_key", &"[redacted]")
            .field("api_secret", &"[redacted]")
            .field(
                "passphrase",
                &self.passphrase.as_ref().map(|_| "[redacted]"),
            )
            .finish()
    }
}

/// The permissions a verification found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerifiedPermissions {
    /// Whether no withdrawal or trading right exists.
    pub read_only: bool,
}

/// One balance as a venue or chain reported it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedBalance {
    /// The token identity observed at the source.
    pub observed_token: String,
    /// The decimal quantity.
    pub quantity: rust_decimal::Decimal,
}

/// Reads a wallet's balances from its chain.
#[async_trait]
pub trait ChainProvider: Send + Sync {
    /// Reads the chain's balances for a public address.
    async fn balances(
        &self,
        chain: &str,
        address: &str,
    ) -> Result<Vec<ObservedBalance>, ExternalError>;
}

/// One token observation to resolve through the Asset Registry (C4).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    /// The chain the token was observed on, when one applies.
    pub chain: Option<String>,
    /// The contract address, when the token is a contract token.
    pub contract_address: Option<String>,
    /// The venue symbol, when no contract address exists.
    pub symbol: Option<String>,
}

/// Resolves observed tokens and reads the supported chains.
#[async_trait]
pub trait AssetRegistry: Send + Sync {
    /// Returns the canonical asset id, or `None` when the asset is unregistered
    /// (a spam holding, BR6.3).
    async fn resolve(&self, observation: &Observation) -> Result<Option<String>, ExternalError>;

    /// Returns the supported EVM chains and Solana (C4 supported-chains read,
    /// BR3.1).
    async fn supported_chains(&self) -> Result<Vec<SupportedChain>, ExternalError>;
}

/// One chain the Asset Registry supports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportedChain {
    /// The chain identifier (`evm:<chainId>` or `solana`).
    pub id: String,
    /// The native symbol.
    pub native_symbol: String,
}

/// Stores, reveals, and destroys Vault secret references.
#[async_trait]
pub trait VaultClient: Send + Sync {
    /// Stores a secret and returns its Vault reference.
    async fn store(&self, owner: &str, secret: &str) -> Result<String, ExternalError>;

    /// Reveals a secret into process memory for one provider call.
    async fn reveal(&self, reference: &str) -> Result<String, ExternalError>;

    /// Destroys a secret. A rejected or removed key is never retained.
    async fn destroy(&self, reference: &str) -> Result<(), ExternalError>;
}

/// Maintains the `has_exchange_credentials` claim through C15.
#[async_trait]
pub trait AuthAdmin: Send + Sync {
    /// Sets or clears `app_metadata.has_exchange_credentials`.
    async fn set_has_exchange_credentials(
        &self,
        user_id: UserId,
        value: bool,
    ) -> Result<(), ExternalError>;
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

/// Sends C9 alerts.
#[async_trait]
pub trait NotificationClient: Send + Sync {
    /// Sends an alert deduplicated by `idempotency_key` (BR7.1).
    async fn notify(
        &self,
        user_id: UserId,
        alert_type: &str,
        idempotency_key: &str,
        payload: Value,
    ) -> Result<(), ExternalError>;
}

/// Wakes the sync worker after a write commits. A failure degrades to the
/// periodic sweep.
#[async_trait]
pub trait WakeTransport: Send + Sync {
    /// Sends the wake call.
    async fn wake(&self) -> Result<(), String>;
}

/// An injectable clock so lease and staleness tests are deterministic.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// Validates a Supabase JWT against JWKS (signature, issuer, audience, expiry).
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the facts Account derives from it.
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
    pub operations: Vec<Operation>,
}

impl RegisteredService {
    /// Whether the caller may call the operation.
    #[must_use]
    pub fn allows(&self, operation: Operation) -> bool {
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
