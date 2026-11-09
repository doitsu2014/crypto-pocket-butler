//! Ports defined inward and implemented outward by `portfolio-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde_json::Value;
use uuid::Uuid;

use portfolio_domain::{
    AuditRecord, FxRate, Guardrail, Operation, OrderIntent, OrderIntentId, OrderSide, Portfolio,
    PortfolioAccount, PortfolioId, PriceSource, Quote, RecordedTrade, RecordedTradeId,
    TargetAllocation, TradeOverlay, UserId, ValuationSnapshot,
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

/// Reads and writes portfolios.
#[async_trait]
pub trait PortfolioRepository: Send + Sync {
    /// Inserts a portfolio.
    async fn insert(&self, portfolio: &Portfolio) -> Result<(), RepositoryError>;

    /// Finds one portfolio by identifier.
    async fn find_by_id(&self, id: PortfolioId) -> Result<Option<Portfolio>, RepositoryError>;

    /// Lists the user's portfolios, oldest first.
    async fn find_for_user(&self, user_id: UserId) -> Result<Vec<Portfolio>, RepositoryError>;

    /// Persists a name or timestamp change.
    async fn update(&self, portfolio: &Portfolio) -> Result<(), RepositoryError>;

    /// Anonymises the user's portfolio subject and name. The `user_id` is
    /// replaced by a random opaque reference so a later export by the original
    /// id returns nothing; the instant is stamped. Returns the number of rows
    /// changed.
    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
        at: DateTime<Utc>,
    ) -> Result<u64, RepositoryError>;

    /// Lists one page of portfolios for the scheduled run.
    async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<Portfolio>, RepositoryError>;
}

/// Reads and writes account membership.
#[async_trait]
pub trait MembershipRepository: Send + Sync {
    /// Assigns the account to the portfolio, moving it from any other portfolio
    /// (BR1.3).
    async fn assign(&self, membership: &PortfolioAccount) -> Result<(), RepositoryError>;

    /// Lists the portfolio's accounts.
    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError>;

    /// Lists every membership of the user's portfolios.
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<PortfolioAccount>, RepositoryError>;
}

/// Reads and writes targets.
#[async_trait]
pub trait TargetRepository: Send + Sync {
    /// Lists the portfolio's targets, sorted by asset.
    async fn list_targets(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<TargetAllocation>, RepositoryError>;

    /// Lists the portfolio's guardrails, sorted by asset.
    async fn list_guardrails(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<Guardrail>, RepositoryError>;

    /// Replaces the whole saved set in one transaction.
    async fn replace_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
        targets: &[TargetAllocation],
        guardrails: &[Guardrail],
    ) -> Result<(), RepositoryError>;
}

/// Persists guardrail crossing state.
#[async_trait]
pub trait GuardrailRepository: Send + Sync {
    /// Persists the crossing state.
    async fn update(&self, guardrail: &Guardrail) -> Result<(), RepositoryError>;
}

/// Reads and writes immutable snapshots.
#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Inserts a snapshot. A duplicate `(portfolio_id, hour_bucket)` returns
    /// [`RepositoryError::Conflict`].
    async fn insert(&self, snapshot: &ValuationSnapshot) -> Result<(), RepositoryError>;

    /// The newest snapshot for a portfolio.
    async fn latest(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError>;

    /// Lists a portfolio's snapshots at or after `from`, newest first, using a
    /// `(as_of)` cursor and a bounded page.
    async fn list(
        &self,
        portfolio_id: PortfolioId,
        from: Option<DateTime<Utc>>,
        limit: i64,
        cursor: Option<DateTime<Utc>>,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError>;

    /// Finds one snapshot for an on-demand reproduction.
    async fn find_by_id(
        &self,
        id: portfolio_domain::ValuationSnapshotId,
    ) -> Result<Option<ValuationSnapshot>, RepositoryError>;

    /// Whether the portfolio already has a snapshot in the hour bucket.
    async fn exists_for_hour(
        &self,
        portfolio_id: PortfolioId,
        hour_bucket: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    /// Lists every snapshot of the user's portfolios, for privacy export.
    async fn list_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<ValuationSnapshot>, RepositoryError>;
}

/// Reads and writes trade overlays.
#[async_trait]
pub trait OverlayRepository: Send + Sync {
    /// Inserts an overlay.
    async fn insert(&self, overlay: &TradeOverlay) -> Result<(), RepositoryError>;

    /// Lists the portfolio's overlays.
    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<TradeOverlay>, RepositoryError>;

    /// Removes the overlay for a recorded trade once C5 reports it (BR6.1).
    /// Returns the number removed.
    async fn delete_for_recorded_trade(
        &self,
        recorded_trade_id: RecordedTradeId,
    ) -> Result<u64, RepositoryError>;

    /// Expires overlays that are older than the cutoff because no venue sync
    /// ever reported them. Returns the number removed.
    async fn expire_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError>;

    /// Lists every overlay of the user's portfolios, for privacy export.
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<TradeOverlay>, RepositoryError>;
}

/// Reads and writes order intents.
#[async_trait]
pub trait IntentRepository: Send + Sync {
    /// Inserts a pending intent.
    async fn insert(&self, intent: &OrderIntent) -> Result<(), RepositoryError>;

    /// Finds one intent.
    async fn find_by_id(&self, id: OrderIntentId) -> Result<Option<OrderIntent>, RepositoryError>;

    /// Lists the portfolio's intents, newest first.
    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<OrderIntent>, RepositoryError>;

    /// Persists a status change.
    async fn update(&self, intent: &OrderIntent) -> Result<(), RepositoryError>;

    /// Lists every intent of the user's portfolios, for privacy export.
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<OrderIntent>, RepositoryError>;
}

/// Reads and writes recorded trades.
#[async_trait]
pub trait TradeRepository: Send + Sync {
    /// Inserts the recorded trade and its overlay in one transaction.
    async fn insert_with_overlay(
        &self,
        trade: &RecordedTrade,
        overlay: &TradeOverlay,
    ) -> Result<(), RepositoryError>;

    /// Finds a recorded trade by its client idempotency key (BR6.1 replay).
    async fn find_by_idempotency_key(
        &self,
        portfolio_id: PortfolioId,
        idempotency_key: &str,
    ) -> Result<Option<RecordedTrade>, RepositoryError>;

    /// Finds a recorded trade by identifier.
    async fn find_by_id(
        &self,
        id: RecordedTradeId,
    ) -> Result<Option<RecordedTrade>, RepositoryError>;

    /// Lists the portfolio's recorded trades, newest first.
    async fn list_for_portfolio(
        &self,
        portfolio_id: PortfolioId,
    ) -> Result<Vec<RecordedTrade>, RepositoryError>;

    /// Lists every recorded trade of the user's portfolios, for privacy export.
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<RecordedTrade>, RepositoryError>;

    /// Replaces the user id on the user's recorded trades with the random opaque
    /// reference. Returns the number changed.
    async fn anonymise_for_user(
        &self,
        user_id: UserId,
        anonymised_ref: UserId,
    ) -> Result<u64, RepositoryError>;
}

/// A stored valuation run and its lease.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValuationRun {
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

/// The status of a valuation run.
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

/// Reads and writes the single-flight valuation run lease.
#[async_trait]
pub trait RunRepository: Send + Sync {
    /// Claims the job's lease. Returns `false` when a live lease already exists.
    async fn claim(&self, run: &ValuationRun) -> Result<bool, RepositoryError>;

    /// Extends a live lease. Returns `false` when the run no longer owns it.
    async fn renew(
        &self,
        id: Uuid,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    /// Persists a terminal state.
    async fn finish(&self, run: &ValuationRun) -> Result<(), RepositoryError>;

    /// Marks every running run with an expired lease failed and returns it.
    async fn sweep_expired(&self, now: DateTime<Utc>)
        -> Result<Vec<ValuationRun>, RepositoryError>;
}

/// One recorded privacy anonymisation, used to make a retry apply once. It holds
/// only the random opaque reference, never the original user id.
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

/// One holding observed by C5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoldingObservation {
    /// The account.
    pub account_id: portfolio_domain::AccountId,
    /// The canonical asset.
    pub asset_id: String,
    /// The quantity.
    pub quantity: Decimal,
    /// When the holding was observed.
    pub observed_at: DateTime<Utc>,
}

/// One venue transaction reported by C5.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueTradeObservation {
    /// The venue's row identifier.
    pub external_id: Option<String>,
    /// The canonical asset.
    pub asset_id: String,
    /// The side.
    pub side: OrderSide,
    /// The quantity.
    pub quantity: Decimal,
    /// When the transaction executed.
    pub executed_at: DateTime<Utc>,
}

/// Reads C5 holdings and venue transactions.
#[async_trait]
pub trait HoldingsClient: Send + Sync {
    /// Returns every holding the user's accounts report.
    async fn holdings_for_user(
        &self,
        user_id: UserId,
    ) -> Result<Vec<HoldingObservation>, ExternalError>;

    /// Returns the account's venue transactions since `since`.
    async fn transactions_for_account(
        &self,
        account_id: portfolio_domain::AccountId,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<VenueTradeObservation>, ExternalError>;
}

/// Reads C6 prices and FX.
#[async_trait]
pub trait MarketDataClient: Send + Sync {
    /// Returns quotes for the assets, with provenance and freshness.
    async fn prices(&self, asset_ids: &[String]) -> Result<Vec<Quote>, ExternalError>;

    /// Returns the rate for one pair.
    async fn fx(&self, base: &str, quote: &str) -> Result<FxRate, ExternalError>;
}

/// A trade command sent to Ledger C7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerTradeCommand {
    /// The asset.
    pub asset_id: String,
    /// The side.
    pub side: OrderSide,
    /// The quantity.
    pub quantity: Decimal,
    /// The unit price.
    pub price: Decimal,
    /// Where the price came from.
    pub price_source: PriceSource,
    /// The ISO currency the price is quoted in; absent means the base currency.
    pub currency: Option<String>,
    /// When the trade executed.
    pub executed_at: DateTime<Utc>,
}

/// Records a confirmed trade on Ledger C7.
#[async_trait]
pub trait LedgerClient: Send + Sync {
    /// Records the trade with a client idempotency key and returns the Ledger
    /// transaction id.
    async fn record_trade(
        &self,
        user_id: UserId,
        command: &LedgerTradeCommand,
        idempotency_key: &str,
    ) -> Result<String, ExternalError>;
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

/// Reads the user's base-currency preference from Identity.
#[async_trait]
pub trait UserPreferenceClient: Send + Sync {
    /// Returns the preference, or `None` when Identity has no answer.
    async fn base_currency(&self, user_id: UserId) -> Result<Option<String>, ExternalError>;
}

/// An injectable clock so lease and staleness tests are deterministic.
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
    /// Validates the token and returns the facts Portfolio derives from it.
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

/// Wakes the valuation worker after a write commits.
#[async_trait]
pub trait WakeTransport: Send + Sync {
    /// Sends the wake call.
    async fn wake(&self) -> Result<(), String>;
}
