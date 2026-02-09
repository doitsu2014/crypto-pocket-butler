use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::post,
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{accounts, users};
use crate::jobs::account_sync;

// === Request/Response DTOs ===

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

/// Create router for account sync endpoints
pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/api/v1/accounts/{account_id}/sync", post(sync_account_handler))
        .route("/api/v1/accounts/sync-all", post(sync_all_accounts_handler))
}
