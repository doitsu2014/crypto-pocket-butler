use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::evm_chains;
use super::error::ApiError;

// === Request / Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EvmChainResponse {
    pub id: Uuid,
    /// Unique chain identifier, e.g. "ethereum", "bsc"
    pub chain_id: String,
    /// Human-readable chain name
    pub name: String,
    /// RPC URL used to connect to this chain
    pub rpc_url: String,
    /// Native token symbol, e.g. "ETH", "BNB", "HYPE"
    pub native_symbol: String,
    /// Whether this chain is active for account sync
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<evm_chains::Model> for EvmChainResponse {
    fn from(m: evm_chains::Model) -> Self {
        Self {
            id: m.id,
            chain_id: m.chain_id,
            name: m.name,
            rpc_url: m.rpc_url,
            native_symbol: m.native_symbol,
            is_active: m.is_active,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEvmChainRequest {
    /// Unique chain identifier (e.g. "ethereum", "bsc")
    pub chain_id: String,
    /// Human-readable chain name (e.g. "Ethereum")
    pub name: String,
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Native token symbol (e.g. "ETH", "BNB", "HYPE")
    pub native_symbol: String,
    /// Whether to include this chain during account sync (default: true)
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateEvmChainRequest {
    /// Human-readable chain name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// RPC endpoint URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpc_url: Option<String>,
    /// Native token symbol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_symbol: Option<String>,
    /// Whether this chain is active for account sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

fn default_true() -> bool {
    true
}

// === Handlers ===

/// List EVM chains
///
/// Returns all configured EVM chains including their RPC URLs.
#[utoipa::path(
    get,
    path = "/api/v1/evm-chains",
    responses(
        (status = 200, description = "List of EVM chains", body = Vec<EvmChainResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-chains"
)]
pub async fn list_evm_chains_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
) -> Result<Json<Vec<EvmChainResponse>>, ApiError> {
    let rows = evm_chains::Entity::find().all(&db).await?;
    Ok(Json(rows.into_iter().map(|r| r.into()).collect()))
}

/// Get an EVM chain by ID
#[utoipa::path(
    get,
    path = "/api/v1/evm-chains/{chain_id}",
    params(
        ("chain_id" = Uuid, Path, description = "EVM chain record ID")
    ),
    responses(
        (status = 200, description = "EVM chain", body = EvmChainResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-chains"
)]
pub async fn get_evm_chain_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(chain_uuid): Path<Uuid>,
) -> Result<Json<EvmChainResponse>, ApiError> {
    let row = evm_chains::Entity::find_by_id(chain_uuid)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;
    Ok(Json(row.into()))
}

/// Create a new EVM chain
///
/// Adds a new EVM chain configuration with a custom RPC URL.
#[utoipa::path(
    post,
    path = "/api/v1/evm-chains",
    request_body = CreateEvmChainRequest,
    responses(
        (status = 201, description = "Chain created", body = EvmChainResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Chain ID already exists"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-chains"
)]
pub async fn create_evm_chain_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Json(req): Json<CreateEvmChainRequest>,
) -> Result<(StatusCode, Json<EvmChainResponse>), ApiError> {
    if req.chain_id.is_empty() {
        return Err(ApiError::BadRequest("chain_id is required".to_string()));
    }
    if req.name.is_empty() {
        return Err(ApiError::BadRequest("name is required".to_string()));
    }
    if req.rpc_url.is_empty() {
        return Err(ApiError::BadRequest("rpc_url is required".to_string()));
    }

    // Check for duplicate chain_id
    let existing = evm_chains::Entity::find()
        .filter(evm_chains::Column::ChainId.eq(&req.chain_id))
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(ApiError::Conflict(format!(
            "Chain '{}' already exists",
            req.chain_id
        )));
    }

    let new_chain = evm_chains::ActiveModel {
        id: Set(Uuid::new_v4()),
        chain_id: Set(req.chain_id),
        name: Set(req.name),
        rpc_url: Set(req.rpc_url),
        native_symbol: Set(req.native_symbol),
        is_active: Set(req.is_active),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
    };

    let row = new_chain.insert(&db).await?;
    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Update an EVM chain
///
/// Update the name, RPC URL, or active status of an existing EVM chain.
#[utoipa::path(
    put,
    path = "/api/v1/evm-chains/{chain_id}",
    params(
        ("chain_id" = Uuid, Path, description = "EVM chain record ID")
    ),
    request_body = UpdateEvmChainRequest,
    responses(
        (status = 200, description = "Chain updated", body = EvmChainResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-chains"
)]
pub async fn update_evm_chain_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(chain_uuid): Path<Uuid>,
    Json(req): Json<UpdateEvmChainRequest>,
) -> Result<Json<EvmChainResponse>, ApiError> {
    let row = evm_chains::Entity::find_by_id(chain_uuid)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    let mut active: evm_chains::ActiveModel = row.into();

    if let Some(name) = req.name {
        active.name = Set(name);
    }
    if let Some(rpc_url) = req.rpc_url {
        active.rpc_url = Set(rpc_url);
    }
    if let Some(native_symbol) = req.native_symbol {
        active.native_symbol = Set(native_symbol);
    }
    if let Some(is_active) = req.is_active {
        active.is_active = Set(is_active);
    }
    active.updated_at = Set(Utc::now().into());

    let updated = active.update(&db).await?;
    Ok(Json(updated.into()))
}

/// Delete an EVM chain
///
/// Permanently removes an EVM chain configuration.
#[utoipa::path(
    delete,
    path = "/api/v1/evm-chains/{chain_id}",
    params(
        ("chain_id" = Uuid, Path, description = "EVM chain record ID")
    ),
    responses(
        (status = 204, description = "Chain deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-chains"
)]
pub async fn delete_evm_chain_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(chain_uuid): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let row = evm_chains::Entity::find_by_id(chain_uuid)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    let active: evm_chains::ActiveModel = row.into();
    active.delete(&db).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Create router for evm-chains endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route(
            "/api/v1/evm-chains",
            get(list_evm_chains_handler).post(create_evm_chain_handler),
        )
        .route(
            "/api/v1/evm-chains/{chain_id}",
            get(get_evm_chain_handler)
                .put(update_evm_chain_handler)
                .delete(delete_evm_chain_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_request_default_is_active() {
        let json = r#"{"chain_id":"mychain","name":"My Chain","rpc_url":"https://rpc.example.com","native_symbol":"ETH"}"#;
        let req: CreateEvmChainRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_active, "is_active should default to true");
        assert_eq!(req.native_symbol, "ETH");
    }

    #[test]
    fn test_create_request_explicit_inactive() {
        let json = r#"{"chain_id":"mychain","name":"My Chain","rpc_url":"https://rpc.example.com","native_symbol":"HYPE","is_active":false}"#;
        let req: CreateEvmChainRequest = serde_json::from_str(json).unwrap();
        assert!(!req.is_active);
        assert_eq!(req.native_symbol, "HYPE");
    }

    #[test]
    fn test_update_request_partial() {
        let json = r#"{"rpc_url":"https://new-rpc.example.com"}"#;
        let req: UpdateEvmChainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.rpc_url, Some("https://new-rpc.example.com".to_string()));
        assert!(req.name.is_none());
        assert!(req.is_active.is_none());
    }
}
