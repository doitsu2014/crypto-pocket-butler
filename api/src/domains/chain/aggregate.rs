/// EvmChain aggregate root

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::entities::EvmToken;

/// The EvmChain aggregate root.
///
/// Represents a configured EVM blockchain with its tokens.
///
/// # Invariants
/// - `chain_id` is unique across all chains.
/// - `rpc_url` must be a non-empty URL string.
/// - At most one token per (chain_id, symbol) pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmChain {
    pub id: Uuid,
    /// Unique identifier (e.g. "ethereum", "bsc", "arbitrum").
    pub chain_id: String,
    /// Human-readable name (e.g. "Ethereum").
    pub name: String,
    /// RPC endpoint URL.
    pub rpc_url: String,
    /// Native token symbol (e.g. "ETH", "BNB").
    pub native_symbol: String,
    /// Whether this chain is active for account sync.
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    tokens: Vec<EvmToken>,
}

impl EvmChain {
    pub fn new(
        id: Uuid,
        chain_id: impl Into<String>,
        name: impl Into<String>,
        rpc_url: impl Into<String>,
        native_symbol: impl Into<String>,
    ) -> Result<Self, ChainError> {
        let rpc_url = rpc_url.into();
        if rpc_url.is_empty() {
            return Err(ChainError::MissingRpcUrl);
        }
        let now = Utc::now();
        Ok(Self {
            id,
            chain_id: chain_id.into(),
            name: name.into(),
            rpc_url,
            native_symbol: native_symbol.into(),
            is_active: true,
            created_at: now,
            updated_at: now,
            tokens: Vec::new(),
        })
    }

    /// Reconstruct from persisted state.
    #[allow(clippy::too_many_arguments)]
    pub fn from_persistence(
        id: Uuid,
        chain_id: String,
        name: String,
        rpc_url: String,
        native_symbol: String,
        is_active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
        tokens: Vec<EvmToken>,
    ) -> Self {
        Self {
            id,
            chain_id,
            name,
            rpc_url,
            native_symbol,
            is_active,
            created_at,
            updated_at,
            tokens,
        }
    }

    pub fn tokens(&self) -> &[EvmToken] {
        &self.tokens
    }

    /// Add a token to this chain.
    pub fn add_token(&mut self, token: EvmToken) {
        self.tokens.push(token);
        self.updated_at = Utc::now();
    }

    /// Activate this chain for sync.
    pub fn activate(&mut self) {
        self.is_active = true;
        self.updated_at = Utc::now();
    }

    /// Deactivate this chain.
    pub fn deactivate(&mut self) {
        self.is_active = false;
        self.updated_at = Utc::now();
    }
}

/// Domain errors for the Chain aggregate.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ChainError {
    #[error("rpc_url is required")]
    MissingRpcUrl,
    #[error("chain not found")]
    NotFound,
    #[error("persistence error: {0}")]
    PersistenceError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_evm_chain() {
        let chain = EvmChain::new(
            Uuid::new_v4(),
            "ethereum",
            "Ethereum",
            "https://rpc.ankr.com/eth",
            "ETH",
        )
        .unwrap();
        assert_eq!(chain.chain_id, "ethereum");
        assert!(chain.is_active);
    }

    #[test]
    fn test_create_chain_missing_rpc() {
        let err =
            EvmChain::new(Uuid::new_v4(), "ethereum", "Ethereum", "", "ETH").unwrap_err();
        assert_eq!(err, ChainError::MissingRpcUrl);
    }
}
