use axum::{extract::State, response::Json, routing::{get, post}, Router};
use axum_keycloak_auth::decode::KeycloakToken;
use axum::Extension;
use sea_orm::DatabaseConnection;
use serde::Serialize;
use crate::application::jobs::fetch_all_coins;
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

/// Status of a single Apalis cron worker
#[derive(Debug, Serialize, ToSchema)]
pub struct ApalisWorkerStatus {
    /// Unique worker name
    pub name: String,
    /// Whether the worker is enabled
    pub enabled: bool,
    /// Cron expression used to schedule the worker (6-field: sec min hour dom month dow)
    pub cron: String,
}

/// Response from the Apalis jobs status endpoint
#[derive(Debug, Serialize, ToSchema)]
pub struct ApalisJobsStatusResponse {
    /// List of registered Apalis cron workers and their configuration
    pub workers: Vec<ApalisWorkerStatus>,
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

/// Get Apalis job workers status
///
/// Returns the configuration of all Apalis cron workers: whether each worker is enabled
/// and its cron schedule. Values are read from the `APALIS_*` environment variables.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/status",
    responses(
        (status = 200, description = "Apalis workers status", body = ApalisJobsStatusResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - requires administrator role")
    ),
    security(
        ("bearer_auth" = []),
        ("oauth2_client_credentials" = []),
        ("oauth2_authorization_code" = [])
    ),
    tag = "jobs"
)]
pub async fn get_jobs_status_handler(
    Extension(_token): Extension<KeycloakToken<String>>,
) -> Json<ApalisJobsStatusResponse> {
    let fetch_enabled = std::env::var("APALIS_FETCH_ALL_COINS_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    let fetch_cron = std::env::var("APALIS_FETCH_ALL_COINS_CRON")
        .unwrap_or_else(|_| "0 */15 * * * *".to_string());

    let snapshot_enabled = std::env::var("APALIS_EOD_SNAPSHOT_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    let snapshot_cron = std::env::var("APALIS_EOD_SNAPSHOT_CRON")
        .unwrap_or_else(|_| "0 0 23 * * *".to_string());

    Json(ApalisJobsStatusResponse {
        workers: vec![
            ApalisWorkerStatus {
                name: "apalis-fetch-all-coins".to_string(),
                enabled: fetch_enabled,
                cron: fetch_cron,
            },
            ApalisWorkerStatus {
                name: "apalis-eod-snapshot".to_string(),
                enabled: snapshot_enabled,
                cron: snapshot_cron,
            },
        ],
    })
}

/// Create router for job endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/api/v1/jobs/fetch-all-coins", post(fetch_all_coins_handler))
        .route("/api/v1/jobs/status", get(get_jobs_status_handler))
}
