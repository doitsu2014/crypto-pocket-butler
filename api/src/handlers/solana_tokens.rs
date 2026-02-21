use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Extension, Router,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, Condition,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::entities::solana_tokens;
use super::error::ApiError;

// === Request / Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SolanaTokenResponse {
    pub id: Uuid,
    /// Token symbol, e.g. "USDC", "BONK"
    pub symbol: String,
    /// SPL token mint address (Base58 encoded)
    pub mint_address: String,
    /// Whether this token is included during account sync
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<solana_tokens::Model> for SolanaTokenResponse {
    fn from(m: solana_tokens::Model) -> Self {
        Self {
            id: m.id,
            symbol: m.symbol,
            mint_address: m.mint_address,
            is_active: m.is_active,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateSolanaTokenRequest {
    /// Token symbol (e.g. "USDC", "BONK")
    pub symbol: String,
    /// SPL token mint address (Base58 encoded)
    pub mint_address: String,
    /// Whether to include this token during account sync (default: true)
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateSolanaTokenRequest {
    /// Token symbol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Whether this token is included during account sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListSolanaTokensQuery {
    /// Filter by active status
    pub is_active: Option<bool>,
}

fn default_true() -> bool {
    true
}

// === Handlers ===

/// List Solana tokens
///
/// Returns the list of SPL tokens that the Solana connector checks during account sync.
#[utoipa::path(
    get,
    path = "/api/v1/solana-tokens",
    params(ListSolanaTokensQuery),
    responses(
        (status = 200, description = "List of Solana tokens", body = Vec<SolanaTokenResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "solana-tokens"
)]
pub async fn list_solana_tokens_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Query(q): Query<ListSolanaTokensQuery>,
) -> Result<Json<Vec<SolanaTokenResponse>>, ApiError> {
    let mut condition = Condition::all();
    if let Some(is_active) = q.is_active {
        condition = condition.add(solana_tokens::Column::IsActive.eq(is_active));
    }

    let rows = solana_tokens::Entity::find()
        .filter(condition)
        .all(&db)
        .await?;

    Ok(Json(rows.into_iter().map(|r| r.into()).collect()))
}

/// Get a Solana token by ID
#[utoipa::path(
    get,
    path = "/api/v1/solana-tokens/{token_id}",
    params(
        ("token_id" = Uuid, Path, description = "Solana token ID")
    ),
    responses(
        (status = 200, description = "Solana token", body = SolanaTokenResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "solana-tokens"
)]
pub async fn get_solana_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(token_id): Path<Uuid>,
) -> Result<Json<SolanaTokenResponse>, ApiError> {
    let row = solana_tokens::Entity::find_by_id(token_id)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(row.into()))
}

/// Create a new Solana token
///
/// Adds a new SPL token mint address that the Solana connector will check during account sync.
#[utoipa::path(
    post,
    path = "/api/v1/solana-tokens",
    request_body = CreateSolanaTokenRequest,
    responses(
        (status = 201, description = "Token created", body = SolanaTokenResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Token with this mint address already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "solana-tokens"
)]
pub async fn create_solana_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Json(req): Json<CreateSolanaTokenRequest>,
) -> Result<(StatusCode, Json<SolanaTokenResponse>), ApiError> {
    if req.symbol.is_empty() {
        return Err(ApiError::BadRequest("symbol is required".to_string()));
    }
    if req.mint_address.is_empty() {
        return Err(ApiError::BadRequest("mint_address is required".to_string()));
    }

    // Check for duplicate mint_address
    let existing = solana_tokens::Entity::find()
        .filter(solana_tokens::Column::MintAddress.eq(&req.mint_address))
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(ApiError::Conflict(format!(
            "Token with mint address {} already exists",
            req.mint_address
        )));
    }

    let new_token = solana_tokens::ActiveModel {
        id: Set(Uuid::new_v4()),
        symbol: Set(req.symbol),
        mint_address: Set(req.mint_address),
        is_active: Set(req.is_active),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
    };

    let row = new_token.insert(&db).await?;

    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Update a Solana token
///
/// Update the symbol or active status of an existing Solana token entry.
#[utoipa::path(
    put,
    path = "/api/v1/solana-tokens/{token_id}",
    params(
        ("token_id" = Uuid, Path, description = "Solana token ID")
    ),
    request_body = UpdateSolanaTokenRequest,
    responses(
        (status = 200, description = "Token updated", body = SolanaTokenResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "solana-tokens"
)]
pub async fn update_solana_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(token_id): Path<Uuid>,
    Json(req): Json<UpdateSolanaTokenRequest>,
) -> Result<Json<SolanaTokenResponse>, ApiError> {
    let row = solana_tokens::Entity::find_by_id(token_id)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    let mut active: solana_tokens::ActiveModel = row.into();

    if let Some(symbol) = req.symbol {
        active.symbol = Set(symbol);
    }
    if let Some(is_active) = req.is_active {
        active.is_active = Set(is_active);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&db).await?;

    Ok(Json(updated.into()))
}

/// Delete a Solana token
///
/// Permanently removes a Solana token entry. The built-in fallback list is unaffected.
#[utoipa::path(
    delete,
    path = "/api/v1/solana-tokens/{token_id}",
    params(
        ("token_id" = Uuid, Path, description = "Solana token ID")
    ),
    responses(
        (status = 204, description = "Token deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "solana-tokens"
)]
pub async fn delete_solana_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(token_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let row = solana_tokens::Entity::find_by_id(token_id)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    let active: solana_tokens::ActiveModel = row.into();
    active.delete(&db).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create router for solana-tokens endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route(
            "/api/v1/solana-tokens",
            get(list_solana_tokens_handler).post(create_solana_token_handler),
        )
        .route(
            "/api/v1/solana-tokens/{token_id}",
            get(get_solana_token_handler)
                .put(update_solana_token_handler)
                .delete(delete_solana_token_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_request_default_is_active() {
        let json = r#"{"symbol":"USDC","mint_address":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"}"#;
        let req: CreateSolanaTokenRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_active, "is_active should default to true");
    }

    #[test]
    fn test_create_request_explicit_inactive() {
        let json = r#"{"symbol":"USDC","mint_address":"EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v","is_active":false}"#;
        let req: CreateSolanaTokenRequest = serde_json::from_str(json).unwrap();
        assert!(!req.is_active);
    }

    #[test]
    fn test_update_request_partial() {
        let json = r#"{"is_active":false}"#;
        let req: UpdateSolanaTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.is_active, Some(false));
        assert!(req.symbol.is_none());
    }
}
