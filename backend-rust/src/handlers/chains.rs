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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_list_supported_chains() {
        let response = list_supported_chains().await;
        let chains_response = response.0;
        
        // Should return 5 chains
        assert_eq!(chains_response.chains.len(), 5);
        
        // Verify all expected chains are present
        let chain_ids: Vec<String> = chains_response.chains.iter().map(|c| c.id.clone()).collect();
        assert!(chain_ids.contains(&"ethereum".to_string()));
        assert!(chain_ids.contains(&"arbitrum".to_string()));
        assert!(chain_ids.contains(&"optimism".to_string()));
        assert!(chain_ids.contains(&"base".to_string()));
        assert!(chain_ids.contains(&"bsc".to_string()));
        
        // Verify specific chain details
        let ethereum = chains_response.chains.iter().find(|c| c.id == "ethereum").unwrap();
        assert_eq!(ethereum.name, "Ethereum");
        assert_eq!(ethereum.native_symbol, "ETH");
        
        let bsc = chains_response.chains.iter().find(|c| c.id == "bsc").unwrap();
        assert_eq!(bsc.name, "BNB Smart Chain");
        assert_eq!(bsc.native_symbol, "BNB");
    }
}
