use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::post,
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use chrono::NaiveDate;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{portfolios, users};
use crate::jobs::portfolio_snapshot;

// === Request/Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSnapshotRequest {
    /// Snapshot type (default: "manual")
    #[serde(default = "default_snapshot_type")]
    pub snapshot_type: String,
    /// Optional snapshot date (ISO 8601 format, defaults to today)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_date: Option<String>,
}

fn default_snapshot_type() -> String {
    "manual".to_string()
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SnapshotResultResponse {
    pub portfolio_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<Uuid>,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub holdings_count: usize,
    pub total_value_usd: String,
}

impl From<portfolio_snapshot::SnapshotResult> for SnapshotResultResponse {
    fn from(result: portfolio_snapshot::SnapshotResult) -> Self {
        Self {
            portfolio_id: result.portfolio_id,
            snapshot_id: result.snapshot_id,
            success: result.success,
            error: result.error,
            holdings_count: result.holdings_count,
            total_value_usd: result.total_value_usd,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAllSnapshotsResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<SnapshotResultResponse>,
}

// === Helper Functions ===

/// Get or create user in database from Keycloak token
async fn get_or_create_user(
    db: &DatabaseConnection,
    token: &KeycloakToken<String>,
) -> Result<users::Model, Response> {
    let keycloak_user_id = &token.subject;

    // Try to find existing user
    if let Some(user) = users::Entity::find()
        .filter(users::Column::KeycloakUserId.eq(keycloak_user_id))
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?
    {
        return Ok(user);
    }

    // Create new user if not found
    let new_user = users::ActiveModel {
        keycloak_user_id: sea_orm::ActiveValue::Set(keycloak_user_id.clone()),
        email: sea_orm::ActiveValue::Set(Some(token.extra.email.email.clone())),
        preferred_username: sea_orm::ActiveValue::Set(Some(
            token.extra.profile.preferred_username.clone(),
        )),
        ..Default::default()
    };

    new_user
        .insert(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create user: {}", e),
            )
                .into_response()
        })
}

/// Check if portfolio belongs to user
async fn check_portfolio_ownership(
    db: &DatabaseConnection,
    portfolio_id: Uuid,
    user_id: Uuid,
) -> Result<(), Response> {
    let portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .one(db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (StatusCode::NOT_FOUND, "Portfolio not found").into_response()
        })?;

    if portfolio.user_id != user_id {
        return Err((StatusCode::FORBIDDEN, "Access denied").into_response());
    }

    Ok(())
}

// === API Handlers ===

/// Create a snapshot for a specific portfolio
///
/// Creates a snapshot of the current portfolio holdings and valuation.
/// Useful for manual snapshots or testing.
#[utoipa::path(
    post,
    path = "/api/v1/portfolios/{portfolio_id}/snapshots",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID")
    ),
    request_body = CreateSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot created successfully", body = SnapshotResultResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - portfolio does not belong to user"),
        (status = 404, description = "Portfolio not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "snapshots"
)]
async fn create_portfolio_snapshot_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<SnapshotResultResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await?;

    // Verify portfolio belongs to user
    check_portfolio_ownership(&db, portfolio_id, user.id).await?;

    // Parse snapshot date if provided
    let snapshot_date = if let Some(date_str) = request.snapshot_date {
        Some(NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid date format: {}. Expected YYYY-MM-DD", e),
            )
                .into_response()
        })?)
    } else {
        None
    };

    // Create snapshot
    let result = portfolio_snapshot::create_portfolio_snapshot(
        &db,
        portfolio_id,
        snapshot_date,
        &request.snapshot_type,
    )
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create snapshot: {}", e),
        )
            .into_response()
    })?;

    Ok(Json(result.into()))
}

/// Create snapshots for all portfolios owned by the authenticated user
///
/// Creates snapshots for all portfolios belonging to the authenticated user.
/// Useful for manual snapshot creation or testing.
#[utoipa::path(
    post,
    path = "/api/v1/snapshots/create-all",
    request_body = CreateSnapshotRequest,
    responses(
        (status = 200, description = "Snapshots created", body = CreateAllSnapshotsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "snapshots"
)]
async fn create_all_user_snapshots_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<CreateAllSnapshotsResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await?;

    // Parse snapshot date if provided
    let snapshot_date = if let Some(date_str) = request.snapshot_date {
        Some(NaiveDate::parse_from_str(&date_str, "%Y-%m-%d").map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid date format: {}. Expected YYYY-MM-DD", e),
            )
                .into_response()
        })?)
    } else {
        None
    };

    // Get all user's portfolios
    let user_portfolios = portfolios::Entity::find()
        .filter(portfolios::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?;

    // Create snapshots for each portfolio
    let mut results = Vec::new();
    for portfolio in user_portfolios {
        match portfolio_snapshot::create_portfolio_snapshot(
            &db,
            portfolio.id,
            snapshot_date,
            &request.snapshot_type,
        )
        .await
        {
            Ok(result) => results.push(result),
            Err(e) => {
                tracing::error!("Failed to create snapshot for portfolio {}: {}", portfolio.id, e);
                results.push(portfolio_snapshot::SnapshotResult {
                    portfolio_id: portfolio.id,
                    snapshot_id: None,
                    success: false,
                    error: Some(format!("Snapshot failed: {}", e)),
                    holdings_count: 0,
                    total_value_usd: "0".to_string(),
                });
            }
        }
    }

    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = total - successful;

    Ok(Json(CreateAllSnapshotsResponse {
        total,
        successful,
        failed,
        results: results.into_iter().map(|r| r.into()).collect(),
    }))
}

/// Create router for snapshot endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route(
            "/api/v1/portfolios/{portfolio_id}/snapshots",
            post(create_portfolio_snapshot_handler),
        )
        .route(
            "/api/v1/snapshots/create-all",
            post(create_all_user_snapshots_handler),
        )
}
