/// ChainRepository trait — persistence interface for the Chain domain.

use async_trait::async_trait;
use uuid::Uuid;

use super::{
    aggregate::{ChainError, EvmChain},
    entities::SolanaToken,
};

/// Persistence interface for `EvmChain` aggregates and `SolanaToken` entities.
#[async_trait]
pub trait ChainRepository: Send + Sync {
    // ─── EVM chains ──────────────────────────────────────────────────────────────

    /// Find a chain by primary key.
    async fn find_chain_by_id(&self, id: Uuid) -> Result<Option<EvmChain>, ChainError>;

    /// Find a chain by its string identifier (e.g. "ethereum").
    async fn find_chain_by_chain_id(&self, chain_id: &str)
        -> Result<Option<EvmChain>, ChainError>;

    /// Return all configured chains.
    async fn find_all_chains(&self) -> Result<Vec<EvmChain>, ChainError>;

    /// Return all active chains.
    async fn find_active_chains(&self) -> Result<Vec<EvmChain>, ChainError>;

    /// Persist a chain.
    async fn save_chain(&self, chain: &EvmChain) -> Result<(), ChainError>;

    /// Remove a chain by primary key.
    async fn delete_chain(&self, id: Uuid) -> Result<bool, ChainError>;

    // ─── Solana tokens ───────────────────────────────────────────────────────────

    /// Return all Solana tokens.
    async fn find_all_solana_tokens(&self) -> Result<Vec<SolanaToken>, ChainError>;

    /// Persist a Solana token.
    async fn save_solana_token(&self, token: &SolanaToken) -> Result<(), ChainError>;

    /// Remove a Solana token.
    async fn delete_solana_token(&self, id: Uuid) -> Result<bool, ChainError>;
}
