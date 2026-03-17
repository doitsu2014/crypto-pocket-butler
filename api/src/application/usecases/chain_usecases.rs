/// Chain use cases — application-layer orchestration for chain/network queries.
///
/// The `chains.rs` handler uses [`ChainUseCases`] to obtain the list of
/// supported EVM chains and the static Solana entry without touching
/// SeaORM entities directly.

use std::sync::Arc;

use crate::domains::chain::{
    aggregate::{ChainError, EvmChain},
    repository::ChainRepository,
};

/// Container for all chain-related use cases.
///
/// Shared as `Arc<ChainUseCases>` via Axum's `Extension` extractor.
pub struct ChainUseCases {
    repo: Arc<dyn ChainRepository>,
}

impl ChainUseCases {
    pub fn new(repo: Arc<dyn ChainRepository>) -> Self {
        Self { repo }
    }

    /// Return all active EVM chains configured in the database.
    pub async fn list_active_chains(&self) -> Result<Vec<EvmChain>, ChainError> {
        self.repo.find_active_chains().await
    }
}
