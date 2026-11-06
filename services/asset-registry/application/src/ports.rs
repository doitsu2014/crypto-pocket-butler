//! Ports defined inward and implemented outward by `asset-registry-infrastructure`.
//!
//! Use cases depend on these traits only; no sqlx, Axum, or reqwest type crosses
//! this boundary.

use asset_registry_domain::{
    Asset, AssetId, Chain, ChainFamily, ChainId, ContractAddress, ObservationId, ObservationStatus,
    Price, PriceSource, TokenContract, UnmappedObservation,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

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
}

/// Reads and writes canonical assets and their chain representations.
#[async_trait]
pub trait AssetRepository: Send + Sync {
    /// Finds the canonical asset for a chain-plus-contract observation (BR1.1).
    async fn find_asset_by_contract(
        &self,
        chain: &ChainId,
        family: ChainFamily,
        address: &ContractAddress,
    ) -> Result<Option<Asset>, RepositoryError>;

    /// Finds a native asset by chain plus symbol (BR1.1).
    async fn find_native_asset(
        &self,
        chain: &ChainId,
        symbol: &str,
    ) -> Result<Option<Asset>, RepositoryError>;

    /// Finds an asset by identifier.
    async fn find_asset_by_id(&self, id: &AssetId) -> Result<Option<Asset>, RepositoryError>;

    /// Inserts an asset, optionally recording it as native to one chain.
    async fn insert_asset(
        &self,
        asset: &Asset,
        native_chain: Option<&ChainId>,
    ) -> Result<(), RepositoryError>;

    /// Inserts a chain-plus-contract mapping.
    async fn insert_token_contract(&self, contract: &TokenContract) -> Result<(), RepositoryError>;
}

/// Reads and writes chains.
#[async_trait]
pub trait ChainRepository: Send + Sync {
    /// Finds a chain by identifier.
    async fn find_by_id(&self, id: &ChainId) -> Result<Option<Chain>, RepositoryError>;

    /// Lists every configured chain.
    async fn list(&self) -> Result<Vec<Chain>, RepositoryError>;

    /// Inserts or updates a chain.
    async fn upsert(&self, chain: &Chain) -> Result<(), RepositoryError>;
}

/// Reads and writes the price-source priority list (BR2.1).
#[async_trait]
pub trait PriceSourceRepository: Send + Sync {
    /// Lists enabled sources in priority order, lowest first.
    async fn list_enabled_by_priority(&self) -> Result<Vec<PriceSource>, RepositoryError>;

    /// Lists every source.
    async fn list(&self) -> Result<Vec<PriceSource>, RepositoryError>;

    /// Finds a source by its unique name.
    async fn find_by_name(&self, name: &str) -> Result<Option<PriceSource>, RepositoryError>;

    /// Inserts or updates a source by name.
    async fn upsert(&self, source: &PriceSource) -> Result<(), RepositoryError>;

    /// Enables or disables a source by name.
    async fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), RepositoryError>;

    /// Whether another enabled source already holds this priority.
    async fn enabled_priority_exists(
        &self,
        priority: u32,
        except_name: Option<&str>,
    ) -> Result<bool, RepositoryError>;
}

/// Reads and writes the quarantine (BR1.3).
#[async_trait]
pub trait ObservationRepository: Send + Sync {
    /// Finds an observation by its chain-plus-token identity.
    async fn find_by_token(
        &self,
        chain: &ChainId,
        observed_symbol: &str,
        contract_key: Option<&str>,
    ) -> Result<Option<UnmappedObservation>, RepositoryError>;

    /// Finds an observation by identifier.
    async fn find_by_id(
        &self,
        id: &ObservationId,
    ) -> Result<Option<UnmappedObservation>, RepositoryError>;

    /// Inserts a quarantined observation. `contract_key` is the chain-family
    /// normalized address used for deduplication (see `ContractAddress::key_for`).
    async fn insert(
        &self,
        observation: &UnmappedObservation,
        contract_key: Option<&str>,
    ) -> Result<(), RepositoryError>;

    /// Persists a status change.
    async fn save_status(&self, observation: &UnmappedObservation) -> Result<(), RepositoryError>;

    /// Lists observations in one state, oldest first.
    async fn list_by_status(
        &self,
        status: &ObservationStatus,
    ) -> Result<Vec<UnmappedObservation>, RepositoryError>;
}

/// The registry change version exposed to metrics and cache invalidation.
#[async_trait]
pub trait RegistryMetadataRepository: Send + Sync {
    /// The current registry version.
    async fn current_version(&self) -> Result<i64, RepositoryError>;

    /// Increments and returns the registry version.
    async fn bump_version(&self) -> Result<i64, RepositoryError>;
}

/// An audit failure. A change that cannot be audited is refused (BR3.1).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuditError {
    /// The audit destination is unavailable.
    #[error("audit sink unavailable: {0}")]
    Unavailable(String),
    /// The audit destination rejected the event.
    #[error("audit sink rejected the event: {0}")]
    Rejected(String),
}

/// Records security-sensitive events with actor, action, target, time, and outcome.
#[async_trait]
pub trait AuditSink: Send + Sync {
    /// Records one event.
    async fn record(&self, event: &crate::audit::AuditEvent) -> Result<(), AuditError>;
}

/// An injectable clock so tests control time.
pub trait Clock: Send + Sync {
    /// The current UTC time.
    fn now(&self) -> DateTime<Utc>;
}

/// A provider failure, classified for fallback decisions (BR2.1).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProviderError {
    /// The provider did not answer in time; the next source is tried.
    #[error("provider timed out")]
    Timeout,
    /// The provider is unavailable; the next source is tried.
    #[error("provider unavailable: {0}")]
    Unavailable(String),
    /// The provider rejected the request; the next source is tried.
    #[error("provider rejected the request: {0}")]
    Rejected(String),
}

/// A price provider. Returning `Ok(None)` means "no usable price", never zero.
#[async_trait]
pub trait PriceProvider: Send + Sync {
    /// Fetches a usable price for an asset from one source.
    async fn fetch_price(
        &self,
        asset: &Asset,
        source: &PriceSource,
    ) -> Result<Option<Price>, ProviderError>;
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
    /// A registry administrator.
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

/// Validates a Supabase JWT against JWKS before any routing (BR4.1).
#[async_trait]
pub trait TokenValidator: Send + Sync {
    /// Validates the token and returns the caller.
    async fn validate(&self, token: &str) -> Result<Principal, AuthError>;
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
