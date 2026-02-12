use axum::{extract::Extension, response::Json, routing::get, Router};
use axum_keycloak_auth::{
    decode::KeycloakToken, instance::KeycloakAuthInstance, instance::KeycloakConfig,
    layer::KeycloakAuthLayer, PassthroughMode,
};
use crypto_pocket_butler_backend::{db::DbConfig, handlers};
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
        handlers::accounts::list_accounts_handler,
        handlers::accounts::get_account_handler,
        handlers::accounts::create_account_handler,
        handlers::accounts::update_account_handler,
        handlers::accounts::delete_account_handler,
        handlers::accounts::sync_account_handler,
        handlers::accounts::sync_all_accounts_handler,
        handlers::snapshots::create_portfolio_snapshot_handler,
        handlers::snapshots::create_all_user_snapshots_handler,
        handlers::snapshots::list_portfolio_snapshots_handler,
        handlers::recommendations::list_portfolio_recommendations,
        handlers::recommendations::get_recommendation,
        handlers::recommendations::create_recommendation,
        handlers::recommendations::generate_mock_recommendations,
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
            handlers::accounts::CreateAccountRequest,
            handlers::accounts::UpdateAccountRequest,
            handlers::accounts::AccountResponse,
            handlers::accounts::SyncAccountRequest,
            handlers::accounts::SyncResultResponse,
            handlers::accounts::SyncAllAccountsResponse,
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
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "crypto-pocket-butler", description = "Crypto Pocket Butler API endpoints"),
        (name = "portfolios", description = "Portfolio management endpoints"),
        (name = "accounts", description = "Account management and sync endpoints"),
        (name = "snapshots", description = "Portfolio snapshot endpoints"),
        (name = "recommendations", description = "Portfolio recommendation endpoints")
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
        (url = "http://localhost:3000", description = "Local development server")
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

    // Build the Keycloak auth layer
    let auth_layer = KeycloakAuthLayer::<String>::builder()
        .instance(keycloak_auth_instance.clone())
        .passthrough_mode(PassthroughMode::Block)
        .persist_raw_claims(false)
        .expected_audiences(vec![client_id])
        .required_roles(vec![]) // No required roles for basic authentication
        .build();

    // Build protected routes that require authentication
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
        .layer(auth_layer);

    // Build application with public and protected routes
    // Axum handles concurrent requests efficiently using Tokio's async runtime
    // Each request is processed asynchronously without blocking other requests
    let app = Router::new()
        // Swagger UI - publicly accessible (no authentication required)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Public routes (no auth required)
        .route("/", get(root))
        .route("/health", get(health))
        // Merge protected routes
        .merge(protected_routes)
        // Apply database state to all routes
        .with_state(db);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Starting server on {}", addr);
    tracing::info!("Swagger UI available at http://localhost:3000/swagger-ui");
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
    security(
        ("bearer_auth" = [])
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
        ("bearer_auth" = [])
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

