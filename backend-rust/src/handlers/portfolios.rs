use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{delete, get},
    Router,
};
use axum_keycloak_auth::decode::KeycloakToken;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, ModelTrait,
    QueryFilter,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::entities::{accounts, portfolio_accounts, portfolios, users};

// === Request/Response DTOs ===

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreatePortfolioRequest {
    /// Portfolio name
    pub name: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this is the default portfolio
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdatePortfolioRequest {
    /// Portfolio name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this is the default portfolio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_default: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PortfolioResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub is_default: bool,
    pub created_at: String,
    pub updated_at: String,
}

impl From<portfolios::Model> for PortfolioResponse {
    fn from(model: portfolios::Model) -> Self {
        Self {
            id: model.id,
            user_id: model.user_id,
            name: model.name,
            description: model.description,
            is_default: model.is_default,
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AddAccountToPortfolioRequest {
    /// Account ID to add
    pub account_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PortfolioAccountResponse {
    pub id: Uuid,
    pub portfolio_id: Uuid,
    pub account_id: Uuid,
    pub added_at: String,
}

impl From<portfolio_accounts::Model> for PortfolioAccountResponse {
    fn from(model: portfolio_accounts::Model) -> Self {
        Self {
            id: model.id,
            portfolio_id: model.portfolio_id,
            account_id: model.account_id,
            added_at: model.added_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountInPortfolioResponse {
    pub id: Uuid,
    pub name: String,
    pub account_type: String,
    pub exchange_name: Option<String>,
    pub wallet_address: Option<String>,
    pub is_active: bool,
    pub last_synced_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<accounts::Model> for AccountInPortfolioResponse {
    fn from(model: accounts::Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            account_type: model.account_type,
            exchange_name: model.exchange_name,
            wallet_address: model.wallet_address,
            is_active: model.is_active,
            last_synced_at: model.last_synced_at.map(|dt| dt.to_string()),
            created_at: model.created_at.to_string(),
            updated_at: model.updated_at.to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AssetHolding {
    /// Asset symbol (e.g., BTC, ETH, USDT)
    pub asset: String,
    /// Total quantity across all accounts
    pub total_quantity: String,
    /// Total available quantity
    pub total_available: String,
    /// Total frozen quantity
    pub total_frozen: String,
    /// Price per unit in USD
    pub price_usd: f64,
    /// Total value in USD
    pub value_usd: f64,
    /// List of accounts holding this asset
    pub accounts: Vec<AccountHoldingDetail>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AccountHoldingDetail {
    pub account_id: Uuid,
    pub account_name: String,
    pub quantity: String,
    pub available: String,
    pub frozen: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AllocationItem {
    /// Asset symbol
    pub asset: String,
    /// Value in USD
    pub value_usd: f64,
    /// Percentage of total portfolio
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct PortfolioHoldingsResponse {
    /// Portfolio ID
    pub portfolio_id: Uuid,
    /// Total portfolio value in USD
    pub total_value_usd: f64,
    /// Holdings grouped by asset
    pub holdings: Vec<AssetHolding>,
    /// Allocation breakdown
    pub allocation: Vec<AllocationItem>,
    /// Timestamp of the data
    pub as_of: String,
}

// === Error handling ===

#[derive(Debug)]
pub enum ApiError {
    DatabaseError(sea_orm::DbErr),
    NotFound,
    Unauthorized,
    BadRequest(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::DatabaseError(err) => {
                tracing::error!("Database error: {:?}", err);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error".to_string(),
                )
            }
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found".to_string()),
            ApiError::Unauthorized => (StatusCode::FORBIDDEN, "Forbidden".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
        };

        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<sea_orm::DbErr> for ApiError {
    fn from(err: sea_orm::DbErr) -> Self {
        ApiError::DatabaseError(err)
    }
}

// === Helper functions ===

/// Get or create user in database based on Keycloak token
async fn get_or_create_user(
    db: &DatabaseConnection,
    token: &KeycloakToken<String>,
) -> Result<users::Model, ApiError> {
    // Try to find existing user
    let user = users::Entity::find()
        .filter(users::Column::KeycloakUserId.eq(&token.subject))
        .one(db)
        .await?;

    if let Some(user) = user {
        return Ok(user);
    }

    // Create new user if not found
    let new_user = users::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        keycloak_user_id: ActiveValue::Set(token.subject.clone()),
        email: ActiveValue::Set(Some(token.extra.email.email.clone())),
        preferred_username: ActiveValue::Set(Some(token.extra.profile.preferred_username.clone())),
        created_at: ActiveValue::NotSet,
        updated_at: ActiveValue::NotSet,
    };

    let user = new_user.insert(db).await?;
    Ok(user)
}

/// Check if user owns a portfolio
async fn check_portfolio_ownership(
    db: &DatabaseConnection,
    portfolio_id: Uuid,
    user_id: Uuid,
) -> Result<portfolios::Model, ApiError> {
    let portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .one(db)
        .await?
        .ok_or(ApiError::NotFound)?;

    if portfolio.user_id != user_id {
        return Err(ApiError::Unauthorized);
    }

    Ok(portfolio)
}

/// Check if user owns an account
async fn check_account_ownership(
    db: &DatabaseConnection,
    account_id: Uuid,
    user_id: Uuid,
) -> Result<accounts::Model, ApiError> {
    let account = accounts::Entity::find_by_id(account_id)
        .one(db)
        .await?
        .ok_or(ApiError::NotFound)?;

    if account.user_id != user_id {
        return Err(ApiError::Unauthorized);
    }

    Ok(account)
}

/// Unset default flag for all user's portfolios except the specified one
async fn unset_other_default_portfolios(
    db: &DatabaseConnection,
    user_id: Uuid,
    exclude_portfolio_id: Option<Uuid>,
) -> Result<(), ApiError> {
    let mut query = portfolios::Entity::update_many()
        .filter(portfolios::Column::UserId.eq(user_id))
        .filter(portfolios::Column::IsDefault.eq(true));

    if let Some(id) = exclude_portfolio_id {
        query = query.filter(portfolios::Column::Id.ne(id));
    }

    query
        .col_expr(
            portfolios::Column::IsDefault,
            sea_orm::sea_query::Expr::value(false),
        )
        .exec(db)
        .await?;

    Ok(())
}

// === Route handlers ===

/// List all portfolios for the authenticated user
#[utoipa::path(
    get,
    path = "/v1/portfolios",
    responses(
        (status = 200, description = "List of portfolios", body = Vec<PortfolioResponse>),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn list_portfolios(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
) -> Result<Json<Vec<PortfolioResponse>>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    let portfolios = portfolios::Entity::find()
        .filter(portfolios::Column::UserId.eq(user.id))
        .all(&db)
        .await?;

    let response: Vec<PortfolioResponse> = portfolios.into_iter().map(|p| p.into()).collect();
    Ok(Json(response))
}

/// Get a single portfolio by ID
#[utoipa::path(
    get,
    path = "/v1/portfolios/{id}",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 200, description = "Portfolio details", body = PortfolioResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn get_portfolio(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(id): Path<Uuid>,
) -> Result<Json<PortfolioResponse>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    let portfolio = check_portfolio_ownership(&db, id, user.id).await?;
    Ok(Json(portfolio.into()))
}

/// Create a new portfolio
#[utoipa::path(
    post,
    path = "/v1/portfolios",
    request_body = CreatePortfolioRequest,
    responses(
        (status = 201, description = "Portfolio created", body = PortfolioResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn create_portfolio(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Json(req): Json<CreatePortfolioRequest>,
) -> Result<(StatusCode, Json<PortfolioResponse>), ApiError> {
    let user = get_or_create_user(&db, &token).await?;

    // If this is set as default, unset any existing default portfolio
    if req.is_default {
        unset_other_default_portfolios(&db, user.id, None).await?;
    }

    let new_portfolio = portfolios::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        user_id: ActiveValue::Set(user.id),
        name: ActiveValue::Set(req.name),
        description: ActiveValue::Set(req.description),
        is_default: ActiveValue::Set(req.is_default),
        created_at: ActiveValue::NotSet,
        updated_at: ActiveValue::NotSet,
    };

    let portfolio = new_portfolio.insert(&db).await?;
    Ok((StatusCode::CREATED, Json(portfolio.into())))
}

/// Update a portfolio
#[utoipa::path(
    put,
    path = "/v1/portfolios/{id}",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    request_body = UpdatePortfolioRequest,
    responses(
        (status = 200, description = "Portfolio updated", body = PortfolioResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn update_portfolio(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePortfolioRequest>,
) -> Result<Json<PortfolioResponse>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    let portfolio = check_portfolio_ownership(&db, id, user.id).await?;

    // If this is set as default, unset any existing default portfolio
    if req.is_default == Some(true) {
        unset_other_default_portfolios(&db, user.id, Some(id)).await?;
    }

    let mut active_portfolio: portfolios::ActiveModel = portfolio.into();

    if let Some(name) = req.name {
        active_portfolio.name = ActiveValue::Set(name);
    }
    if req.description.is_some() {
        active_portfolio.description = ActiveValue::Set(req.description);
    }
    if let Some(is_default) = req.is_default {
        active_portfolio.is_default = ActiveValue::Set(is_default);
    }

    let updated_portfolio = active_portfolio.update(&db).await?;
    Ok(Json(updated_portfolio.into()))
}

/// Delete a portfolio
#[utoipa::path(
    delete,
    path = "/v1/portfolios/{id}",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 204, description = "Portfolio deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn delete_portfolio(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    let portfolio = check_portfolio_ownership(&db, id, user.id).await?;

    portfolio.delete(&db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// List all accounts in a portfolio
#[utoipa::path(
    get,
    path = "/v1/portfolios/{id}/accounts",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 200, description = "List of accounts in portfolio", body = Vec<AccountInPortfolioResponse>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn list_portfolio_accounts(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<AccountInPortfolioResponse>>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    check_portfolio_ownership(&db, id, user.id).await?;

    // Get all accounts linked to this portfolio
    let portfolio_accounts = portfolio_accounts::Entity::find()
        .filter(portfolio_accounts::Column::PortfolioId.eq(id))
        .all(&db)
        .await?;

    let account_ids: Vec<Uuid> = portfolio_accounts
        .iter()
        .map(|pa| pa.account_id)
        .collect();

    let accounts = accounts::Entity::find()
        .filter(accounts::Column::Id.is_in(account_ids))
        .all(&db)
        .await?;

    let response: Vec<AccountInPortfolioResponse> = accounts.into_iter().map(|a| a.into()).collect();
    Ok(Json(response))
}

/// Add an account to a portfolio
#[utoipa::path(
    post,
    path = "/v1/portfolios/{id}/accounts",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    request_body = AddAccountToPortfolioRequest,
    responses(
        (status = 201, description = "Account added to portfolio", body = PortfolioAccountResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio or account not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn add_account_to_portfolio(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(id): Path<Uuid>,
    Json(req): Json<AddAccountToPortfolioRequest>,
) -> Result<(StatusCode, Json<PortfolioAccountResponse>), ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    
    // Check portfolio ownership
    check_portfolio_ownership(&db, id, user.id).await?;
    
    // Check account ownership
    check_account_ownership(&db, req.account_id, user.id).await?;

    // Check if account is already in the portfolio
    let existing = portfolio_accounts::Entity::find()
        .filter(portfolio_accounts::Column::PortfolioId.eq(id))
        .filter(portfolio_accounts::Column::AccountId.eq(req.account_id))
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(ApiError::BadRequest(
            "Account is already in this portfolio".to_string(),
        ));
    }

    // Add account to portfolio
    let portfolio_account = portfolio_accounts::ActiveModel {
        id: ActiveValue::Set(Uuid::new_v4()),
        portfolio_id: ActiveValue::Set(id),
        account_id: ActiveValue::Set(req.account_id),
        added_at: ActiveValue::NotSet,
    };

    let result = portfolio_account.insert(&db).await?;
    Ok((StatusCode::CREATED, Json(result.into())))
}

/// Remove an account from a portfolio
#[utoipa::path(
    delete,
    path = "/v1/portfolios/{id}/accounts/{account_id}",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID"),
        ("account_id" = Uuid, Path, description = "Account ID")
    ),
    responses(
        (status = 204, description = "Account removed from portfolio"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio or account association not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn remove_account_from_portfolio(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path((id, account_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    
    // Check portfolio ownership
    check_portfolio_ownership(&db, id, user.id).await?;

    // Find and delete the association
    let portfolio_account = portfolio_accounts::Entity::find()
        .filter(portfolio_accounts::Column::PortfolioId.eq(id))
        .filter(portfolio_accounts::Column::AccountId.eq(account_id))
        .one(&db)
        .await?
        .ok_or(ApiError::NotFound)?;

    portfolio_account.delete(&db).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Get portfolio holdings and allocation
#[utoipa::path(
    get,
    path = "/v1/portfolios/{id}/holdings",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 200, description = "Portfolio holdings and allocation", body = PortfolioHoldingsResponse),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Portfolio not found")
    ),
    security(
        ("bearer_auth" = [])
    ),
    tag = "portfolios"
)]
pub async fn get_portfolio_holdings(
    State(db): State<DatabaseConnection>,
    Extension(token): Extension<KeycloakToken<String>>,
    Path(id): Path<Uuid>,
) -> Result<Json<PortfolioHoldingsResponse>, ApiError> {
    let user = get_or_create_user(&db, &token).await?;
    check_portfolio_ownership(&db, id, user.id).await?;

    // Get all accounts linked to this portfolio
    let portfolio_accounts = portfolio_accounts::Entity::find()
        .filter(portfolio_accounts::Column::PortfolioId.eq(id))
        .all(&db)
        .await?;

    let account_ids: Vec<Uuid> = portfolio_accounts
        .iter()
        .map(|pa| pa.account_id)
        .collect();

    let accounts = accounts::Entity::find()
        .filter(accounts::Column::Id.is_in(account_ids))
        .all(&db)
        .await?;

    // Aggregate holdings by asset
    let mut holdings_map: HashMap<String, AssetHolding> = HashMap::new();

    for account in accounts {
        if let Some(holdings_json) = account.holdings {
            // holdings_json is sea_orm::prelude::Json which wraps serde_json::Value
            let holdings: Vec<serde_json::Value> = 
                serde_json::from_value(serde_json::Value::from(holdings_json)).unwrap_or_default();
            
            for holding in holdings {
                let asset = holding["asset"].as_str().unwrap_or("UNKNOWN").to_string();
                let quantity = holding["quantity"].as_str().unwrap_or("0").to_string();
                let available = holding["available"].as_str().unwrap_or("0").to_string();
                let frozen = holding["frozen"].as_str().unwrap_or("0").to_string();
                let price_usd = holding["price_usd"].as_f64().unwrap_or(0.0);
                let value_usd = holding["value_usd"].as_f64().unwrap_or(0.0);

                let entry = holdings_map.entry(asset.clone()).or_insert_with(|| {
                    AssetHolding {
                        asset: asset.clone(),
                        total_quantity: "0".to_string(),
                        total_available: "0".to_string(),
                        total_frozen: "0".to_string(),
                        price_usd,
                        value_usd: 0.0,
                        accounts: Vec::new(),
                    }
                });

                // Add to totals (simple string concatenation for now - in production use Decimal)
                let qty: f64 = quantity.parse().unwrap_or(0.0);
                let avail: f64 = available.parse().unwrap_or(0.0);
                let frz: f64 = frozen.parse().unwrap_or(0.0);
                let curr_qty: f64 = entry.total_quantity.parse().unwrap_or(0.0);
                let curr_avail: f64 = entry.total_available.parse().unwrap_or(0.0);
                let curr_frz: f64 = entry.total_frozen.parse().unwrap_or(0.0);

                entry.total_quantity = (curr_qty + qty).to_string();
                entry.total_available = (curr_avail + avail).to_string();
                entry.total_frozen = (curr_frz + frz).to_string();
                entry.value_usd += value_usd;

                entry.accounts.push(AccountHoldingDetail {
                    account_id: account.id,
                    account_name: account.name.clone(),
                    quantity,
                    available,
                    frozen,
                });
            }
        }
    }

    // Convert to vec and sort by value descending
    let mut holdings: Vec<AssetHolding> = holdings_map.into_values().collect();
    holdings.sort_by(|a, b| b.value_usd.partial_cmp(&a.value_usd).unwrap());

    // Calculate total value and allocation
    let total_value_usd: f64 = holdings.iter().map(|h| h.value_usd).sum();

    let allocation: Vec<AllocationItem> = holdings
        .iter()
        .map(|h| AllocationItem {
            asset: h.asset.clone(),
            value_usd: h.value_usd,
            percentage: if total_value_usd > 0.0 {
                (h.value_usd / total_value_usd) * 100.0
            } else {
                0.0
            },
        })
        .collect();

    Ok(Json(PortfolioHoldingsResponse {
        portfolio_id: id,
        total_value_usd,
        holdings,
        allocation,
        as_of: chrono::Utc::now().to_rfc3339(),
    }))
}

// === Router setup ===

pub fn create_router() -> Router<DatabaseConnection> {
    Router::new()
        .route("/v1/portfolios", get(list_portfolios).post(create_portfolio))
        .route(
            "/v1/portfolios/:id",
            get(get_portfolio)
                .put(update_portfolio)
                .delete(delete_portfolio),
        )
        .route(
            "/v1/portfolios/:id/accounts",
            get(list_portfolio_accounts).post(add_account_to_portfolio),
        )
        .route(
            "/v1/portfolios/:id/accounts/:account_id",
            delete(remove_account_from_portfolio),
        )
        .route(
            "/v1/portfolios/:id/holdings",
            get(get_portfolio_holdings),
        )
}
