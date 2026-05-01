/// Account DTOs — request and response types for Account API endpoints.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Request to create a new account.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAccountDto {
    /// Human-readable account name.
    pub name: String,
    /// Account type: `"exchange"` or `"wallet"`.
    pub account_type: String,
    /// Exchange name (required when `account_type == "exchange"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_name: Option<String>,
    /// Wallet address (required when `account_type == "wallet"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    /// EVM chains enabled for wallet accounts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_chains: Option<Vec<String>>,
    /// API key (exchange accounts only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// API secret (exchange accounts only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    /// Passphrase (exchange accounts only, e.g. OKX).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

/// Request to update an existing account.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAccountDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

/// API response for an account — credentials are **never** included.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountResponseDto {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub account_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_chains: Option<Vec<String>>,
    pub created_at: String,
    pub updated_at: String,
}
