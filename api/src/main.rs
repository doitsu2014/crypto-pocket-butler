use axum::{extract::Extension, response::Json, routing::get, Router};
use axum_keycloak_auth::{
    decode::KeycloakToken, instance::KeycloakAuthInstance, instance::KeycloakConfig,
    layer::KeycloakAuthLayer, PassthroughMode,
};
use crypto_pocket_butler_backend::{db::DbConfig, handlers, jobs};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tokio_cron_scheduler::{JobScheduler, Job};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

/// User information response
#[derive(Serialize, Deserialize, ToSchema)]
struct UserInfo {
    /// User ID from JWT sub claim
    user_id: String,
    /// Preferred username
    #[serde(skip_serializing_if = "Option::is_none")]
    preferred_username: Option<String>,
    /// Email address
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<String>,
    /// Whether email is verified
    #[serde(skip_serializing_if = "Option::is_none")]
    email_verified: Option<bool>,
}

/// Protected endpoint response
#[derive(Serialize, Deserialize, ToSchema)]
struct ProtectedResponse {
    /// Response message
    message: String,
    /// User ID
    user_id: String,
}

/// Health check response
#[derive(Serialize, Deserialize, ToSchema)]
struct HealthResponse {
    /// Service status
    status: String,
    /// Service name
    service: String,
}

/// OpenAPI documentation
#[derive(OpenApi)]
#[openapi(
    paths(
        root,
        health,
        get_user_info,
        protected_endpoint,
        handlers::portfolios::list_portfolios,
        handlers::portfolios::get_portfolio,
        handlers::portfolios::create_portfolio,
        handlers::portfolios::update_portfolio,
        handlers::portfolios::delete_portfolio,
        handlers::portfolios::list_portfolio_accounts,
        handlers::portfolios::add_account_to_portfolio,
        handlers::portfolios::remove_account_from_portfolio,
        handlers::portfolios::construct_portfolio_allocation,
        handlers::accounts::list_accounts_handler,
        handlers::accounts::get_account_handler,
        handlers::accounts::create_account_handler,
        handlers::accounts::update_account_handler,
        handlers::accounts::delete_account_handler,
        handlers::accounts::sync_account_handler,
        handlers::accounts::sync_all_accounts_handler,
        handlers::chains::list_supported_chains,
        handlers::snapshots::create_portfolio_snapshot_handler,
        handlers::snapshots::create_all_user_snapshots_handler,
        handlers::snapshots::list_portfolio_snapshots_handler,
        handlers::snapshots::get_latest_portfolio_snapshot_handler,
        handlers::recommendations::list_portfolio_recommendations,
        handlers::recommendations::get_recommendation,
        handlers::recommendations::create_recommendation,
        handlers::recommendations::generate_mock_recommendations,
        handlers::migrations::migrate_handler,
        handlers::jobs::fetch_all_coins_handler,
        handlers::evm_tokens::list_evm_tokens_handler,
        handlers::evm_tokens::get_evm_token_handler,
        handlers::evm_tokens::create_evm_token_handler,
        handlers::evm_tokens::update_evm_token_handler,
        handlers::evm_tokens::delete_evm_token_handler,
        handlers::evm_tokens::sync_tokens_from_contracts_handler,
        handlers::evm_tokens::lookup_contracts_handler,
        handlers::evm_chains::list_evm_chains_handler,
        handlers::evm_chains::get_evm_chain_handler,
        handlers::evm_chains::create_evm_chain_handler,
        handlers::evm_chains::update_evm_chain_handler,
        handlers::evm_chains::delete_evm_chain_handler,
        handlers::solana_tokens::list_solana_tokens_handler,
        handlers::solana_tokens::get_solana_token_handler,
        handlers::solana_tokens::create_solana_token_handler,
        handlers::solana_tokens::update_solana_token_handler,
        handlers::solana_tokens::delete_solana_token_handler,
    ),
    components(
        schemas(
            UserInfo, 
            ProtectedResponse, 
            HealthResponse,
            handlers::portfolios::CreatePortfolioRequest,
            handlers::portfolios::UpdatePortfolioRequest,
            handlers::portfolios::PortfolioResponse,
            handlers::portfolios::AddAccountToPortfolioRequest,
            handlers::portfolios::PortfolioAccountResponse,
            handlers::portfolios::AccountInPortfolioResponse,
            handlers::portfolios::AllocationHolding,
            handlers::portfolios::ConstructAllocationResponse,
            handlers::accounts::CreateAccountRequest,
            handlers::accounts::UpdateAccountRequest,
            handlers::accounts::AccountResponse,
            handlers::accounts::SyncAccountRequest,
            handlers::accounts::SyncResultResponse,
            handlers::accounts::SyncInitiatedResponse,
            handlers::accounts::SyncAllInitiatedResponse,
            handlers::chains::ChainInfo,
            handlers::chains::ListChainsResponse,
            handlers::snapshots::CreateSnapshotRequest,
            handlers::snapshots::SnapshotResultResponse,
            handlers::snapshots::CreateAllSnapshotsResponse,
            handlers::snapshots::SnapshotResponse,
            handlers::snapshots::ListSnapshotsQuery,
            handlers::snapshots::ListSnapshotsResponse,
            handlers::recommendations::RecommendationResponse,
            handlers::recommendations::ListRecommendationsResponse,
            handlers::recommendations::ListRecommendationsQuery,
            handlers::recommendations::CreateRecommendationRequest,
            handlers::migrations::MigrationResponse,
            handlers::jobs::FetchAllCoinsResponse,
            handlers::evm_tokens::EvmTokenResponse,
            handlers::evm_tokens::CreateEvmTokenRequest,
            handlers::evm_tokens::UpdateEvmTokenRequest,
            handlers::evm_tokens::SyncFromContractsResponse,
            handlers::evm_tokens::LookupContractsResponse,
            handlers::evm_tokens::ChainContractEntry,
            handlers::evm_chains::EvmChainResponse,
            handlers::evm_chains::CreateEvmChainRequest,
            handlers::evm_chains::UpdateEvmChainRequest,
            handlers::solana_tokens::SolanaTokenResponse,
            handlers::solana_tokens::CreateSolanaTokenRequest,
            handlers::solana_tokens::UpdateSolanaTokenRequest,
            handlers::error::ErrorResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "crypto-pocket-butler", description = "Crypto Pocket Butler API endpoints"),
        (name = "portfolios", description = "Portfolio management endpoints"),
        (name = "accounts", description = "Account management and sync endpoints"),
        (name = "chains", description = "Supported blockchain chains endpoints"),
        (name = "snapshots", description = "Portfolio snapshot endpoints"),
        (name = "recommendations", description = "Portfolio recommendation endpoints"),
        (name = "migrations", description = "Database migration endpoints"),
        (name = "evm-tokens", description = "EVM token registry – configurable list of ERC-20 tokens checked during wallet sync"),
        (name = "evm-chains", description = "EVM chain registry – configurable list of EVM chains with RPC URLs"),
        (name = "solana-tokens", description = "Solana token registry – configurable list of SPL tokens checked during wallet sync"),
    ),
    info(
        title = "Crypto Pocket Butler API",
        version = "0.1.0",
        description = "API for managing crypto portfolio with Keycloak authentication.\n\n\
        ## Authentication\n\n\
        This API supports multiple authentication methods:\n\n\
        1. **Bearer Token (JWT)**: Use a Keycloak JWT token obtained from a successful login\n\
        2. **OAuth2 Client Credentials**: Authenticate using client ID and client secret (for service-to-service)\n\
        3. **OAuth2 Authorization Code**: Authenticate using client ID via authorization code flow (for user authentication)\n\n\
        To use OAuth2 flows in Swagger UI, click the 'Authorize' button and enter your Keycloak credentials.",
    ),
    servers(
        (url = "http://localhost:3001", description = "Local development server (standalone)"),
        (url = "http://localhost:3000", description = "Docker backend server")
    )
)]
struct ApiDoc;

use utoipa::Modify;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            // Bearer token authentication (JWT)
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .description(Some("Enter your Keycloak JWT token"))
                        .build(),
                ),
            );

            // Get Keycloak configuration from environment
            let keycloak_server = std::env::var("KEYCLOAK_SERVER")
                .unwrap_or_else(|_| "http://localhost:8080".to_string());
            let keycloak_realm = std::env::var("KEYCLOAK_REALM")
                .unwrap_or_else(|_| "myrealm".to_string());
            
            let token_url = format!("{}/realms/{}/protocol/openid-connect/token", keycloak_server, keycloak_realm);
            let auth_url = format!("{}/realms/{}/protocol/openid-connect/auth", keycloak_server, keycloak_realm);

            // Create empty scopes (Keycloak handles scopes via client configuration)
            use utoipa::openapi::security::Scopes;
            let scopes = Scopes::new();

            // OAuth2 Client Credentials flow
            components.add_security_scheme(
                "oauth2_client_credentials",
                utoipa::openapi::security::SecurityScheme::OAuth2(
                    utoipa::openapi::security::OAuth2::new([
                        utoipa::openapi::security::Flow::ClientCredentials(
                            utoipa::openapi::security::ClientCredentials::new(token_url.clone(), scopes.clone())
                        )
                    ])
                ),
            );

            // OAuth2 Authorization Code flow
            components.add_security_scheme(
                "oauth2_authorization_code",
                utoipa::openapi::security::SecurityScheme::OAuth2(
                    utoipa::openapi::security::OAuth2::new([
                        utoipa::openapi::security::Flow::AuthorizationCode(
                            utoipa::openapi::security::AuthorizationCode::new(
                                auth_url,
                                token_url,
                                scopes
                            )
                        )
                    ])
                ),
            );

            use utoipa::openapi::security::SecurityRequirement;
            openapi.security = Some(vec![
                SecurityRequirement::new("bearer_auth", Vec::<String>::new()),
                SecurityRequirement::new("oauth2_client_credentials", Vec::<String>::new()),
                SecurityRequirement::new("oauth2_authorization_code", Vec::<String>::new()),
            ]);
        }
    }
}

#[tokio::main]
async fn main() {
    // Load environment variables from .env file if it exists
    // This will not override existing environment variables
    if let Err(e) = dotenvy::dotenv() {
        // It's okay if .env file doesn't exist, we'll use system environment variables
        eprintln!("Warning: Could not load .env file: {}", e);
    }

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crypto_pocket_butler_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Crypto Pocket Butler Backend");
    tracing::info!(
        "Tokio runtime: multi-threaded with {} worker threads",
        num_cpus::get()
    );

    // Initialize database connection pool
    // The connection pool handles concurrent database access efficiently
    // by maintaining a pool of reusable connections
    tracing::info!("Connecting to database...");
    let db = DbConfig::from_env()
        .await
        .expect("Failed to connect to database");
    tracing::info!("Database connection pool established");

    // Initialize job scheduler
    tracing::info!("Initializing job scheduler...");
    let scheduler = JobScheduler::new().await.expect("Failed to create job scheduler");
    
    // Configure fetch all coins job (replaces top_coins_collection and contract_addresses_collection)
    let fetch_all_coins_enabled = std::env::var("FETCH_ALL_COINS_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    
    if fetch_all_coins_enabled {
        let fetch_all_coins_schedule = std::env::var("FETCH_ALL_COINS_SCHEDULE")
            .unwrap_or_else(|_| "0 0 0 * * *".to_string()); // Default: daily at midnight UTC
        
        tracing::info!(
            "Scheduling fetch all coins job: schedule='{}'",
            fetch_all_coins_schedule
        );

        let db_clone = db.clone();
        let job = Job::new_async(fetch_all_coins_schedule.as_str(), move |_job_id, _scheduler| {
            let db = db_clone.clone();
            Box::pin(async move {
                tracing::info!("Running scheduled fetch all coins job");
                match jobs::fetch_all_coins::fetch_all_coins(&db).await {
                    Ok(result) => {
                        if result.success {
                            tracing::info!(
                                "Fetch all coins job completed successfully: {} coins fetched, {} assets created, {} updated, {} prices stored",
                                result.coins_fetched,
                                result.assets_created,
                                result.assets_updated,
                                result.prices_stored
                            );
                        } else {
                            tracing::error!(
                                "Fetch all coins job failed: {}",
                                result.error.unwrap_or_else(|| "Unknown error".to_string())
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Fetch all coins job failed with error: {}", e);
                    }
                }
            })
        })
        .expect("Failed to create fetch all coins job");

        scheduler.add(job).await.expect("Failed to add fetch all coins job to scheduler");
        tracing::info!("Fetch all coins job scheduled successfully");
    } else {
        tracing::info!("Fetch all coins job is disabled");
    }

    // Configure price collection job
    let price_collection_enabled = std::env::var("PRICE_COLLECTION_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    
    if price_collection_enabled {
        let price_collection_schedule = std::env::var("PRICE_COLLECTION_SCHEDULE")
            .unwrap_or_else(|_| "0 */15 * * * *".to_string()); // Default: every 15 minutes
        let price_collection_limit = std::env::var("PRICE_COLLECTION_LIMIT")
            .unwrap_or_else(|_| "100".to_string())
            .parse::<usize>()
            .unwrap_or(100);
        
        tracing::info!(
            "Scheduling price collection job: schedule='{}', limit={}",
            price_collection_schedule,
            price_collection_limit
        );

        let db_clone = db.clone();
        let job = Job::new_async(price_collection_schedule.as_str(), move |_job_id, _scheduler| {
            let db = db_clone.clone();
            let limit = price_collection_limit;
            Box::pin(async move {
                tracing::info!("Running scheduled price collection job");
                match jobs::price_collection::collect_prices(&db, limit).await {
                    Ok(result) => {
                        if result.success {
                            tracing::info!(
                                "Price collection job completed successfully: {} assets tracked, {} prices collected, {} prices stored",
                                result.assets_tracked,
                                result.prices_collected,
                                result.prices_stored
                            );
                        } else {
                            tracing::error!(
                                "Price collection job failed: {}",
                                result.error.unwrap_or_else(|| "Unknown error".to_string())
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Price collection job failed with error: {}", e);
                    }
                }
            })
        })
        .expect("Failed to create price collection job");

        scheduler.add(job).await.expect("Failed to add price collection job to scheduler");
        tracing::info!("Price collection job scheduled successfully");
    } else {
        tracing::info!("Price collection job is disabled");
    }

    // Configure EOD snapshot job
    let eod_snapshot_enabled = std::env::var("EOD_SNAPSHOT_ENABLED")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);
    
    if eod_snapshot_enabled {
        let eod_snapshot_schedule = std::env::var("EOD_SNAPSHOT_SCHEDULE")
            .unwrap_or_else(|_| "0 0 23 * * *".to_string()); // Default: daily at 23:00 UTC
        
        tracing::info!(
            "Scheduling EOD snapshot job: schedule='{}'",
            eod_snapshot_schedule
        );

        let db_clone = db.clone();
        let job = Job::new_async(eod_snapshot_schedule.as_str(), move |_job_id, _scheduler| {
            let db = db_clone.clone();
            Box::pin(async move {
                tracing::info!("Running scheduled EOD snapshot job");
                match jobs::portfolio_snapshot::create_all_portfolio_snapshots(&db, None).await {
                    Ok(results) => {
                        let successful = results.iter().filter(|r| r.success).count();
                        let failed = results.iter().filter(|r| !r.success).count();
                        
                        tracing::info!(
                            "EOD snapshot job completed: {} portfolios processed, {} successful, {} failed",
                            results.len(),
                            successful,
                            failed
                        );
                        
                        // Log failures
                        for result in results.iter().filter(|r| !r.success) {
                            if let Some(error) = &result.error {
                                tracing::error!(
                                    "Failed to create EOD snapshot for portfolio {}: {}",
                                    result.portfolio_id,
                                    error
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("EOD snapshot job failed with error: {}", e);
                    }
                }
            })
        })
        .expect("Failed to create EOD snapshot job");

        scheduler.add(job).await.expect("Failed to add EOD snapshot job to scheduler");
        tracing::info!("EOD snapshot job scheduled successfully");
    } else {
        tracing::info!("EOD snapshot job is disabled");
    }

    // Start the scheduler
    scheduler.start().await.expect("Failed to start job scheduler");
    tracing::info!("Job scheduler started");

    // Keycloak configuration from environment variables
    let server_url = std::env::var("KEYCLOAK_SERVER")
        .unwrap_or_else(|_| "https://keycloak.example.com".to_string());
    let realm = std::env::var("KEYCLOAK_REALM").unwrap_or_else(|_| "myrealm".to_string());
    let client_id = std::env::var("KEYCLOAK_AUDIENCE").unwrap_or_else(|_| "account".to_string());

    tracing::info!(
        "Initializing Keycloak auth instance from: {}/realms/{}",
        server_url,
        realm
    );

    // Build Keycloak configuration
    let keycloak_config = KeycloakConfig {
        server: server_url.parse().expect("Invalid Keycloak server URL"),
        realm,
        retry: (5, 1), // 5 retries with 1 second delay
    };

    // Initialize Keycloak auth instance with OIDC discovery
    let keycloak_auth_instance = Arc::new(KeycloakAuthInstance::new(keycloak_config));

    // Build the Keycloak auth layer — any authenticated user
    let auth_layer = KeycloakAuthLayer::<String>::builder()
        .instance(keycloak_auth_instance.clone())
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![client_id.clone()])
        .required_roles(vec![]) // No required roles for basic authentication
        .build();

    // Build the admin auth layer — requires the "administrator" Keycloak realm role
    let admin_auth_layer = KeycloakAuthLayer::<String>::builder()
        .instance(keycloak_auth_instance.clone())
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![client_id])
        .required_roles(vec!["administrator".to_string()])
        .build();

    // Build protected routes that require authentication (any authenticated user)
    let protected_routes = Router::new()
        // Protected routes that require authentication
        .route("/api/me", get(get_user_info))
        .route("/api/protected", get(protected_endpoint))
        // Portfolio API routes (protected)
        .merge(handlers::portfolios::create_router())
        // Account sync API routes (protected)
        .merge(handlers::accounts::create_router())
        // Snapshot API routes (protected)
        .merge(handlers::snapshots::create_router())
        // Recommendation API routes (protected)
        .merge(handlers::recommendations::create_router())
        // Migration API routes (protected)
        .merge(handlers::migrations::create_router())
        .layer(auth_layer);

    // Build admin-only routes — require the "administrator" Keycloak realm role
    let admin_routes = Router::new()
        // Job management API routes (admin only)
        .merge(handlers::jobs::create_router())
        // EVM token registry API routes (admin only)
        .merge(handlers::evm_tokens::create_router())
        // EVM chain registry API routes (admin only)
        .merge(handlers::evm_chains::create_router())
        // Solana token registry API routes (admin only)
        .merge(handlers::solana_tokens::create_router())
        .layer(admin_auth_layer);

    // Build application with public and protected routes
    // Axum handles concurrent requests efficiently using Tokio's async runtime
    // Each request is processed asynchronously without blocking other requests
    let app = Router::new()
        // Swagger UI - publicly accessible (no authentication required)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Public routes (no auth required)
        .route("/", get(root))
        .route("/health", get(health))
        // Chains API routes (public)
        .merge(handlers::chains::create_router())
        // Merge protected routes
        .merge(protected_routes)
        // Merge admin-only routes
        .merge(admin_routes)
        // Apply database state to all routes
        .with_state(db);

    // Run the server
    let port_str = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "3001".to_string());
    let port = port_str.parse::<u16>()
        .unwrap_or_else(|_| panic!("SERVER_PORT must be a valid port number, got: {}", port_str));
    
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Starting server on {}", addr);
    tracing::info!("Swagger UI available at http://localhost:{}/swagger-ui", port);
    tracing::info!("Server ready to handle concurrent requests");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Root endpoint
///
/// Returns API information
#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "API information", body = String)
    ),
    tag = "crypto-pocket-butler"
)]
async fn root() -> &'static str {
    "Crypto Pocket Butler API"
}

/// Health check endpoint
///
/// Returns service health status
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Service health status", body = HealthResponse)
    ),
    tag = "crypto-pocket-butler"
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "crypto-pocket-butler-backend".to_string(),
    })
}

/// Get authenticated user information
///
/// Returns information about the authenticated user extracted from JWT token
#[utoipa::path(
    get,
    path = "/api/me",
    responses(
        (status = 200, description = "User information", body = UserInfo),
        (status = 401, description = "Unauthorized - invalid or missing JWT token")
    ),
    tag = "crypto-pocket-butler"
)]
async fn get_user_info(Extension(token): Extension<KeycloakToken<String>>) -> Json<UserInfo> {
    Json(UserInfo {
        user_id: token.subject,
        preferred_username: Some(token.extra.profile.preferred_username),
        email: Some(token.extra.email.email),
        email_verified: Some(token.extra.email.email_verified),
    })
}

/// Protected endpoint example
///
/// Example of a protected endpoint that requires authentication
#[utoipa::path(
    get,
    path = "/api/protected",
    responses(
        (status = 200, description = "Protected resource accessed", body = ProtectedResponse),
        (status = 401, description = "Unauthorized - invalid or missing JWT token")
    ),
    security(
        ("bearer_auth" = []),
        ("oauth2_client_credentials" = []),
        ("oauth2_authorization_code" = [])
    ),
    tag = "crypto-pocket-butler"
)]
async fn protected_endpoint(
    Extension(token): Extension<KeycloakToken<String>>,
) -> Json<ProtectedResponse> {
    Json(ProtectedResponse {
        message: "This is a protected endpoint".to_string(),
        user_id: token.subject,
    })
}

