use axum::{extract::State, response::Json, routing::post, Router};
use axum_keycloak_auth::decode::KeycloakToken;
use axum::Extension;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use crate::jobs::{top_coins_collection, contract_addresses_collection};
use utoipa::ToSchema;

/// Request to trigger top coins collection
#[derive(Debug, Deserialize, ToSchema)]
pub struct CollectTopCoinsRequest {
    /// Number of top coins to collect (default: 100, max: 250)
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    100
}

/// Response from top coins collection
#[derive(Debug, Serialize, ToSchema)]
pub struct CollectTopCoinsResponse {
    /// Whether the collection was successful
    pub success: bool,
    /// Number of coins collected
    pub coins_collected: usize,
    /// Number of new assets created
    pub assets_created: usize,
    /// Number of existing assets updated
    pub assets_updated: usize,
    /// Number of ranking records created
    pub rankings_created: usize,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Manually trigger top coins collection
///
/// Fetches top N coins from CoinPaprika and stores them in the database.
/// This endpoint allows manual triggering of the scheduled job.
#[utoipa::path(
    post,
    path = "/api/v1/jobs/collect-top-coins",
    request_body = CollectTopCoinsRequest,
    responses(
        (status = 200, description = "Collection completed", body = CollectTopCoinsResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = []),
        ("oauth2_client_credentials" = []),
        ("oauth2_authorization_code" = [])
    ),
    tag = "jobs"
)]
async fn collect_top_coins_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<KeycloakToken<String>>,
    Json(request): Json<CollectTopCoinsRequest>,
) -> Json<CollectTopCoinsResponse> {
    // Validate limit
    let limit = if request.limit == 0 || request.limit > 250 {
        100 // Use default if invalid
    } else {
        request.limit
    };

    tracing::info!("Manual top coins collection triggered with limit={}", limit);

    match top_coins_collection::collect_top_coins(&db, limit).await {
        Ok(result) => {
            tracing::info!(
                "Top coins collection completed: success={}, coins_collected={}, assets_created={}, assets_updated={}, rankings_created={}",
                result.success,
                result.coins_collected,
                result.assets_created,
                result.assets_updated,
                result.rankings_created
            );

            Json(CollectTopCoinsResponse {
                success: result.success,
                coins_collected: result.coins_collected,
                assets_created: result.assets_created,
                assets_updated: result.assets_updated,
                rankings_created: result.rankings_created,
                error: result.error,
            })
        }
        Err(e) => {
            tracing::error!("Top coins collection failed: {}", e);
            Json(CollectTopCoinsResponse {
                success: false,
                coins_collected: 0,
                assets_created: 0,
                assets_updated: 0,
                rankings_created: 0,
                error: Some(format!("Collection failed: {}", e)),
            })
        }
    }
}

/// Request to trigger contract addresses collection
#[derive(Debug, Deserialize, ToSchema)]
pub struct CollectContractAddressesRequest {
    /// Optional limit on number of assets to process (for testing/rate limiting)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// Response from contract addresses collection
#[derive(Debug, Serialize, ToSchema)]
pub struct CollectContractAddressesResponse {
    /// Whether the collection was successful
    pub success: bool,
    /// Number of assets processed
    pub assets_processed: usize,
    /// Number of new contracts created
    pub contracts_created: usize,
    /// Number of existing contracts updated
    pub contracts_updated: usize,
    /// Number of assets skipped
    pub assets_skipped: usize,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Manually trigger contract addresses collection
///
/// Fetches contract addresses for assets from CoinPaprika and stores them in the database.
/// This endpoint allows manual triggering of the scheduled job.
#[utoipa::path(
    post,
    path = "/api/v1/jobs/collect-contract-addresses",
    request_body = CollectContractAddressesRequest,
    responses(
        (status = 200, description = "Collection completed", body = CollectContractAddressesResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = []),
        ("oauth2_client_credentials" = []),
        ("oauth2_authorization_code" = [])
    ),
    tag = "jobs"
)]
async fn collect_contract_addresses_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<KeycloakToken<String>>,
    Json(request): Json<CollectContractAddressesRequest>,
) -> Json<CollectContractAddressesResponse> {
    tracing::info!("Manual contract addresses collection triggered with limit={:?}", request.limit);

    match contract_addresses_collection::collect_contract_addresses(&db, request.limit).await {
        Ok(result) => {
            tracing::info!(
                "Contract addresses collection completed: success={}, assets_processed={}, contracts_created={}, contracts_updated={}, assets_skipped={}",
                result.success,
                result.assets_processed,
                result.contracts_created,
                result.contracts_updated,
                result.assets_skipped
            );

            Json(CollectContractAddressesResponse {
                success: result.success,
                assets_processed: result.assets_processed,
                contracts_created: result.contracts_created,
                contracts_updated: result.contracts_updated,
                assets_skipped: result.assets_skipped,
                error: result.error,
            })
        }
        Err(e) => {
            tracing::error!("Contract addresses collection failed: {}", e);
            Json(CollectContractAddressesResponse {
                success: false,
                assets_processed: 0,
                contracts_created: 0,
                contracts_updated: 0,
                assets_skipped: 0,
                error: Some(format!("Collection failed: {}", e)),
            })
        }
    }
}

/// Create router for job endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/api/v1/jobs/collect-top-coins", post(collect_top_coins_handler))
        .route("/api/v1/jobs/collect-contract-addresses", post(collect_contract_addresses_handler))
}
