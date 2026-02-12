use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get, post, put},
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{accounts, users};
use crate::jobs::account_sync;

// === Request/Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateAccountRequest {
    /// Account name
    pub name: String,
    /// Account type: "exchange" or "wallet"
    pub account_type: String,
    /// Exchange name (required if account_type is "exchange")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_name: Option<String>,
    /// Wallet address (required if account_type is "wallet")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    /// API key (for exchange accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// API secret (for exchange accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    /// Passphrase (for exchange accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateAccountRequest {
    /// Account name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Whether account is active
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_active: Option<bool>,
    /// API key (for exchange accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// API secret (for exchange accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_secret: Option<String>,
    /// Passphrase (for exchange accounts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub account_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exchange_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wallet_address: Option<String>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<accounts::Model> for AccountResponse {
    fn from(account: accounts::Model) -> Self {
        Self {
            id: account.id,
            user_id: account.user_id,
            name: account.name,
            account_type: account.account_type,
            exchange_name: account.exchange_name,
            wallet_address: account.wallet_address,
            is_active: account.is_active,
            last_synced_at: account.last_synced_at.map(|dt| dt.to_rfc3339()),
            created_at: account.created_at.to_rfc3339(),
            updated_at: account.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncAccountRequest {
    /// Account ID to sync
    pub account_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncResultResponse {
    pub account_id: Uuid,
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub holdings_count: usize,
}

impl From<account_sync::SyncResult> for SyncResultResponse {
    fn from(result: account_sync::SyncResult) -> Self {
        Self {
            account_id: result.account_id,
            success: result.success,
            error: result.error,
            holdings_count: result.holdings_count,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncAllAccountsResponse {
    pub total: usize,
    pub successful: usize,
    pub failed: usize,
    pub results: Vec<SyncResultResponse>,
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

// === API Handlers ===

/// List all accounts for the authenticated user
#[utoipa::path(
    get,
    path = "/api/v1/accounts",
    responses(
        (status = 200, description = "List of accounts", body = Vec<AccountResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn list_accounts_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
) -> Result<Json<Vec<AccountResponse>>, Response> {
    let user = get_or_create_user(&db, &token).await?;

    let accounts = accounts::Entity::find()
        .filter(accounts::Column::UserId.eq(user.id))
        .all(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?;

    Ok(Json(accounts.into_iter().map(|a| a.into()).collect()))
}

/// Get a specific account
#[utoipa::path(
    get,
    path = "/api/v1/accounts/{account_id}",
    params(
        ("account_id" = Uuid, Path, description = "Account ID")
    ),
    responses(
        (status = 200, description = "Account details", body = AccountResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn get_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(account_id): Path<Uuid>,
) -> Result<Json<AccountResponse>, Response> {
    let user = get_or_create_user(&db, &token).await?;

    let account = accounts::Entity::find_by_id(account_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found").into_response())?;

    if account.user_id != user.id {
        return Err((StatusCode::FORBIDDEN, "Access denied").into_response());
    }

    Ok(Json(account.into()))
}

/// Create a new account
#[utoipa::path(
    post,
    path = "/api/v1/accounts",
    request_body = CreateAccountRequest,
    responses(
        (status = 201, description = "Account created", body = AccountResponse),
        (status = 400, description = "Invalid request"),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn create_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, Response> {
    let user = get_or_create_user(&db, &token).await?;

    // Validate account type
    if req.account_type != "exchange" && req.account_type != "wallet" {
        return Err((
            StatusCode::BAD_REQUEST,
            "account_type must be 'exchange' or 'wallet'",
        )
            .into_response());
    }

    // Validate required fields based on account type
    if req.account_type == "exchange" && req.exchange_name.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "exchange_name is required for exchange accounts",
        )
            .into_response());
    }

    if req.account_type == "wallet" && req.wallet_address.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            "wallet_address is required for wallet accounts",
        )
            .into_response());
    }

    // Create account
    // SECURITY NOTE: API credentials should be encrypted before storage
    // Current implementation stores credentials in plaintext with _encrypted suffix as placeholder
    // TODO: Implement proper encryption/decryption for api_key, api_secret, and passphrase fields
    let new_account = accounts::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user.id),
        name: Set(req.name),
        account_type: Set(req.account_type),
        exchange_name: Set(req.exchange_name),
        wallet_address: Set(req.wallet_address),
        api_key_encrypted: Set(req.api_key), // TODO: Encrypt before storing
        api_secret_encrypted: Set(req.api_secret), // TODO: Encrypt before storing
        passphrase_encrypted: Set(req.passphrase), // TODO: Encrypt before storing
        is_active: Set(true),
        ..Default::default()
    };

    let account = new_account.insert(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create account: {}", e),
        )
            .into_response()
    })?;

    Ok(Json(account.into()))
}

/// Update an existing account
#[utoipa::path(
    put,
    path = "/api/v1/accounts/{account_id}",
    params(
        ("account_id" = Uuid, Path, description = "Account ID")
    ),
    request_body = UpdateAccountRequest,
    responses(
        (status = 200, description = "Account updated", body = AccountResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn update_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(account_id): Path<Uuid>,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, Response> {
    let user = get_or_create_user(&db, &token).await?;

    // Find and verify ownership
    let account = accounts::Entity::find_by_id(account_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found").into_response())?;

    if account.user_id != user.id {
        return Err((StatusCode::FORBIDDEN, "Access denied").into_response());
    }

    // Update account
    // SECURITY NOTE: When updating API credentials, they should be encrypted before storage
    // TODO: Implement proper encryption for credential updates
    let mut active_account: accounts::ActiveModel = account.into();
    
    if let Some(name) = req.name {
        active_account.name = Set(name);
    }
    if let Some(is_active) = req.is_active {
        active_account.is_active = Set(is_active);
    }
    if let Some(api_key) = req.api_key {
        active_account.api_key_encrypted = Set(Some(api_key)); // TODO: Encrypt before storing
    }
    if let Some(api_secret) = req.api_secret {
        active_account.api_secret_encrypted = Set(Some(api_secret)); // TODO: Encrypt before storing
    }
    if let Some(passphrase) = req.passphrase {
        active_account.passphrase_encrypted = Set(Some(passphrase)); // TODO: Encrypt before storing
    }

    let updated_account = active_account.update(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to update account: {}", e),
        )
            .into_response()
    })?;

    Ok(Json(updated_account.into()))
}

/// Delete an account
#[utoipa::path(
    delete,
    path = "/api/v1/accounts/{account_id}",
    params(
        ("account_id" = Uuid, Path, description = "Account ID")
    ),
    responses(
        (status = 204, description = "Account deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn delete_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(account_id): Path<Uuid>,
) -> Result<StatusCode, Response> {
    let user = get_or_create_user(&db, &token).await?;

    // Find and verify ownership
    let account = accounts::Entity::find_by_id(account_id)
        .one(&db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            )
                .into_response()
        })?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Account not found").into_response())?;

    if account.user_id != user.id {
        return Err((StatusCode::FORBIDDEN, "Access denied").into_response());
    }

    // Delete account
    let active_account: accounts::ActiveModel = account.into();
    active_account.delete(&db).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to delete account: {}", e),
        )
            .into_response()
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Sync a specific account
///
/// Triggers a sync for a specific account to fetch latest balances from the exchange
#[utoipa::path(
    post,
    path = "/api/v1/accounts/{account_id}/sync",
    params(
        ("account_id" = Uuid, Path, description = "Account ID to sync")
    ),
    responses(
        (status = 200, description = "Sync completed", body = SyncResultResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn sync_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(account_id): Path<Uuid>,
) -> Result<Json<SyncResultResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await?;

    // Verify account belongs to user
    let account = accounts::Entity::find_by_id(account_id)
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
            (StatusCode::NOT_FOUND, "Account not found").into_response()
        })?;

    if account.user_id != user.id {
        return Err((StatusCode::FORBIDDEN, "Access denied").into_response());
    }

    // Perform sync
    let result = account_sync::sync_account(&db, account_id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sync failed: {}", e),
            )
                .into_response()
        })?;

    Ok(Json(result.into()))
}

/// Sync all accounts for the authenticated user
///
/// Triggers a sync for all active accounts belonging to the authenticated user
#[utoipa::path(
    post,
    path = "/api/v1/accounts/sync-all",
    responses(
        (status = 200, description = "Sync completed", body = SyncAllAccountsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "accounts"
)]
async fn sync_all_accounts_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
) -> Result<Json<SyncAllAccountsResponse>, Response> {
    // Get or create user
    let user = get_or_create_user(&db, &token).await?;

    // Perform sync for all user accounts
    let results = account_sync::sync_user_accounts(&db, user.id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Sync failed: {}", e),
            )
                .into_response()
        })?;

    let total = results.len();
    let successful = results.iter().filter(|r| r.success).count();
    let failed = total - successful;

    Ok(Json(SyncAllAccountsResponse {
        total,
        successful,
        failed,
        results: results.into_iter().map(|r| r.into()).collect(),
    }))
}

/// Create router for account endpoints
/// 
/// Note: Axum uses curly braces for path parameters {param}, not colon notation :param
/// This is different from Express.js and some other frameworks
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/api/v1/accounts", get(list_accounts_handler).post(create_account_handler))
        .route("/api/v1/accounts/{account_id}", get(get_account_handler).put(update_account_handler).delete(delete_account_handler))
        .route("/api/v1/accounts/{account_id}/sync", post(sync_account_handler))
        .route("/api/v1/accounts/sync-all", post(sync_all_accounts_handler))
}
