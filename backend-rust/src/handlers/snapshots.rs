use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use chrono::NaiveDate;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{portfolios, snapshots};
use crate::helpers::auth::get_or_create_user;
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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SnapshotResponse {
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub snapshot_date: String, // ISO 8601 date
    pub snapshot_type: String,
    pub total_value_usd: String,
    pub holdings: serde_json::Value, // JSON holdings data
    pub metadata: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allocation_id: Option<Uuid>, // Reference to portfolio_allocations
    pub created_at: String, // ISO 8601 datetime
}

impl From<snapshots::Model> for SnapshotResponse {
    fn from(model: snapshots::Model) -> Self {
        Self {
            id: model.id,
            portfolio_id: model.portfolio_id,
            snapshot_date: model.snapshot_date.to_string(),
            snapshot_type: model.snapshot_type,
            total_value_usd: model.total_value_usd.to_string(),
            holdings: model.holdings,
            metadata: model.metadata,
            allocation_id: model.allocation_id,
            created_at: model.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListSnapshotsQuery {
    /// Filter by start date (ISO 8601 format, inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<String>,
    /// Filter by end date (ISO 8601 format, inclusive)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<String>,
    /// Filter by snapshot type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListSnapshotsResponse {
    pub portfolio_id: Uuid,
    pub snapshots: Vec<SnapshotResponse>,
    pub total_count: usize,
}

// === Helper Functions ===

/// Get or create user in database from Keycloak token

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

/// Get snapshots for a specific portfolio
///
/// Retrieves all snapshots for the specified portfolio, with optional date and type filtering.
#[utoipa::path(
    get,
    path = "/api/v1/portfolios/{portfolio_id}/snapshots",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID"),
        ("start_date" = Option<String>, Query, description = "Start date filter (YYYY-MM-DD, inclusive)"),
        ("end_date" = Option<String>, Query, description = "End date filter (YYYY-MM-DD, inclusive)"),
        ("snapshot_type" = Option<String>, Query, description = "Snapshot type filter (eod, manual, hourly)")
    ),
    responses(
        (status = 200, description = "Snapshots retrieved successfully", body = ListSnapshotsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - portfolio does not belong to user"),
        (status = 404, description = "Portfolio not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "snapshots"
)]
async fn list_portfolio_snapshots_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<ListSnapshotsQuery>,
) -> Result<Json<ListSnapshotsResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    // Verify portfolio belongs to user
    check_portfolio_ownership(&db, portfolio_id, user.id).await?;

    // Build query
    let mut snapshot_query = snapshots::Entity::find()
        .filter(snapshots::Column::PortfolioId.eq(portfolio_id));

    // Apply date filters if provided
    if let Some(start_date_str) = query.start_date {
        let start_date = NaiveDate::parse_from_str(&start_date_str, "%Y-%m-%d").map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid start_date format: {}. Expected YYYY-MM-DD", e),
            )
                .into_response()
        })?;
        snapshot_query = snapshot_query.filter(snapshots::Column::SnapshotDate.gte(start_date));
    }

    if let Some(end_date_str) = query.end_date {
        let end_date = NaiveDate::parse_from_str(&end_date_str, "%Y-%m-%d").map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                format!("Invalid end_date format: {}. Expected YYYY-MM-DD", e),
            )
                .into_response()
        })?;
        snapshot_query = snapshot_query.filter(snapshots::Column::SnapshotDate.lte(end_date));
    }

    // Apply type filter if provided
    if let Some(snapshot_type) = query.snapshot_type {
        snapshot_query = snapshot_query.filter(snapshots::Column::SnapshotType.eq(snapshot_type));
    }

    // Order by date descending (most recent first)
    snapshot_query = snapshot_query.order_by_desc(snapshots::Column::SnapshotDate);

    // Execute query
    let snapshot_models = snapshot_query.all(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    let total_count = snapshot_models.len();
    let snapshots: Vec<SnapshotResponse> = snapshot_models
        .into_iter()
        .map(|model| model.into())
        .collect();

    Ok(Json(ListSnapshotsResponse {
        portfolio_id,
        snapshots,
        total_count,
    }))
}

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

    tag = "snapshots"
)]
async fn create_portfolio_snapshot_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<SnapshotResultResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

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

    tag = "snapshots"
)]
async fn create_all_user_snapshots_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Json(request): Json<CreateSnapshotRequest>,
) -> Result<Json<CreateAllSnapshotsResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

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

/// Get the latest snapshot for a specific portfolio
///
/// Retrieves the most recent snapshot for the specified portfolio based on snapshot_date and created_at.
#[utoipa::path(
    get,
    path = "/api/v1/portfolios/{portfolio_id}/snapshots/latest",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 200, description = "Latest snapshot retrieved successfully", body = SnapshotResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden - portfolio does not belong to user"),
        (status = 404, description = "Portfolio or snapshot not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "snapshots"
)]
async fn get_latest_portfolio_snapshot_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
) -> Result<Json<SnapshotResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    // Verify portfolio belongs to user
    check_portfolio_ownership(&db, portfolio_id, user.id).await?;

    // Get the latest snapshot for this portfolio
    let latest_snapshot = snapshots::Entity::find()
        .filter(snapshots::Column::PortfolioId.eq(portfolio_id))
        .order_by_desc(snapshots::Column::SnapshotDate)
        .order_by_desc(snapshots::Column::CreatedAt)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                "No snapshots found for this portfolio",
            )
                .into_response()
        })?;

    Ok(Json(latest_snapshot.into()))
}

/// Create router for snapshot endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route(
            "/api/v1/portfolios/{portfolio_id}/snapshots",
            get(list_portfolio_snapshots_handler).post(create_portfolio_snapshot_handler),
        )
        .route(
            "/api/v1/portfolios/{portfolio_id}/snapshots/latest",
            get(get_latest_portfolio_snapshot_handler),
        )
        .route(
            "/api/v1/snapshots/create-all",
            post(create_all_user_snapshots_handler),
        )
}
