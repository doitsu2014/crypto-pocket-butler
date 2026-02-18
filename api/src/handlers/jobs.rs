use axum::{extract::State, response::Json, routing::post, Router};
use axum_keycloak_auth::decode::KeycloakToken;
use axum::Extension;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use crate::jobs::fetch_all_coins;
use utoipa::ToSchema;

/// Response from fetch all coins job
#[derive(Debug, Serialize, ToSchema)]
pub struct FetchAllCoinsResponse {
    /// Whether the collection was successful
    pub success: bool,
    /// Number of coins fetched
    pub coins_fetched: usize,
    /// Number of new assets created
    pub assets_created: usize,
    /// Number of existing assets updated
    pub assets_updated: usize,
    /// Number of price records stored
    pub prices_stored: usize,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Manually trigger fetch all coins job
///
/// Fetches all active coins from CoinPaprika in one request and stores them in the database.
/// This endpoint allows manual triggering of the scheduled job.
#[utoipa::path(
    post,
    path = "/api/v1/jobs/fetch-all-coins",
    responses(
        (status = 200, description = "Collection completed", body = FetchAllCoinsResponse),
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
pub async fn fetch_all_coins_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<KeycloakToken<String>>,
) -> Json<FetchAllCoinsResponse> {
    tracing::info!("Manual fetch all coins triggered");

    match fetch_all_coins::fetch_all_coins(&db).await {
        Ok(result) => {
            tracing::info!(
                "Fetch all coins completed: success={}, coins_fetched={}, assets_created={}, assets_updated={}, prices_stored={}",
                result.success,
                result.coins_fetched,
                result.assets_created,
                result.assets_updated,
                result.prices_stored
            );

            Json(FetchAllCoinsResponse {
                success: result.success,
                coins_fetched: result.coins_fetched,
                assets_created: result.assets_created,
                assets_updated: result.assets_updated,
                prices_stored: result.prices_stored,
                error: result.error,
            })
        }
        Err(e) => {
            tracing::error!("Fetch all coins failed: {}", e);
            Json(FetchAllCoinsResponse {
                success: false,
                coins_fetched: 0,
                assets_created: 0,
                assets_updated: 0,
                prices_stored: 0,
                error: Some(format!("Collection failed: {}", e)),
            })
        }
    }
}

/// Create router for job endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/api/v1/jobs/fetch-all-coins", post(fetch_all_coins_handler))
}
