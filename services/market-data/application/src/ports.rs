//! Ports defined inward and implemented outward by `market-data-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use market_data_domain::{
    AssetPriceTier, AuditRecord, ErrorCategory, FxRate, ObservationId, Operation, PriceObservation,
    PriceTier, RefreshJob, RefreshRun, RefreshRunId,
};

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

/// An injectable clock so freshness, lease, and circuit-breaker tests are
/// deterministic.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// The bounded pause a retry uses. Production sleeps; tests record the pause.
#[async_trait]
pub trait Sleeper: Send + Sync {
    /// Waits for `duration`.
    async fn sleep(&self, duration: Duration);
}

/// One enabled price source as C4 reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PriceSource {
    /// The unique source name.
    pub name: String,
    /// Lower is tried first.
    pub priority: u32,
    /// The source's quote currency.
    pub quote_currency: String,
    /// The source's rate limit per minute.
    pub rate_limit_per_minute: u32,
    /// Whether the feed is delayed.
    pub delayed_feed: bool,
    /// Whether the source is enabled.
    pub enabled: bool,
}

impl PriceSource {
    /// Validates and builds a source.
    ///
    /// # Errors
    /// Returns a message when the name is blank, the priority is zero, the
    /// currency is not three letters, or the rate limit is zero.
    pub fn new(
        name: &str,
        priority: u32,
        quote_currency: &str,
        rate_limit_per_minute: u32,
        delayed_feed: bool,
        enabled: bool,
    ) -> Result<Self, String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 80 {
            return Err("the source name is blank or too long".to_string());
        }
        if priority == 0 {
            return Err(format!("source '{name}' has priority zero"));
        }
        let currency = market_data_domain::normalise_currency(quote_currency)
            .map_err(|_| format!("source '{name}' has a malformed quote currency"))?;
        if rate_limit_per_minute == 0 {
            return Err(format!("source '{name}' has a zero rate limit"));
        }
        Ok(Self {
            name: name.to_string(),
            priority,
            quote_currency: currency,
            rate_limit_per_minute,
            delayed_feed,
            enabled,
        })
    }
}

/// One canonical asset as C4 reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAsset {
    /// The canonical asset id.
    pub id: String,
    /// The canonical rank, from one.
    pub rank: u32,
}

/// Reads the Asset Registry's source registry and canonical asset list (C4).
#[async_trait]
pub trait AssetRegistry: Send + Sync {
    /// Every price source, enabled or not, as C4 reports it. The read path calls
    /// this once per request with a 150 ms budget and no retry.
    async fn sources(&self) -> Result<Vec<PriceSource>, ExternalError>;

    /// The canonical asset list ordered by rank. A refresh run calls this with
    /// the run budget and bounded retries.
    async fn canonical_assets(&self) -> Result<Vec<CanonicalAsset>, ExternalError>;
}

/// The newest accepted observation per asset for one source.
#[async_trait]
pub trait ObservationRepository: Send + Sync {
    /// Inserts an accepted or rejected observation.
    async fn insert(&self, observation: &PriceObservation) -> Result<(), RepositoryError>;

    /// The newest accepted observation per requested asset for one source
    /// (`DISTINCT ON (asset_id) ... ORDER BY asset_id, observed_at DESC`).
    async fn latest_accepted_for_source(
        &self,
        source_name: &str,
        asset_ids: &[String],
    ) -> Result<Vec<PriceObservation>, RepositoryError>;

    /// The newest accepted observation per requested asset across every source.
    /// This is the degraded C4 path (NFR1.11).
    async fn latest_accepted_for_assets(
        &self,
        asset_ids: &[String],
    ) -> Result<Vec<PriceObservation>, RepositoryError>;

    /// Deletes observations older than `cutoff`, keeping the newest row per
    /// `(asset_id, source_name)` (BR5.3). Returns the number removed.
    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError>;
}

/// Reads and writes FX readings.
#[async_trait]
pub trait FxRepository: Send + Sync {
    /// Inserts a reading.
    async fn insert(&self, rate: &FxRate) -> Result<(), RepositoryError>;

    /// The newest reading for the pair, regardless of age.
    async fn latest(&self, base: &str, quote: &str) -> Result<Option<FxRate>, RepositoryError>;

    /// Deletes readings older than `cutoff`, keeping the newest row per
    /// `(base, quote)` (BR5.3). Returns the number removed.
    async fn prune_before(&self, cutoff: DateTime<Utc>) -> Result<u64, RepositoryError>;
}

/// What a claim attempt produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimOutcome {
    /// The run now owns the job's lease.
    Claimed,
    /// Another run of the same job holds a live lease.
    Held(RefreshRun),
}

/// Claims, renews, finishes, and sweeps refresh runs.
#[async_trait]
pub trait RefreshRunRepository: Send + Sync {
    /// Claims a running run. An expired lease is failed first, in the same
    /// transaction, so it never blocks a later run (BR4.1).
    async fn claim(&self, run: &RefreshRun) -> Result<ClaimOutcome, RepositoryError>;

    /// Finds one run by identifier.
    async fn find_by_id(&self, id: RefreshRunId) -> Result<Option<RefreshRun>, RepositoryError>;

    /// The job's live running run, if any.
    async fn active_for_job(
        &self,
        job: RefreshJob,
        now: DateTime<Utc>,
    ) -> Result<Option<RefreshRun>, RepositoryError>;

    /// Extends a live lease. Returns `false` when the run no longer owns it.
    async fn renew(
        &self,
        id: RefreshRunId,
        lease_until: DateTime<Utc>,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;

    /// Persists a terminal run state with its counts. A run that is not running
    /// is left alone.
    async fn finish(&self, run: &RefreshRun) -> Result<(), RepositoryError>;

    /// Marks every running run with an expired lease failed and returns it.
    async fn sweep_expired(&self, now: DateTime<Utc>) -> Result<Vec<RefreshRun>, RepositoryError>;
}

/// Reads and writes per-asset freshness tiers.
#[async_trait]
pub trait TierRepository: Send + Sync {
    /// Inserts or refreshes a tier row.
    async fn upsert(&self, tier: &AssetPriceTier) -> Result<(), RepositoryError>;

    /// Finds one tier row.
    async fn find(&self, asset_id: &str) -> Result<Option<AssetPriceTier>, RepositoryError>;

    /// The bound for each requested asset. An asset with no row is absent; the
    /// caller treats it as `tail`.
    async fn bounds_for(&self, asset_ids: &[String])
        -> Result<Vec<(String, i64)>, RepositoryError>;
}

/// One reading a provider returned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderQuote {
    /// The price or rate as the provider sent it, still to be parsed.
    pub price: String,
    /// The provider's quote currency.
    pub currency: String,
    /// The provider's own timestamp.
    pub observed_at: DateTime<Utc>,
}

/// A classified provider failure (BR4.2).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{message}")]
pub struct ProviderError {
    /// The failure class.
    pub category: ErrorCategory,
    /// Whether a retry may help (timeout, 5xx, 429).
    pub retryable: bool,
    /// A non-secret summary.
    pub message: String,
}

impl ProviderError {
    /// A transient failure worth retrying.
    #[must_use]
    pub fn transient(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            retryable: true,
            message: message.into(),
        }
    }

    /// A permanent failure not worth retrying.
    #[must_use]
    pub fn permanent(category: ErrorCategory, message: impl Into<String>) -> Self {
        Self {
            category,
            retryable: false,
            message: message.into(),
        }
    }
}

/// Calls the configured price sources. No provider is ever called on the read
/// path (NFR2.1).
#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Reads one asset price from one source.
    async fn quote(
        &self,
        source: &PriceSource,
        asset_id: &str,
        api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError>;

    /// Reads one FX pair from one source.
    async fn fx_rate(
        &self,
        source: &PriceSource,
        base: &str,
        quote: &str,
        api_key: Option<&str>,
    ) -> Result<ProviderQuote, ProviderError>;
}

/// Reads provider keys from Vault through the `SECURITY DEFINER` function.
#[async_trait]
pub trait ProviderSecrets: Send + Sync {
    /// The key for a source, when one is configured.
    async fn provider_key(&self, source: &str) -> Result<Option<String>, ExternalError>;
}

/// The shared Postgres token bucket (BR4.3).
#[async_trait]
pub trait RateLimiter: Send + Sync {
    /// Refills and takes one token for the source. `false` means the source's
    /// limit would be exceeded and the work is deferred.
    async fn try_acquire(
        &self,
        source_name: &str,
        rate_limit_per_minute: u32,
        now: DateTime<Utc>,
    ) -> Result<bool, RepositoryError>;
}

/// Appends C10 events.
#[async_trait]
pub trait AuditClient: Send + Sync {
    /// Appends one event. A failure is logged and surfaced, never dropped.
    async fn append(
        &self,
        record: &AuditRecord,
        correlation_id: &str,
    ) -> Result<(), RepositoryError>;
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

/// The metrics the application emits. The HTTP crate's registry implements it;
/// tests use a recording fake.
pub trait ServiceMetrics: Send + Sync {
    /// One served quote's freshness, tier, and age.
    fn record_quote(
        &self,
        freshness: market_data_domain::Freshness,
        tier: PriceTier,
        age_seconds: i64,
    );
    /// One served FX reading's age.
    fn record_fx(&self, pair: &str, age_seconds: i64);
    /// One classified source failure.
    fn record_source_error(&self, source: &str, category: ErrorCategory);
    /// One finished run.
    fn record_run(&self, job: RefreshJob, status: &str);
    /// The unpriced assets a run left.
    fn record_unpriced(&self, job: RefreshJob, count: u32);
    /// A failed audit write.
    fn record_audit_failure(&self);
}

/// A metrics sink that records nothing.
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopMetrics;

impl ServiceMetrics for NoopMetrics {
    fn record_quote(
        &self,
        _freshness: market_data_domain::Freshness,
        _tier: PriceTier,
        _age_seconds: i64,
    ) {
    }

    fn record_fx(&self, _pair: &str, _age_seconds: i64) {}

    fn record_source_error(&self, _source: &str, _category: ErrorCategory) {}

    fn record_run(&self, _job: RefreshJob, _status: &str) {}

    fn record_unpriced(&self, _job: RefreshJob, _count: u32) {}

    fn record_audit_failure(&self) {}
}

/// The observation id helper keeps the port surface free of UUID details.
#[must_use]
pub fn new_observation_id() -> ObservationId {
    ObservationId::generate()
}
