use axum::{extract::Extension, response::Json, routing::get, Router};
use axum_keycloak_auth::{
    decode::KeycloakToken, instance::KeycloakAuthInstance, instance::KeycloakConfig,
    layer::KeycloakAuthLayer, PassthroughMode,
};
use apalis_board_api::ui::ServeUI;
use apalis_postgres::PostgresStorage;
use crypto_pocket_butler_backend::{
    application::{
        services::{account_service::AccountService, portfolio_service::PortfolioService},
        usecases::{
            account_usecases::AccountUseCases, chain_usecases::ChainUseCases,
            portfolio_usecases::PortfolioUseCases,
        },
    },
    db::DbConfig,
    handlers,
    infrastructure::persistence::{
        AccountRepositoryImpl, ChainRepositoryImpl, PortfolioRepositoryImpl,
    },
    jobs, transport,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
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
        handlers::jobs::get_jobs_status_handler,
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
            handlers::jobs::ApalisWorkerStatus,
            handlers::jobs::ApalisJobsStatusResponse,
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
    modifiers(&SecurityAddon, &ServerAddon),
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
)]
struct ApiDoc;

use utoipa::Modify;

struct SecurityAddon;

struct ServerAddon;

impl Modify for ServerAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let mut servers = Vec::new();

        // If a public URL is configured (e.g. via nginx domain mapping), put it first
        // so Swagger UI uses it as the default server.
        // Set API_PUBLIC_URL=https://api.yourdomain.com in your deployment environment.
        if let Ok(url) = std::env::var("API_PUBLIC_URL") {
            servers.push(
                utoipa::openapi::ServerBuilder::new()
                    .url(url)
                    .description(Some("Deployed server"))
                    .build(),
            );
        }

        servers.push(
            utoipa::openapi::ServerBuilder::new()
                .url("http://localhost:3001")
                .description(Some("Local development server (standalone)"))
                .build(),
        );
        servers.push(
            utoipa::openapi::ServerBuilder::new()
                .url("http://localhost:3000")
                .description(Some("Docker backend server"))
                .build(),
        );

        openapi.servers = Some(servers);
    }
}

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

    // Extract the underlying sqlx PgPool from the sea-orm DatabaseConnection.
    // apalis-postgres uses sqlx directly for its own migrations and storage.
    let pg_pool = db.get_postgres_connection_pool().clone();

    // Run apalis-postgres schema migrations (creates the `apalis_jobs` table
    // and supporting objects if they don't already exist).
    tracing::info!("Running apalis-postgres schema setup...");
    PostgresStorage::setup(&pg_pool)
        .await
        .expect("Failed to run apalis-postgres migrations");
    tracing::info!("apalis-postgres schema ready");

    // Initialize and start Apalis job workers
    tracing::info!("Initializing Apalis job workers...");
    let components = jobs::apalis_runner::build_monitor(db.clone(), pg_pool);
    let board_router = jobs::apalis_runner::build_board_router(
        components.fetch_storage,
        components.snapshot_storage,
    );
    tokio::spawn(async move {
        if let Err(e) = components.monitor.run().await {
            tracing::error!("Apalis job monitor stopped with error: {}", e);
        }
    });
    tracing::info!("Apalis job workers started");

    // ─── Build application-layer use cases ───────────────────────────────────────
    //
    // Use cases are constructed once at startup and shared across all matching
    // handlers via Axum's `Extension` extractor.  Each use case holds an `Arc`
    // to its repository implementation so no allocations happen per-request.
    tracing::info!("Initializing application-layer use cases...");

    let account_use_cases = {
        let repo = Arc::new(AccountRepositoryImpl::new(db.clone()));
        let service = Arc::new(AccountService::new(repo));
        Arc::new(AccountUseCases::new(service))
    };

    let portfolio_use_cases = {
        let repo = Arc::new(PortfolioRepositoryImpl::new(db.clone()));
        let service = Arc::new(PortfolioService::new(repo));
        Arc::new(PortfolioUseCases::new(service))
    };

    let chain_use_cases = {
        let repo = Arc::new(ChainRepositoryImpl::new(db.clone()));
        Arc::new(ChainUseCases::new(repo))
    };

    tracing::info!("Application-layer use cases initialized");

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
        // Domain-specific protected routes
        .merge(transport::http::routes::protected_routes())
        .layer(auth_layer);

    // Build admin-only routes — require the "administrator" Keycloak realm role
    // Clone the layer so we can apply it to the board routes separately
    // (board routes are Router<()> while other admin routes are Router<DatabaseConnection>)
    let admin_auth_layer_board = admin_auth_layer.clone();

    let admin_routes = Router::new()
        // Domain-specific admin-only routes
        .merge(transport::http::routes::admin_routes())
        .layer(admin_auth_layer);

    // apalis-board routes are Router<()> (they don't use axum State extractor)
    // so they must be built separately and merged into the app after .with_state(db).
    //
    // Board API is mounted at /api/v1 so the pre-built board SPA (which hard-codes
    // /api/v1 as its API base URL) can reach these routes:
    //   GET /api/v1/queues            — list queues
    //   GET /api/v1/overview          — aggregate stats
    //   GET /api/v1/workers           — all workers
    //   GET /api/v1/tasks             — all tasks
    //   GET /api/v1/events            — SSE tracing stream
    //   GET /api/v1/queues/{q}/*      — per-queue routes
    //
    // The board SPA's static assets (JS/WASM/CSS) are loaded via absolute paths
    // (/apalis-board-web-*.js etc.) so a fallback_service catches those requests.
    let board_admin_routes = Router::new()
        .nest("/api/v1", board_router)
        .route("/admin/jobs", get(|| async { axum::response::Redirect::temporary("/admin/jobs/") }))
        .fallback_service(ServeUI::new())
        .layer(admin_auth_layer_board);

    // Build application with public and protected routes
    // Axum handles concurrent requests efficiently using Tokio's async runtime
    // Each request is processed asynchronously without blocking other requests
    let app = Router::new()
        // Swagger UI - publicly accessible (no authentication required)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Public routes (no auth required)
        .route("/", get(root))
        .route("/health", get(health))
        // Public domain routes
        .merge(transport::http::routes::public_routes())
        // Merge protected routes
        .merge(protected_routes)
        // Merge admin-only routes
        .merge(admin_routes)
        // Apply database state to all routes
        .with_state(db)
        // Inject use cases as shared Extensions (available to all handlers)
        .layer(Extension(account_use_cases))
        .layer(Extension(portfolio_use_cases))
        .layer(Extension(chain_use_cases))
        // Merge board admin routes (Router<()>) AFTER .with_state() so types align
        .merge(board_admin_routes);

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

