/// Chain domain entities: EvmToken and SolanaToken

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An ERC-20 token registered for a specific EVM chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvmToken {
    pub id: Uuid,
    /// EVM chain identifier (e.g. "ethereum", "bsc").
    pub chain: String,
    /// Token symbol (e.g. "USDC").
    pub symbol: String,
    /// ERC-20 contract address (checksummed hex).
    pub contract_address: String,
    /// Whether the EVM connector includes this token during sync.
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EvmToken {
    pub fn new(
        id: Uuid,
        chain: impl Into<String>,
        symbol: impl Into<String>,
        contract_address: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            chain: chain.into(),
            symbol: symbol.into(),
            contract_address: contract_address.into(),
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}

/// A Solana SPL token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaToken {
    pub id: Uuid,
    /// Token symbol (e.g. "USDC", "BONK").
    pub symbol: String,
    /// SPL token mint address (Base58 encoded).
    pub mint_address: String,
    /// Whether the Solana connector includes this token during sync.
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SolanaToken {
    pub fn new(
        id: Uuid,
        symbol: impl Into<String>,
        mint_address: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id,
            symbol: symbol.into(),
            mint_address: mint_address.into(),
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }
}
