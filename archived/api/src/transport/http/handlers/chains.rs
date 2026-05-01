use axum::{extract::Extension, routing::get, Json, Router};
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

use crate::application::usecases::chain_usecases::ChainUseCases;

/// Supported chain information
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ChainInfo {
    /// Chain identifier (e.g., "ethereum", "solana")
    pub id: String,
    /// Human-readable chain name
    pub name: String,
    /// Native token symbol
    pub native_symbol: String,
}

/// Response containing list of supported chains
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListChainsResponse {
    /// List of supported chains
    pub chains: Vec<ChainInfo>,
}

/// List all supported chains
///
/// Returns a list of active chains from the database that can be selected for wallet accounts.
/// EVM chains are used in the `enabled_chains` field when creating or updating wallet accounts.
/// Solana wallets use `exchange_name: "solana"` with no `enabled_chains` needed.
#[utoipa::path(
    get,
    path = "/v1/chains",
    responses(
        (status = 200, description = "List of supported chains", body = ListChainsResponse),
    ),
    tag = "chains"
)]
pub async fn list_supported_chains(
    Extension(use_cases): Extension<Arc<ChainUseCases>>,
) -> Json<ListChainsResponse> {
    let evm_chains = use_cases.list_active_chains().await.unwrap_or_default();

    let mut chains: Vec<ChainInfo> = evm_chains
        .into_iter()
        .map(|c| ChainInfo {
            id: c.chain_id,
            name: c.name,
            native_symbol: c.native_symbol,
        })
        .collect();

    // Solana is not stored in the evm_chains table; append it here
    chains.push(ChainInfo {
        id: "solana".to_string(),
        name: "Solana".to_string(),
        native_symbol: "SOL".to_string(),
    });

    Json(ListChainsResponse { chains })
}

/// Create router for chains endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new().route("/v1/chains", get(list_supported_chains))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_info_serialization() {
        let info = ChainInfo {
            id: "ethereum".to_string(),
            name: "Ethereum".to_string(),
            native_symbol: "ETH".to_string(),
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("ethereum"));
        assert!(json.contains("ETH"));
    }
}
