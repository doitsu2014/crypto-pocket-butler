//! WF3: administer the registry (BR1.2, BR2.1, BR3.1).

use std::sync::Arc;

use asset_registry_domain::{
    ensure_decimals_match, Asset, AssetId, ChainId, ContractAddress, ObservationId, PriceSource,
    TokenContract, UnmappedObservation,
};

use crate::audit::AuditEvent;
use crate::ports::{
    AssetRepository, AuditError, AuditSink, ChainRepository, Clock, ObservationRepository,
    PriceSourceRepository, RegistryMetadataRepository, RepositoryError, Role,
};

/// The authenticated operator performing an administrative change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminActor {
    /// The Supabase Auth subject.
    pub subject: String,
    /// The caller's role.
    pub role: Role,
}

impl AdminActor {
    /// Creates an actor.
    ///
    /// # Errors
    /// Returns [`AdminError::Forbidden`] for a blank subject.
    pub fn new(subject: impl Into<String>, role: Role) -> Result<Self, AdminError> {
        let subject = subject.into();
        if subject.trim().is_empty() {
            return Err(AdminError::Forbidden("empty principal".to_string()));
        }
        Ok(Self { subject, role })
    }

    fn ensure_admin(&self) -> Result<(), AdminError> {
        if self.role == Role::Admin {
            Ok(())
        } else {
            Err(AdminError::Forbidden(format!(
                "role {:?} may not administer the registry",
                self.role
            )))
        }
    }
}

/// Adds a canonical asset.
#[derive(Debug, Clone)]
pub struct AddAssetCommand {
    /// The stable identifier.
    pub id: String,
    /// The ticker.
    pub symbol: String,
    /// The display name.
    pub name: String,
    /// The canonical precision.
    pub decimals: u8,
    /// When set, the asset is native to this chain and resolves by symbol.
    pub native_chain: Option<String>,
}

/// Adds a chain-plus-contract mapping.
#[derive(Debug, Clone)]
pub struct AddTokenContractCommand {
    /// The stable identifier.
    pub id: String,
    /// The chain identifier.
    pub chain_id: String,
    /// The contract address as observed.
    pub contract_address: String,
    /// The canonical asset identifier.
    pub asset_id: String,
    /// The contract precision; must equal the asset's.
    pub decimals: u8,
}

/// Creates or updates a price source.
#[derive(Debug, Clone)]
pub struct UpsertPriceSourceCommand {
    /// The stable identifier.
    pub id: String,
    /// The unique source name.
    pub name: String,
    /// Lower values are tried first.
    pub priority: u32,
    /// The licence that permits production use.
    pub licence_ref: Option<String>,
    /// Whether the source should be enabled.
    pub enabled: bool,
}

/// Maps a quarantined observation to an asset.
#[derive(Debug, Clone)]
pub struct ResolveObservationCommand {
    /// The observation identifier.
    pub observation_id: String,
    /// The canonical asset to link.
    pub asset_id: String,
}

/// Excludes a quarantined observation permanently.
#[derive(Debug, Clone)]
pub struct IgnoreObservationCommand {
    /// The observation identifier.
    pub observation_id: String,
    /// Why the observation is ignored.
    pub reason: String,
}

/// Why an administrative change was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AdminError {
    /// The caller may not administer the registry.
    #[error("forbidden: {0}")]
    Forbidden(String),
    /// A domain invariant or input check failed.
    #[error("validation: {0}")]
    Validation(String),
    /// A referenced record does not exist.
    #[error("not found: {0}")]
    NotFound(String),
    /// The change conflicts with existing state or a uniqueness rule.
    #[error("conflict: {0}")]
    Conflict(String),
    /// The change could not be audited and is therefore refused (BR3.1).
    #[error("audit refused the change: {0}")]
    Audit(String),
    /// A repository failed.
    #[error("repository unavailable: {0}")]
    Repository(String),
}

impl From<RepositoryError> for AdminError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::Conflict(message) => Self::Conflict(message),
            RepositoryError::InvalidData(message) => Self::Validation(message),
            RepositoryError::Unavailable(message) => Self::Repository(message),
        }
    }
}

impl From<asset_registry_domain::DomainError> for AdminError {
    fn from(error: asset_registry_domain::DomainError) -> Self {
        match error {
            asset_registry_domain::DomainError::InvalidTransition { .. } => {
                Self::Conflict(error.to_string())
            }
            other => Self::Validation(other.to_string()),
        }
    }
}

impl From<AuditError> for AdminError {
    fn from(error: AuditError) -> Self {
        Self::Audit(error.to_string())
    }
}

/// Administers the registry and audits every successful change.
pub struct RegistryAdmin {
    assets: Arc<dyn AssetRepository>,
    chains: Arc<dyn ChainRepository>,
    sources: Arc<dyn PriceSourceRepository>,
    observations: Arc<dyn ObservationRepository>,
    metadata: Arc<dyn RegistryMetadataRepository>,
    audit: Arc<dyn AuditSink>,
    clock: Arc<dyn Clock>,
}

impl RegistryAdmin {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        assets: Arc<dyn AssetRepository>,
        chains: Arc<dyn ChainRepository>,
        sources: Arc<dyn PriceSourceRepository>,
        observations: Arc<dyn ObservationRepository>,
        metadata: Arc<dyn RegistryMetadataRepository>,
        audit: Arc<dyn AuditSink>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            assets,
            chains,
            sources,
            observations,
            metadata,
            audit,
            clock,
        }
    }

    /// Adds an asset.
    ///
    /// # Errors
    /// Returns [`AdminError`] when the caller is not an admin, the asset is
    /// invalid, the native chain is unknown, the change conflicts, or it cannot
    /// be audited.
    pub async fn add_asset(
        &self,
        actor: &AdminActor,
        command: AddAssetCommand,
        correlation_id: Option<&str>,
    ) -> Result<Asset, AdminError> {
        actor.ensure_admin()?;
        let native_chain = command
            .native_chain
            .as_deref()
            .map(ChainId::new)
            .transpose()
            .map_err(|error| AdminError::Validation(error.to_string()))?;
        if let Some(chain_id) = &native_chain {
            if self.chains.find_by_id(chain_id).await?.is_none() {
                return Err(AdminError::NotFound(format!("chain '{chain_id}'")));
            }
        }
        let asset = Asset::new(
            command.id,
            &command.symbol,
            &command.name,
            command.decimals,
            true,
        )?;
        self.assets
            .insert_asset(&asset, native_chain.as_ref())
            .await?;
        self.finish(actor, "asset.create", asset.id().as_str(), correlation_id)
            .await?;
        Ok(asset)
    }

    /// Adds a chain-plus-contract mapping.
    ///
    /// # Errors
    /// Returns [`AdminError`] when the caller is not an admin, the chain or asset
    /// is unknown, the decimals differ (BR1.2), the mapping duplicates an existing
    /// one (BR1.1), or the change cannot be audited.
    pub async fn add_token_contract(
        &self,
        actor: &AdminActor,
        command: AddTokenContractCommand,
        correlation_id: Option<&str>,
    ) -> Result<TokenContract, AdminError> {
        actor.ensure_admin()?;
        let chain_id = ChainId::new(command.chain_id)
            .map_err(|error| AdminError::Validation(error.to_string()))?;
        if self.chains.find_by_id(&chain_id).await?.is_none() {
            return Err(AdminError::NotFound(format!("chain '{chain_id}'")));
        }
        let asset_id = AssetId::new(command.asset_id)
            .map_err(|error| AdminError::Validation(error.to_string()))?;
        let asset = self
            .assets
            .find_asset_by_id(&asset_id)
            .await?
            .ok_or_else(|| AdminError::NotFound(format!("asset '{asset_id}'")))?;
        let address = ContractAddress::parse(&command.contract_address)?;
        ensure_decimals_match(&asset, command.decimals)?;
        let contract =
            TokenContract::new(command.id, &chain_id, address, &asset_id, command.decimals)?;
        self.assets.insert_token_contract(&contract).await?;
        self.finish(
            actor,
            "token_contract.create",
            contract.id().as_str(),
            correlation_id,
        )
        .await?;
        Ok(contract)
    }

    /// Creates or updates a price source.
    ///
    /// # Errors
    /// Returns [`AdminError`] when the caller is not an admin, an enabled source
    /// has no licence, the priority is already held (BR2.1), or the change cannot
    /// be audited.
    pub async fn upsert_price_source(
        &self,
        actor: &AdminActor,
        command: UpsertPriceSourceCommand,
        correlation_id: Option<&str>,
    ) -> Result<PriceSource, AdminError> {
        actor.ensure_admin()?;
        let mut source = PriceSource::new(
            command.id,
            &command.name,
            command.priority,
            command.licence_ref.as_deref(),
        )?;
        if command.enabled {
            source.enable()?;
        } else {
            source.disable();
        }
        if source.enabled()
            && self
                .sources
                .enabled_priority_exists(source.priority(), Some(source.name()))
                .await?
        {
            return Err(AdminError::Conflict(format!(
                "priority {} is already held by another enabled source",
                source.priority()
            )));
        }
        self.sources.upsert(&source).await?;
        self.finish(actor, "price_source.upsert", source.name(), correlation_id)
            .await?;
        Ok(source)
    }

    /// Enables or disables a price source.
    ///
    /// # Errors
    /// Returns [`AdminError`] when the source is unknown, enabling is unlicensed
    /// (BR2.1), the priority is already held, or the change cannot be audited.
    pub async fn set_price_source_enabled(
        &self,
        actor: &AdminActor,
        name: &str,
        enabled: bool,
        correlation_id: Option<&str>,
    ) -> Result<PriceSource, AdminError> {
        actor.ensure_admin()?;
        let mut source = self
            .sources
            .find_by_name(name)
            .await?
            .ok_or_else(|| AdminError::NotFound(format!("price source '{name}'")))?;
        if enabled {
            source.enable()?;
            if self
                .sources
                .enabled_priority_exists(source.priority(), Some(source.name()))
                .await?
            {
                return Err(AdminError::Conflict(format!(
                    "priority {} is already held by another enabled source",
                    source.priority()
                )));
            }
        } else {
            source.disable();
        }
        self.sources.set_enabled(source.name(), enabled).await?;
        let action = if enabled {
            "price_source.enable"
        } else {
            "price_source.disable"
        };
        self.finish(actor, action, source.name(), correlation_id)
            .await?;
        Ok(source)
    }

    /// Maps a quarantined observation to a canonical asset.
    ///
    /// # Errors
    /// Returns [`AdminError`] when the caller is not an admin, the observation or
    /// asset is unknown, the transition is refused, or the change cannot be audited.
    pub async fn resolve_observation(
        &self,
        actor: &AdminActor,
        command: ResolveObservationCommand,
        correlation_id: Option<&str>,
    ) -> Result<UnmappedObservation, AdminError> {
        actor.ensure_admin()?;
        let observation_id = ObservationId::new(command.observation_id)
            .map_err(|error| AdminError::Validation(error.to_string()))?;
        let mut observation = self
            .observations
            .find_by_id(&observation_id)
            .await?
            .ok_or_else(|| AdminError::NotFound(format!("observation '{observation_id}'")))?;
        let asset_id = AssetId::new(command.asset_id)
            .map_err(|error| AdminError::Validation(error.to_string()))?;
        if self.assets.find_asset_by_id(&asset_id).await?.is_none() {
            return Err(AdminError::NotFound(format!("asset '{asset_id}'")));
        }
        observation.resolve(asset_id)?;
        self.observations.save_status(&observation).await?;
        self.finish(
            actor,
            "observation.resolve",
            observation.id().as_str(),
            correlation_id,
        )
        .await?;
        Ok(observation)
    }

    /// Excludes a quarantined observation permanently.
    ///
    /// # Errors
    /// Returns [`AdminError`] when the caller is not an admin, the observation is
    /// unknown, the reason is blank, the transition is refused, or the change
    /// cannot be audited.
    pub async fn ignore_observation(
        &self,
        actor: &AdminActor,
        command: IgnoreObservationCommand,
        correlation_id: Option<&str>,
    ) -> Result<UnmappedObservation, AdminError> {
        actor.ensure_admin()?;
        let observation_id = ObservationId::new(command.observation_id)
            .map_err(|error| AdminError::Validation(error.to_string()))?;
        let mut observation = self
            .observations
            .find_by_id(&observation_id)
            .await?
            .ok_or_else(|| AdminError::NotFound(format!("observation '{observation_id}'")))?;
        observation.ignore(&command.reason)?;
        self.observations.save_status(&observation).await?;
        self.finish(
            actor,
            "observation.ignore",
            observation.id().as_str(),
            correlation_id,
        )
        .await?;
        Ok(observation)
    }

    async fn finish(
        &self,
        actor: &AdminActor,
        action: &str,
        target: &str,
        correlation_id: Option<&str>,
    ) -> Result<(), AdminError> {
        self.metadata.bump_version().await?;
        let event = AuditEvent::new(
            &actor.subject,
            action,
            target,
            "success",
            correlation_id.map(ToString::to_string),
            self.clock.now(),
        )?;
        self.audit.record(&event).await?;
        Ok(())
    }
}
