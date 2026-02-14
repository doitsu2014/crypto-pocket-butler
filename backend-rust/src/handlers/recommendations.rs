use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    Set,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{portfolios, recommendations};
use crate::helpers::auth::get_or_create_user;

// === Request/Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct RecommendationResponse {
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub status: String,
    pub recommendation_type: String,
    pub rationale: String,
    pub proposed_orders: serde_json::Value,
    pub expected_impact: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
    pub executed_at: Option<String>,
}

impl From<recommendations::Model> for RecommendationResponse {
    fn from(model: recommendations::Model) -> Self {
        Self {
            id: model.id,
            portfolio_id: model.portfolio_id,
            status: model.status,
            recommendation_type: model.recommendation_type,
            rationale: model.rationale,
            proposed_orders: model.proposed_orders,
            expected_impact: model.expected_impact.map(|d| d.to_string()),
            metadata: model.metadata,
            created_at: model.created_at.to_rfc3339(),
            updated_at: model.updated_at.to_rfc3339(),
            executed_at: model.executed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ListRecommendationsResponse {
    pub portfolio_id: Uuid,
    pub recommendations: Vec<RecommendationResponse>,
    pub total_count: usize,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListRecommendationsQuery {
    /// Filter by status (e.g., "pending", "approved", "rejected", "executed")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateRecommendationRequest {
    pub recommendation_type: String,
    pub rationale: String,
    pub proposed_orders: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_impact: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

// === Helper Functions ===

/// Get or create user in database from Keycloak token

// === API Handlers ===

/// List recommendations for a portfolio
#[utoipa::path(
    get,
    path = "/api/v1/portfolios/{portfolio_id}/recommendations",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID"),
        ("status" = Option<String>, Query, description = "Filter by status")
    ),
    responses(
        (status = 200, description = "Recommendations retrieved successfully", body = ListRecommendationsResponse),
        (status = 404, description = "Portfolio not found"),
    ),
    tag = "recommendations"
)]
pub async fn list_portfolio_recommendations(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
    Query(query): Query<ListRecommendationsQuery>,
) -> Result<Json<ListRecommendationsResponse>, Response> {
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    // Verify portfolio ownership
    let _portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .filter(portfolios::Column::UserId.eq(user.id))
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
                "Portfolio not found or access denied",
            )
                .into_response()
        })?;

    // Build query with optional status filter
    let mut query_builder = recommendations::Entity::find()
        .filter(recommendations::Column::PortfolioId.eq(portfolio_id))
        .order_by_desc(recommendations::Column::CreatedAt);

    if let Some(status) = query.status {
        query_builder = query_builder.filter(recommendations::Column::Status.eq(status));
    }

    let recommendations_list = query_builder
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?;

    let total_count = recommendations_list.len();
    let recommendations: Vec<RecommendationResponse> = recommendations_list
        .into_iter()
        .map(RecommendationResponse::from)
        .collect();

    Ok(Json(ListRecommendationsResponse {
        portfolio_id,
        recommendations,
        total_count,
    }))
}

/// Get a specific recommendation by ID
#[utoipa::path(
    get,
    path = "/api/v1/portfolios/{portfolio_id}/recommendations/{recommendation_id}",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID"),
        ("recommendation_id" = Uuid, Path, description = "Recommendation ID")
    ),
    responses(
        (status = 200, description = "Recommendation retrieved successfully", body = RecommendationResponse),
        (status = 404, description = "Recommendation not found"),
    ),
    tag = "recommendations"
)]
pub async fn get_recommendation(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path((portfolio_id, recommendation_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<RecommendationResponse>, Response> {
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    // Verify portfolio ownership
    let _portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .filter(portfolios::Column::UserId.eq(user.id))
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
                "Portfolio not found or access denied",
            )
                .into_response()
        })?;

    // Get recommendation
    let recommendation = recommendations::Entity::find_by_id(recommendation_id)
        .filter(recommendations::Column::PortfolioId.eq(portfolio_id))
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
            (StatusCode::NOT_FOUND, "Recommendation not found").into_response()
        })?;

    Ok(Json(RecommendationResponse::from(recommendation)))
}

/// Create a recommendation for a portfolio (for testing/demo purposes)
#[utoipa::path(
    post,
    path = "/api/v1/portfolios/{portfolio_id}/recommendations",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID")
    ),
    request_body = CreateRecommendationRequest,
    responses(
        (status = 201, description = "Recommendation created successfully", body = RecommendationResponse),
        (status = 404, description = "Portfolio not found"),
    ),
    tag = "recommendations"
)]
pub async fn create_recommendation(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
    Json(payload): Json<CreateRecommendationRequest>,
) -> Result<Response, Response> {
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    // Verify portfolio ownership
    let _portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .filter(portfolios::Column::UserId.eq(user.id))
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
                "Portfolio not found or access denied",
            )
                .into_response()
        })?;

    // Parse expected_impact if provided
    let expected_impact = if let Some(impact_str) = payload.expected_impact {
        Some(
            impact_str
                .parse::<Decimal>()
                .map_err(|e| {
                    (
                        StatusCode::BAD_REQUEST,
                        format!("Invalid expected_impact value: {}", e),
                    )
                        .into_response()
                })?,
        )
    } else {
        None
    };

    let now = Utc::now();
    let new_recommendation = recommendations::ActiveModel {
        portfolio_id: Set(portfolio_id),
        status: Set("pending".to_string()),
        recommendation_type: Set(payload.recommendation_type),
        rationale: Set(payload.rationale),
        proposed_orders: Set(payload.proposed_orders),
        expected_impact: Set(expected_impact),
        metadata: Set(payload.metadata),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    };

    let recommendation = new_recommendation
        .insert(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create recommendation: {}", e),
            )
                .into_response()
        })?;

    let response = RecommendationResponse::from(recommendation);
    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// Generate mock recommendations for a portfolio (demo feature)
#[utoipa::path(
    post,
    path = "/api/v1/portfolios/{portfolio_id}/recommendations/generate",
    params(
        ("portfolio_id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 200, description = "Mock recommendations generated", body = ListRecommendationsResponse),
        (status = 404, description = "Portfolio not found"),
    ),
    tag = "recommendations"
)]
pub async fn generate_mock_recommendations(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(portfolio_id): Path<Uuid>,
) -> Result<Json<ListRecommendationsResponse>, Response> {
    let user = get_or_create_user(&db, &token).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Database error: {}", e),
        )
            .into_response()
    })?;

    // Verify portfolio ownership
    let _portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .filter(portfolios::Column::UserId.eq(user.id))
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
                "Portfolio not found or access denied",
            )
                .into_response()
        })?;

    // Generate mock recommendations
    let now = Utc::now();
    let mock_recommendations = vec![
        recommendations::ActiveModel {
            portfolio_id: Set(portfolio_id),
            status: Set("pending".to_string()),
            recommendation_type: Set("rebalance".to_string()),
            rationale: Set(
                "Portfolio allocation has drifted significantly from target. BTC allocation is 45% (target: 40%), ETH is 25% (target: 30%). Recommend rebalancing to maintain strategic allocation.".to_string(),
            ),
            proposed_orders: Set(json!([
                {
                    "action": "sell",
                    "asset": "BTC",
                    "quantity": "0.05",
                    "estimated_price": "65000",
                    "estimated_value_usd": "3250.00"
                },
                {
                    "action": "buy",
                    "asset": "ETH",
                    "quantity": "1.2",
                    "estimated_price": "3200",
                    "estimated_value_usd": "3840.00"
                }
            ])),
            expected_impact: Set(Some(Decimal::new(590, 2))), // 5.90
            metadata: Set(Some(json!({
                "risk_score": 0.3,
                "confidence": 0.85,
                "drift_percentage": 5.2
            }))),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        },
        recommendations::ActiveModel {
            portfolio_id: Set(portfolio_id),
            status: Set("pending".to_string()),
            recommendation_type: Set("take_profit".to_string()),
            rationale: Set(
                "SOL has appreciated 45% in the past 30 days and is approaching resistance at $150. Consider taking partial profits to secure gains while maintaining exposure.".to_string(),
            ),
            proposed_orders: Set(json!([
                {
                    "action": "sell",
                    "asset": "SOL",
                    "quantity": "50",
                    "estimated_price": "145",
                    "estimated_value_usd": "7250.00"
                }
            ])),
            expected_impact: Set(Some(Decimal::new(7250, 2))), // 72.50
            metadata: Set(Some(json!({
                "risk_score": 0.2,
                "confidence": 0.75,
                "price_appreciation_30d": 0.45,
                "resistance_level": "$150"
            }))),
            created_at: Set(now.into()),
            updated_at: Set(now.into()),
            ..Default::default()
        },
    ];

    let mut created_recommendations = Vec::new();
    for rec in mock_recommendations {
        let created = rec.insert(&db).await.map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create recommendation: {}", e),
            )
                .into_response()
        })?;
        created_recommendations.push(RecommendationResponse::from(created));
    }

    let total_count = created_recommendations.len();

    Ok(Json(ListRecommendationsResponse {
        portfolio_id,
        recommendations: created_recommendations,
        total_count,
    }))
}

// === Router Configuration ===

pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route(
            "/api/v1/portfolios/{portfolio_id}/recommendations",
            get(list_portfolio_recommendations).post(create_recommendation),
        )
        .route(
            "/api/v1/portfolios/{portfolio_id}/recommendations/generate",
            post(generate_mock_recommendations),
        )
        .route(
            "/api/v1/portfolios/{portfolio_id}/recommendations/{recommendation_id}",
            get(get_recommendation),
        )
}
