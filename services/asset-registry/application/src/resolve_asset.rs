//! WF1: resolve an observation to a canonical asset, or quarantine it (BR1.1, BR1.3).

use std::sync::Arc;

use asset_registry_domain::{
    normalize_symbol, Asset, ChainId, ContractAddress, ObservationStatus, UnmappedObservation,
};
use uuid::Uuid;

use crate::ports::{
    AssetRepository, ChainRepository, Clock, ObservationRepository, RepositoryError,
};

/// One observation to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveAssetQuery {
    /// The chain identifier, e.g. `ethereum`.
    pub chain: String,
    /// The observed contract address, when the holding is a token.
    pub contract_address: Option<String>,
    /// The observed symbol.
    pub symbol: Option<String>,
}

/// The resolution outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolveOutcome {
    /// A canonical asset resolved.
    Resolved(Asset),
    /// Nothing resolved; the observation is quarantined and excluded (BR1.3).
    Unresolved(UnmappedObservation),
}

/// Why a resolution could not run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResolveError {
    /// The request did not carry a usable chain, contract, or symbol.
    #[error("invalid input: {0}")]
    InvalidInput(String),
    /// The chain is not configured.
    #[error("unknown chain: {0}")]
    UnknownChain(String),
    /// The chain is configured but excluded from launch coverage.
    #[error("chain is out of launch coverage: {0}")]
    OutOfCoverage(String),
    /// A port failed.
    #[error(transparent)]
    Repository(#[from] RepositoryError),
}

/// Resolves observations to canonical assets.
pub struct ResolveAsset {
    assets: Arc<dyn AssetRepository>,
    chains: Arc<dyn ChainRepository>,
    observations: Arc<dyn ObservationRepository>,
    clock: Arc<dyn Clock>,
}

impl ResolveAsset {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        assets: Arc<dyn AssetRepository>,
        chains: Arc<dyn ChainRepository>,
        observations: Arc<dyn ObservationRepository>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            assets,
            chains,
            observations,
            clock,
        }
    }

    /// Resolves the query or quarantines the observation.
    ///
    /// # Errors
    /// Returns [`ResolveError`] for invalid input, an unknown or out-of-coverage
    /// chain, or a repository failure.
    pub async fn execute(&self, query: ResolveAssetQuery) -> Result<ResolveOutcome, ResolveError> {
        let chain_id = ChainId::new(query.chain.trim())
            .map_err(|error| ResolveError::InvalidInput(error.to_string()))?;
        let chain = self
            .chains
            .find_by_id(&chain_id)
            .await?
            .ok_or_else(|| ResolveError::UnknownChain(chain_id.to_string()))?;
        if !chain.launch_coverage() {
            return Err(ResolveError::OutOfCoverage(chain_id.to_string()));
        }

        let contract = query
            .contract_address
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let (observed_symbol, contract_address, contract_key) = if let Some(raw_address) = contract
        {
            let address = ContractAddress::parse(raw_address)
                .map_err(|error| ResolveError::InvalidInput(error.to_string()))?;
            if let Some(asset) = self
                .assets
                .find_asset_by_contract(&chain_id, chain.family(), &address)
                .await?
            {
                return Ok(ResolveOutcome::Resolved(asset));
            }
            let symbol = self.required_symbol(&query)?;
            let key = address.key_for(chain.family());
            (symbol, Some(address), Some(key))
        } else {
            let symbol = self.required_symbol(&query)?;
            if let Some(asset) = self.assets.find_native_asset(&chain_id, &symbol).await? {
                return Ok(ResolveOutcome::Resolved(asset));
            }
            (symbol, None, None)
        };

        if let Some(existing) = self
            .observations
            .find_by_token(&chain_id, &observed_symbol, contract_key.as_deref())
            .await?
        {
            return self.outcome_for_existing(existing).await;
        }

        let observation = UnmappedObservation::quarantine(
            Uuid::new_v4().to_string(),
            &observed_symbol,
            &chain_id,
            contract_address,
            self.clock.now(),
        )
        .map_err(|error| ResolveError::InvalidInput(error.to_string()))?;

        match self
            .observations
            .insert(&observation, contract_key.as_deref())
            .await
        {
            Ok(()) => Ok(ResolveOutcome::Unresolved(observation)),
            Err(RepositoryError::Conflict(_)) => {
                // Another request quarantined the same token first; return that record.
                let existing = self
                    .observations
                    .find_by_token(&chain_id, &observed_symbol, contract_key.as_deref())
                    .await?
                    .ok_or_else(|| {
                        RepositoryError::Unavailable(
                            "quarantine insert raced but no record is visible".to_string(),
                        )
                    })?;
                Ok(ResolveOutcome::Unresolved(existing))
            }
            Err(error) => Err(ResolveError::Repository(error)),
        }
    }

    fn required_symbol(&self, query: &ResolveAssetQuery) -> Result<String, ResolveError> {
        let symbol = query
            .symbol
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                ResolveError::InvalidInput(
                    "symbol is required to identify the observation".to_string(),
                )
            })?;
        normalize_symbol(symbol).map_err(|error| ResolveError::InvalidInput(error.to_string()))
    }

    async fn outcome_for_existing(
        &self,
        observation: UnmappedObservation,
    ) -> Result<ResolveOutcome, ResolveError> {
        match observation.status() {
            ObservationStatus::Resolved { asset_id } => {
                let asset_id = asset_id.clone();
                match self.assets.find_asset_by_id(&asset_id).await? {
                    Some(asset) => Ok(ResolveOutcome::Resolved(asset)),
                    None => Err(ResolveError::Repository(RepositoryError::InvalidData(
                        format!("resolved observation links to missing asset '{asset_id}'"),
                    ))),
                }
            }
            _ => Ok(ResolveOutcome::Unresolved(observation)),
        }
    }
}
