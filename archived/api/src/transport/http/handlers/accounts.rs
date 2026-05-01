use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::application::usecases::account_usecases::{
    AccountUseCases, CreateAccountCommand, UpdateAccountCommand,
};
use crate::helpers::auth::get_or_create_user;
use crate::application::jobs::account_sync;
use super::super::error::ApiError;

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
    /// Enabled EVM chains for wallet accounts (e.g., ["ethereum", "arbitrum", "bsc"])
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_chains: Option<Vec<String>>,
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

#[derive(Debug, Serialize, Deserialize, ToSchema, Clone)]
pub struct AccountHolding {
    pub asset: String,
    pub quantity: String,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled_chains: Option<Vec<String>>,
    pub is_active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_synced_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub holdings: Option<Vec<AccountHolding>>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<crate::domains::account::aggregate::Account> for AccountResponse {
    fn from(account: crate::domains::account::aggregate::Account) -> Self {
        let enabled_chains = if account.enabled_chains.is_empty() {
            None
        } else {
            Some(account.enabled_chains.clone())
        };

        let holdings = if account.holdings().is_empty() {
            None
        } else {
            Some(
                account
                    .holdings()
                    .items
                    .iter()
                    .map(|h| AccountHolding {
                        asset: h.asset.clone(),
                        quantity: h.quantity.clone(),
                    })
                    .collect(),
            )
        };

        Self {
            id: account.id,
            user_id: account.user_id,
            name: account.name,
            account_type: account.account_type.as_str().to_string(),
            exchange_name: account.exchange_name,
            wallet_address: account.wallet_address,
            enabled_chains,
            is_active: account.is_active,
            last_synced_at: account.last_synced_at.map(|dt| dt.to_rfc3339()),
            holdings,
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
pub struct SyncInitiatedResponse {
    /// Human-readable status message
    pub message: String,
    /// Account ID that sync was initiated for
    pub account_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncAllInitiatedResponse {
    /// Human-readable status message
    pub message: String,
    /// Number of accounts queued for background sync
    pub account_count: usize,
}

// === Helper Functions ===


// === API Handlers ===

/// Converts a domain `AccountError` to the appropriate `ApiError`.
fn map_account_error(e: crate::domains::account::aggregate::AccountError) -> ApiError {
    use crate::domains::account::aggregate::AccountError;
    match e {
        AccountError::NotFound => ApiError::NotFound,
        AccountError::MissingExchangeName | AccountError::MissingWalletAddress | AccountError::InvalidCredentials | AccountError::NotAnExchangeAccount => {
            ApiError::BadRequest(e.to_string())
        }
        AccountError::PersistenceError(msg) => ApiError::InternalServerError(msg),
    }
}

/// List all accounts for the authenticated user
#[utoipa::path(
    get,
    path = "/api/v1/accounts",
    responses(
        (status = 200, description = "List of accounts", body = Vec<AccountResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),
    tag = "accounts"
)]
async fn list_accounts_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
) -> Result<Json<Vec<AccountResponse>>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    let accounts = use_cases.list_accounts(user.id).await.map_err(map_account_error)?;
    Ok(Json(accounts.into_iter().map(AccountResponse::from).collect()))
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

    tag = "accounts"
)]
async fn get_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
    Path(account_id): Path<Uuid>,
) -> Result<Json<AccountResponse>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    let account = use_cases.get_account(account_id).await.map_err(map_account_error)?;
    if account.user_id != user.id {
        return Err(ApiError::Forbidden);
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

    tag = "accounts"
)]
async fn create_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
    Json(req): Json<CreateAccountRequest>,
) -> Result<Json<AccountResponse>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    // SECURITY NOTE: API credentials should be encrypted before storage.
    // Current implementation stores credentials in plaintext.
    // TODO: Implement proper encryption/decryption for api_key, api_secret, and passphrase fields.
    let cmd = CreateAccountCommand {
        user_id: user.id,
        name: req.name,
        account_type: req.account_type,
        exchange_name: req.exchange_name,
        wallet_address: req.wallet_address,
        enabled_chains: req.enabled_chains,
        api_key: req.api_key,
        api_secret: req.api_secret,
        passphrase: req.passphrase,
    };

    let account = use_cases.create_account(cmd).await.map_err(map_account_error)?;
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

    tag = "accounts"
)]
async fn update_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
    Path(account_id): Path<Uuid>,
    Json(req): Json<UpdateAccountRequest>,
) -> Result<Json<AccountResponse>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    // Ownership check: load the account first, verify the user owns it.
    let existing = use_cases.get_account(account_id).await.map_err(map_account_error)?;
    if existing.user_id != user.id {
        return Err(ApiError::Forbidden);
    }

    // SECURITY NOTE: When updating API credentials, they should be encrypted before storage.
    // TODO: Implement proper encryption for credential updates.
    let cmd = UpdateAccountCommand {
        id: account_id,
        name: req.name,
        is_active: req.is_active,
        api_key: req.api_key,
        api_secret: req.api_secret,
        passphrase: req.passphrase,
    };

    let updated = use_cases.update_account(cmd).await.map_err(map_account_error)?;
    Ok(Json(updated.into()))
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

    tag = "accounts"
)]
async fn delete_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
    Path(account_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    // Ownership check before deletion.
    let account = use_cases.get_account(account_id).await.map_err(map_account_error)?;
    if account.user_id != user.id {
        return Err(ApiError::Forbidden);
    }

    use_cases.delete_account(account_id).await.map_err(map_account_error)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Sync a specific account
///
/// Triggers a background sync for a specific account to fetch latest balances.
/// Returns immediately with 202 Accepted while sync runs in the background.
#[utoipa::path(
    post,
    path = "/api/v1/accounts/{account_id}/sync",
    params(
        ("account_id" = Uuid, Path, description = "Account ID to sync")
    ),
    responses(
        (status = 202, description = "Sync started in background", body = SyncInitiatedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Account not found"),
        (status = 500, description = "Internal server error")
    ),

    tag = "accounts"
)]
async fn sync_account_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
    Path(account_id): Path<Uuid>,
) -> Result<(StatusCode, Json<SyncInitiatedResponse>), ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    // Ownership check via use case.
    let account = use_cases.get_account(account_id).await.map_err(map_account_error)?;
    if account.user_id != user.id {
        return Err(ApiError::Forbidden);
    }

    // Spawn background sync task – return 202 immediately so the UI is not blocked.
    // A watcher task monitors the JoinHandle so panics are observable in logs.
    let db_bg = db.clone();
    let handle = tokio::spawn(async move {
        match account_sync::sync_account(&db_bg, account_id).await {
            Ok(result) => {
                if result.success {
                    tracing::info!(
                        "Background sync completed for account {}: {} holdings",
                        account_id, result.holdings_count
                    );
                } else {
                    tracing::warn!(
                        "Background sync finished with error for account {}: {:?}",
                        account_id, result.error
                    );
                }
            }
            Err(e) => {
                tracing::error!("Background sync failed for account {}: {}", account_id, e);
            }
        }
    });
    tokio::spawn(async move {
        if let Err(e) = handle.await {
            tracing::error!("Background sync task panicked for account {}: {:?}", account_id, e);
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(SyncInitiatedResponse {
            message: "Sync started in background".to_string(),
            account_id,
        }),
    ))
}

/// Sync all accounts for the authenticated user
///
/// Triggers background syncs for all active accounts belonging to the authenticated user.
/// Returns immediately with 202 Accepted while syncs run in the background.
#[utoipa::path(
    post,
    path = "/api/v1/accounts/sync-all",
    responses(
        (status = 202, description = "Sync started in background", body = SyncAllInitiatedResponse),
        (status = 401, description = "Unauthorized"),
        (status = 500, description = "Internal server error")
    ),

    tag = "accounts"
)]
async fn sync_all_accounts_handler(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Extension(use_cases): Extension<Arc<AccountUseCases>>,
) -> Result<(StatusCode, Json<SyncAllInitiatedResponse>), ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    // List active accounts so we can report the count in the response.
    let all_accounts = use_cases.list_accounts(user.id).await.map_err(map_account_error)?;
    let account_count = all_accounts.iter().filter(|a| a.is_active).count();
    let user_id = user.id;

    // Spawn a single background task that syncs all accounts sequentially.
    // A watcher task monitors the JoinHandle so panics are observable in logs.
    let db_bg = db.clone();
    let handle = tokio::spawn(async move {
        tracing::info!("Background sync started for {} accounts of user {}", account_count, user_id);
        match account_sync::sync_user_accounts(&db_bg, user_id).await {
            Ok(results) => {
                let successful = results.iter().filter(|r| r.success).count();
                let failed = results.len() - successful;
                tracing::info!(
                    "Background sync completed for user {}: {} successful, {} failed",
                    user_id, successful, failed
                );
            }
            Err(e) => {
                tracing::error!("Background sync failed for user {}: {}", user_id, e);
            }
        }
    });
    tokio::spawn(async move {
        if let Err(e) = handle.await {
            tracing::error!("Background sync-all task panicked for user {}: {:?}", user_id, e);
        }
    });

    Ok((
        StatusCode::ACCEPTED,
        Json(SyncAllInitiatedResponse {
            message: "Sync started in background".to_string(),
            account_count,
        }),
    ))
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
