use axum::{routing::get, Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Supported EVM chain information
#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct ChainInfo {
    /// Chain identifier (e.g., "ethereum", "arbitrum")
    pub id: String,
    /// Human-readable chain name
    pub name: String,
    /// Native token symbol
    pub native_symbol: String,
}

/// Response containing list of supported chains
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListChainsResponse {
    /// List of supported EVM chains
    pub chains: Vec<ChainInfo>,
}

/// List all supported EVM chains
///
/// Returns a list of EVM chains that can be selected for wallet accounts.
/// These chains are used in the `enabled_chains` field when creating or updating wallet accounts.
#[utoipa::path(
    get,
    path = "/v1/chains",
    responses(
        (status = 200, description = "List of supported EVM chains", body = ListChainsResponse),
    ),
    tag = "chains"
)]
pub async fn list_supported_chains() -> Json<ListChainsResponse> {
    let chains = vec![
        ChainInfo {
            id: "ethereum".to_string(),
            name: "Ethereum".to_string(),
            native_symbol: "ETH".to_string(),
        },
        ChainInfo {
            id: "arbitrum".to_string(),
            name: "Arbitrum".to_string(),
            native_symbol: "ETH".to_string(),
        },
        ChainInfo {
            id: "optimism".to_string(),
            name: "Optimism".to_string(),
            native_symbol: "ETH".to_string(),
        },
        ChainInfo {
            id: "base".to_string(),
            name: "Base".to_string(),
            native_symbol: "ETH".to_string(),
        },
        ChainInfo {
            id: "bsc".to_string(),
            name: "BNB Smart Chain".to_string(),
            native_symbol: "BNB".to_string(),
        },
    ];

    Json(ListChainsResponse { chains })
}

/// Create router for chains endpoints
pub fn create_router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().route("/v1/chains", get(list_supported_chains))
}
