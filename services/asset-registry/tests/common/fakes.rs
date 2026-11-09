//! Port fakes for unit-level application and HTTP tests.
//!
//! Each fake is small, deterministic, and records what the use case asked it to do.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Mutex;

use asset_registry_application::audit::AuditEvent;
use asset_registry_application::ports::{
    AssetRepository, AuditError, AuditSink, AuthError, ChainRepository, Clock,
    ObservationRepository, PriceProvider, PriceSourceRepository, Principal, ProviderError,
    RegistryMetadataRepository, RepositoryError, Role, TokenValidator,
};
use asset_registry_domain::{
    Asset, AssetId, Chain, ChainFamily, ChainId, ContractAddress, ObservationId, ObservationStatus,
    Price, PriceSource, TokenContract, UnmappedObservation,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

pub struct FakeAssets {
    pub assets: Mutex<Vec<Asset>>,
    pub contracts: Mutex<Vec<TokenContract>>,
    pub native: Mutex<Vec<(ChainId, AssetId)>>,
    pub failure: Mutex<Option<RepositoryError>>,
}

impl Default for FakeAssets {
    fn default() -> Self {
        Self {
            assets: Mutex::new(Vec::new()),
            contracts: Mutex::new(Vec::new()),
            native: Mutex::new(Vec::new()),
            failure: Mutex::new(None),
        }
    }
}

impl FakeAssets {
    fn maybe_fail(&self) -> Result<(), RepositoryError> {
        match self.failure.lock().unwrap().take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    pub fn asset_count(&self) -> usize {
        self.assets.lock().unwrap().len()
    }

    pub fn contract_count(&self) -> usize {
        self.contracts.lock().unwrap().len()
    }
}

#[async_trait]
impl AssetRepository for FakeAssets {
    async fn find_asset_by_contract(
        &self,
        chain: &ChainId,
        family: ChainFamily,
        address: &ContractAddress,
    ) -> Result<Option<Asset>, RepositoryError> {
        self.maybe_fail()?;
        let key = address.key_for(family);
        let asset_id = {
            let contracts = self.contracts.lock().unwrap();
            contracts
                .iter()
                .find(|contract| {
                    contract.chain_id() == chain
                        && contract.contract_address().key_for(family) == key
                })
                .map(|contract| contract.asset_id().clone())
        };
        match asset_id {
            Some(id) => self.find_asset_by_id(&id).await,
            None => Ok(None),
        }
    }

    async fn find_native_asset(
        &self,
        chain: &ChainId,
        symbol: &str,
    ) -> Result<Option<Asset>, RepositoryError> {
        self.maybe_fail()?;
        let links = self.native.lock().unwrap();
        let assets = self.assets.lock().unwrap();
        Ok(links
            .iter()
            .filter(|(linked_chain, _)| linked_chain == chain)
            .find_map(|(_, asset_id)| {
                assets
                    .iter()
                    .find(|asset| asset.id() == asset_id && asset.symbol() == symbol)
                    .cloned()
            }))
    }

    async fn find_asset_by_id(&self, id: &AssetId) -> Result<Option<Asset>, RepositoryError> {
        self.maybe_fail()?;
        Ok(self
            .assets
            .lock()
            .unwrap()
            .iter()
            .find(|asset| asset.id() == id)
            .cloned())
    }

    async fn insert_asset(
        &self,
        asset: &Asset,
        native_chain: Option<&ChainId>,
    ) -> Result<(), RepositoryError> {
        self.maybe_fail()?;
        self.assets.lock().unwrap().push(asset.clone());
        if let Some(chain) = native_chain {
            self.native
                .lock()
                .unwrap()
                .push((chain.clone(), asset.id().clone()));
        }
        Ok(())
    }

    async fn insert_token_contract(&self, contract: &TokenContract) -> Result<(), RepositoryError> {
        self.maybe_fail()?;
        let key = contract.contract_address().as_str().to_ascii_lowercase();
        let mut contracts = self.contracts.lock().unwrap();
        if contracts.iter().any(|existing| {
            existing.chain_id() == contract.chain_id()
                && existing.contract_address().as_str().to_ascii_lowercase() == key
        }) {
            return Err(RepositoryError::Conflict(
                "duplicate chain-plus-contract mapping".to_string(),
            ));
        }
        contracts.push(contract.clone());
        Ok(())
    }
}

pub struct FakeChains {
    pub chains: Mutex<Vec<Chain>>,
}

impl FakeChains {
    pub fn with_launch_chains() -> Self {
        let chains = vec![
            Chain::new("ethereum", "Ethereum", ChainFamily::Evm, true).unwrap(),
            Chain::new("bnb-smart-chain", "BNB Smart Chain", ChainFamily::Bnb, true).unwrap(),
            Chain::new("solana", "Solana", ChainFamily::Solana, true).unwrap(),
            Chain::new("legacy-testnet", "Legacy Testnet", ChainFamily::Evm, false).unwrap(),
        ];
        Self {
            chains: Mutex::new(chains),
        }
    }
}

impl Default for FakeChains {
    fn default() -> Self {
        Self::with_launch_chains()
    }
}

#[async_trait]
impl ChainRepository for FakeChains {
    async fn find_by_id(&self, id: &ChainId) -> Result<Option<Chain>, RepositoryError> {
        Ok(self
            .chains
            .lock()
            .unwrap()
            .iter()
            .find(|chain| chain.id() == id)
            .cloned())
    }

    async fn list(&self) -> Result<Vec<Chain>, RepositoryError> {
        Ok(self.chains.lock().unwrap().clone())
    }

    async fn upsert(&self, chain: &Chain) -> Result<(), RepositoryError> {
        let mut chains = self.chains.lock().unwrap();
        chains.retain(|existing| existing.id() != chain.id());
        chains.push(chain.clone());
        Ok(())
    }
}

pub struct FakeSources {
    pub sources: Mutex<Vec<PriceSource>>,
}

impl Default for FakeSources {
    fn default() -> Self {
        Self {
            sources: Mutex::new(Vec::new()),
        }
    }
}

impl FakeSources {
    pub fn with_sources(sources: Vec<PriceSource>) -> Self {
        Self {
            sources: Mutex::new(sources),
        }
    }
}

#[async_trait]
impl PriceSourceRepository for FakeSources {
    async fn list_enabled_by_priority(&self) -> Result<Vec<PriceSource>, RepositoryError> {
        let mut enabled: Vec<_> = self
            .sources
            .lock()
            .unwrap()
            .iter()
            .filter(|source| source.enabled())
            .cloned()
            .collect();
        enabled.sort_by_key(PriceSource::priority);
        Ok(enabled)
    }

    async fn list(&self) -> Result<Vec<PriceSource>, RepositoryError> {
        Ok(self.sources.lock().unwrap().clone())
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<PriceSource>, RepositoryError> {
        Ok(self
            .sources
            .lock()
            .unwrap()
            .iter()
            .find(|source| source.name() == name)
            .cloned())
    }

    async fn upsert(&self, source: &PriceSource) -> Result<(), RepositoryError> {
        let mut sources = self.sources.lock().unwrap();
        if source.enabled()
            && sources.iter().any(|existing| {
                existing.enabled()
                    && existing.priority() == source.priority()
                    && existing.name() != source.name()
            })
        {
            return Err(RepositoryError::Conflict(
                "priority already held".to_string(),
            ));
        }
        sources.retain(|existing| existing.name() != source.name());
        sources.push(source.clone());
        Ok(())
    }

    async fn set_enabled(&self, name: &str, enabled: bool) -> Result<(), RepositoryError> {
        let mut sources = self.sources.lock().unwrap();
        let source = sources
            .iter_mut()
            .find(|source| source.name() == name)
            .ok_or_else(|| RepositoryError::Conflict(format!("price source '{name}' not found")))?;
        if enabled {
            source
                .enable()
                .map_err(|error| RepositoryError::Conflict(error.to_string()))?;
        } else {
            source.disable();
        }
        Ok(())
    }

    async fn enabled_priority_exists(
        &self,
        priority: u32,
        except_name: Option<&str>,
    ) -> Result<bool, RepositoryError> {
        Ok(self.sources.lock().unwrap().iter().any(|source| {
            source.enabled() && source.priority() == priority && except_name != Some(source.name())
        }))
    }
}

#[derive(Clone)]
pub struct StoredObservation {
    pub observation: UnmappedObservation,
    pub contract_key: Option<String>,
}

pub struct FakeObservations {
    pub observations: Mutex<Vec<StoredObservation>>,
}

impl Default for FakeObservations {
    fn default() -> Self {
        Self {
            observations: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl ObservationRepository for FakeObservations {
    async fn find_by_token(
        &self,
        chain: &ChainId,
        observed_symbol: &str,
        contract_key: Option<&str>,
    ) -> Result<Option<UnmappedObservation>, RepositoryError> {
        Ok(self
            .observations
            .lock()
            .unwrap()
            .iter()
            .find(|stored| {
                stored.observation.chain_id() == chain
                    && stored.observation.observed_symbol() == observed_symbol
                    && stored.contract_key.as_deref() == contract_key
            })
            .map(|stored| stored.observation.clone()))
    }

    async fn find_by_id(
        &self,
        id: &ObservationId,
    ) -> Result<Option<UnmappedObservation>, RepositoryError> {
        Ok(self
            .observations
            .lock()
            .unwrap()
            .iter()
            .find(|stored| stored.observation.id() == id)
            .map(|stored| stored.observation.clone()))
    }

    async fn insert(
        &self,
        observation: &UnmappedObservation,
        contract_key: Option<&str>,
    ) -> Result<(), RepositoryError> {
        let mut observations = self.observations.lock().unwrap();
        if observations.iter().any(|stored| {
            stored.observation.chain_id() == observation.chain_id()
                && stored.observation.observed_symbol() == observation.observed_symbol()
                && stored.contract_key.as_deref() == contract_key
        }) {
            return Err(RepositoryError::Conflict(
                "observation already quarantined".to_string(),
            ));
        }
        observations.push(StoredObservation {
            observation: observation.clone(),
            contract_key: contract_key.map(ToString::to_string),
        });
        Ok(())
    }

    async fn save_status(&self, observation: &UnmappedObservation) -> Result<(), RepositoryError> {
        let mut observations = self.observations.lock().unwrap();
        let stored = observations
            .iter_mut()
            .find(|stored| stored.observation.id() == observation.id())
            .ok_or_else(|| {
                RepositoryError::Conflict(format!("observation '{}' not found", observation.id()))
            })?;
        stored.observation = observation.clone();
        Ok(())
    }

    async fn list_by_status(
        &self,
        status: &ObservationStatus,
    ) -> Result<Vec<UnmappedObservation>, RepositoryError> {
        Ok(self
            .observations
            .lock()
            .unwrap()
            .iter()
            .filter(|stored| stored.observation.status().label() == status.label())
            .map(|stored| stored.observation.clone())
            .collect())
    }
}

pub struct FakeMetadata {
    pub version: AtomicI64,
}

impl Default for FakeMetadata {
    fn default() -> Self {
        Self {
            version: AtomicI64::new(0),
        }
    }
}

#[async_trait]
impl RegistryMetadataRepository for FakeMetadata {
    async fn current_version(&self) -> Result<i64, RepositoryError> {
        Ok(self.version.load(Ordering::SeqCst))
    }

    async fn bump_version(&self) -> Result<i64, RepositoryError> {
        Ok(self.version.fetch_add(1, Ordering::SeqCst) + 1)
    }
}

pub struct FakeAudit {
    pub events: Mutex<Vec<AuditEvent>>,
    pub fail: bool,
}

impl Default for FakeAudit {
    fn default() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            fail: false,
        }
    }
}

impl FakeAudit {
    pub fn failing() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
            fail: true,
        }
    }

    pub fn events(&self) -> Vec<AuditEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl AuditSink for FakeAudit {
    async fn record(&self, event: &AuditEvent) -> Result<(), AuditError> {
        if self.fail {
            return Err(AuditError::Unavailable("audit store down".to_string()));
        }
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

#[derive(Clone)]
pub struct FixedClock {
    pub now: DateTime<Utc>,
}

impl FixedClock {
    pub fn at(now: DateTime<Utc>) -> Self {
        Self { now }
    }
}

impl Clock for FixedClock {
    fn now(&self) -> DateTime<Utc> {
        self.now
    }
}

pub struct FakePriceProvider {
    pub responses: Mutex<HashMap<String, Result<Option<Price>, ProviderError>>>,
    pub calls: Mutex<Vec<String>>,
}

impl FakePriceProvider {
    pub fn with(responses: Vec<(&str, Result<Option<Price>, ProviderError>)>) -> Self {
        Self {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|(name, response)| (name.to_string(), response))
                    .collect(),
            ),
            calls: Mutex::new(Vec::new()),
        }
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[async_trait]
impl PriceProvider for FakePriceProvider {
    async fn fetch_price(
        &self,
        _asset: &Asset,
        source: &PriceSource,
    ) -> Result<Option<Price>, ProviderError> {
        self.calls.lock().unwrap().push(source.name().to_string());
        match self.responses.lock().unwrap().get(source.name()) {
            Some(Ok(price)) => Ok(*price),
            Some(Err(error)) => Err(error.clone()),
            None => Ok(None),
        }
    }
}

pub struct FakeTokenValidator {
    pub principals: Mutex<HashMap<String, Result<Principal, AuthError>>>,
}

impl FakeTokenValidator {
    pub fn with_token(token: &str, principal: Principal) -> Self {
        let mut principals = HashMap::new();
        principals.insert(token.to_string(), Ok(principal));
        Self {
            principals: Mutex::new(principals),
        }
    }

    pub fn user() -> Self {
        Self::with_token(
            "valid-user-token",
            Principal {
                subject: "user-1".to_string(),
                role: Role::User,
            },
        )
    }

    pub fn admin() -> Self {
        Self::with_token(
            "valid-admin-token",
            Principal {
                subject: "admin-1".to_string(),
                role: Role::Admin,
            },
        )
    }
}

#[async_trait]
impl TokenValidator for FakeTokenValidator {
    async fn validate(&self, token: &str) -> Result<Principal, AuthError> {
        match self.principals.lock().unwrap().get(token) {
            Some(result) => result.clone(),
            None => Err(AuthError::Invalid("unknown test token".to_string())),
        }
    }
}
