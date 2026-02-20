use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
    Condition,
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::entities::{asset_contracts, evm_tokens};
use super::error::ApiError;

// === Request / Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct EvmTokenResponse {
    pub id: Uuid,
    /// EVM chain identifier, e.g. "ethereum", "arbitrum", "bsc"
    pub chain: String,
    /// Token symbol, e.g. "USDC"
    pub symbol: String,
    /// ERC-20 contract address
    pub contract_address: String,
    /// Whether this token is included during account sync
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<evm_tokens::Model> for EvmTokenResponse {
    fn from(m: evm_tokens::Model) -> Self {
        Self {
            id: m.id,
            chain: m.chain,
            symbol: m.symbol,
            contract_address: m.contract_address,
            is_active: m.is_active,
            created_at: m.created_at.to_rfc3339(),
            updated_at: m.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateEvmTokenRequest {
    /// EVM chain identifier (e.g. "ethereum", "arbitrum", "optimism", "base", "bsc")
    pub chain: String,
    /// Token symbol (e.g. "USDC")
    pub symbol: String,
    /// ERC-20 contract address (checksummed hex)
    pub contract_address: String,
    /// Whether to include this token during account sync (default: true)
    #[serde(default = "default_true")]
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateEvmTokenRequest {
    /// Token symbol
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// Whether this token is included during account sync
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListEvmTokensQuery {
    /// Filter by chain (e.g. "ethereum")
    pub chain: Option<String>,
    /// Filter by active status
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncFromContractsResponse {
    /// Number of new tokens inserted
    pub inserted: usize,
    /// Number of tokens that already existed and were skipped
    pub skipped: usize,
}

fn default_true() -> bool {
    true
}

// === Handlers ===

/// List EVM tokens
///
/// Returns the list of ERC-20 tokens that the EVM connector checks during account sync.
/// Results can be filtered by chain and/or active status.
#[utoipa::path(
    get,
    path = "/api/v1/evm-tokens",
    params(ListEvmTokensQuery),
    responses(
        (status = 200, description = "List of EVM tokens", body = Vec<EvmTokenResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-tokens"
)]
pub async fn list_evm_tokens_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Query(q): Query<ListEvmTokensQuery>,
) -> Result<Json<Vec<EvmTokenResponse>>, ApiError> {
    let mut condition = Condition::all();
    if let Some(chain) = q.chain {
        condition = condition.add(evm_tokens::Column::Chain.eq(chain));
    }
    if let Some(is_active) = q.is_active {
        condition = condition.add(evm_tokens::Column::IsActive.eq(is_active));
    }

    let rows = evm_tokens::Entity::find()
        .filter(condition)
        .all(&db)
        .await?;

    Ok(Json(rows.into_iter().map(|r| r.into()).collect()))
}

/// Get an EVM token by ID
#[utoipa::path(
    get,
    path = "/api/v1/evm-tokens/{token_id}",
    params(
        ("token_id" = Uuid, Path, description = "EVM token ID")
    ),
    responses(
        (status = 200, description = "EVM token", body = EvmTokenResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-tokens"
)]
pub async fn get_evm_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(token_id): Path<Uuid>,
) -> Result<Json<EvmTokenResponse>, ApiError> {
    let row = evm_tokens::Entity::find_by_id(token_id)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    Ok(Json(row.into()))
}

/// Create a new EVM token
///
/// Adds a new ERC-20 token address that the EVM connector will check during account sync.
#[utoipa::path(
    post,
    path = "/api/v1/evm-tokens",
    request_body = CreateEvmTokenRequest,
    responses(
        (status = 201, description = "Token created", body = EvmTokenResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 409, description = "Token already exists for this chain/address"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-tokens"
)]
pub async fn create_evm_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Json(req): Json<CreateEvmTokenRequest>,
) -> Result<(StatusCode, Json<EvmTokenResponse>), ApiError> {
    if req.chain.is_empty() {
        return Err(ApiError::BadRequest("chain is required".to_string()));
    }
    if req.symbol.is_empty() {
        return Err(ApiError::BadRequest("symbol is required".to_string()));
    }
    if req.contract_address.is_empty() {
        return Err(ApiError::BadRequest("contract_address is required".to_string()));
    }

    // Check for duplicate (chain, contract_address)
    let existing = evm_tokens::Entity::find()
        .filter(
            Condition::all()
                .add(evm_tokens::Column::Chain.eq(&req.chain))
                .add(evm_tokens::Column::ContractAddress.eq(&req.contract_address)),
        )
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(ApiError::Conflict(format!(
            "Token {} already exists on chain {}",
            req.contract_address, req.chain
        )));
    }

    let new_token = evm_tokens::ActiveModel {
        id: Set(Uuid::new_v4()),
        chain: Set(req.chain),
        symbol: Set(req.symbol),
        contract_address: Set(req.contract_address),
        is_active: Set(req.is_active),
        created_at: Set(Utc::now().into()),
        updated_at: Set(Utc::now().into()),
    };

    let row = new_token.insert(&db).await?;

    Ok((StatusCode::CREATED, Json(row.into())))
}

/// Update an EVM token
///
/// Update the symbol or active status of an existing EVM token entry.
#[utoipa::path(
    put,
    path = "/api/v1/evm-tokens/{token_id}",
    params(
        ("token_id" = Uuid, Path, description = "EVM token ID")
    ),
    request_body = UpdateEvmTokenRequest,
    responses(
        (status = 200, description = "Token updated", body = EvmTokenResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-tokens"
)]
pub async fn update_evm_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(token_id): Path<Uuid>,
    Json(req): Json<UpdateEvmTokenRequest>,
) -> Result<Json<EvmTokenResponse>, ApiError> {
    let row = evm_tokens::Entity::find_by_id(token_id)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    let mut active: evm_tokens::ActiveModel = row.into();

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

/// Delete an EVM token
///
/// Permanently removes an EVM token entry. The built-in fallback list is unaffected.
#[utoipa::path(
    delete,
    path = "/api/v1/evm-tokens/{token_id}",
    params(
        ("token_id" = Uuid, Path, description = "EVM token ID")
    ),
    responses(
        (status = 204, description = "Token deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-tokens"
)]
pub async fn delete_evm_token_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
    Path(token_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let row = evm_tokens::Entity::find_by_id(token_id)
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    let active: evm_tokens::ActiveModel = row.into();
    active.delete(&db).await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Sync EVM tokens from asset_contracts
///
/// Auto-populates the `evm_tokens` table from the `asset_contracts` table for all supported
/// EVM chains.  The `asset_contracts` table is kept up to date by the nightly
/// `fetch_all_coins` background job (CoinPaprika data).
///
/// Only new contracts are inserted; existing rows are not modified.
#[utoipa::path(
    post,
    path = "/api/v1/evm-tokens/sync-from-contracts",
    responses(
        (status = 200, description = "Sync result", body = SyncFromContractsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "evm-tokens"
)]
pub async fn sync_tokens_from_contracts_handler(
    State(db): State<DatabaseConnection>,
    Extension(_token): Extension<axum_keycloak_auth::decode::KeycloakToken<String>>,
) -> Result<Json<SyncFromContractsResponse>, ApiError> {
    use crate::connectors::evm::EvmChain;

    // Derive the supported chain names from EvmChain::all() – single source of truth
    let supported_chains: Vec<String> = EvmChain::all().iter().map(|c| c.name().to_string()).collect();

    // Build an OR filter for all supported chains
    let chain_condition = supported_chains.iter().fold(Condition::any(), |cond, chain| {
        cond.add(asset_contracts::Column::Chain.eq(chain.as_str()))
    });

    // Fetch all asset_contracts rows for supported chains
    let contracts = asset_contracts::Entity::find()
        .filter(chain_condition)
        .all(&db)
        .await?;

    tracing::info!(
        "Found {} asset_contracts rows for supported chains",
        contracts.len()
    );

    // Batch-fetch all referenced assets upfront to avoid N+1 queries
    let asset_ids: Vec<Uuid> = contracts.iter().map(|c| c.asset_id).collect();
    let assets: std::collections::HashMap<Uuid, String> =
        crate::entities::assets::Entity::find()
            .filter(crate::entities::assets::Column::Id.is_in(asset_ids))
            .all(&db)
            .await?
            .into_iter()
            .map(|a| (a.id, a.symbol))
            .collect();

    // Fetch existing evm_tokens (chain, contract_address) to detect duplicates
    let existing: std::collections::HashSet<(String, String)> = evm_tokens::Entity::find()
        .all(&db)
        .await?
        .into_iter()
        .map(|r| (r.chain, r.contract_address))
        .collect();

    let mut inserted = 0usize;
    let mut skipped = 0usize;

    for contract in contracts {
        // Skip unsupported chains (defensive check - should never trigger due to DB filter above)
        if !supported_chains.contains(&contract.chain) {
            continue;
        }

        let key = (contract.chain.clone(), contract.contract_address.clone());
        if existing.contains(&key) {
            skipped += 1;
            continue;
        }

        // Look up symbol from pre-fetched asset map
        let asset_symbol = match assets.get(&contract.asset_id) {
            Some(s) => s.clone(),
            None => {
                tracing::warn!(
                    "asset_id {} not found for contract {}, skipping",
                    contract.asset_id,
                    contract.contract_address
                );
                skipped += 1;
                continue;
            }
        };

        let new_token = evm_tokens::ActiveModel {
            id: Set(Uuid::new_v4()),
            chain: Set(contract.chain),
            symbol: Set(asset_symbol),
            contract_address: Set(contract.contract_address),
            is_active: Set(true),
            created_at: Set(Utc::now().into()),
            updated_at: Set(Utc::now().into()),
        };

        match new_token.insert(&db).await {
            Ok(_) => inserted += 1,
            Err(e) => {
                tracing::warn!("Failed to insert token from contract: {}", e);
                skipped += 1;
            }
        }
    }

    tracing::info!(
        "sync-from-contracts completed: {} inserted, {} skipped",
        inserted,
        skipped
    );

    Ok(Json(SyncFromContractsResponse { inserted, skipped }))
}

/// Create router for evm-tokens endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route(
            "/api/v1/evm-tokens",
            get(list_evm_tokens_handler).post(create_evm_token_handler),
        )
        .route(
            "/api/v1/evm-tokens/{token_id}",
            get(get_evm_token_handler)
                .put(update_evm_token_handler)
                .delete(delete_evm_token_handler),
        )
        .route(
            "/api/v1/evm-tokens/sync-from-contracts",
            post(sync_tokens_from_contracts_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_request_default_is_active() {
        let json = r#"{"chain":"ethereum","symbol":"USDC","contract_address":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"}"#;
        let req: CreateEvmTokenRequest = serde_json::from_str(json).unwrap();
        assert!(req.is_active, "is_active should default to true");
    }

    #[test]
    fn test_create_request_explicit_inactive() {
        let json = r#"{"chain":"ethereum","symbol":"USDC","contract_address":"0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48","is_active":false}"#;
        let req: CreateEvmTokenRequest = serde_json::from_str(json).unwrap();
        assert!(!req.is_active);
    }

    #[test]
    fn test_update_request_partial() {
        let json = r#"{"is_active":false}"#;
        let req: UpdateEvmTokenRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.is_active, Some(false));
        assert!(req.symbol.is_none());
    }
}
