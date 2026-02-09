use axum::{extract::Extension, response::Json, routing::get, Router};
use axum_keycloak_auth::{
    decode::KeycloakToken, instance::KeycloakAuthInstance, instance::KeycloakConfig,
    layer::KeycloakAuthLayer, PassthroughMode,
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
    ),
    components(
        schemas(UserInfo, ProtectedResponse, HealthResponse)
    ),
    tags(
        (name = "crypto-pocket-butler", description = "Crypto Pocket Butler API endpoints")
    ),
    info(
        title = "Crypto Pocket Butler API",
        version = "0.1.0",
        description = "API for managing crypto portfolio with Keycloak JWT authentication",
    ),
    servers(
        (url = "http://localhost:3000", description = "Local development server")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crypto_pocket_butler_backend=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

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

    // Build application with public and protected routes
    let app = Router::new()
        // Swagger UI - publicly accessible
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // Public routes (no auth required)
        .route("/", get(root))
        .route("/health", get(health))
        // Protected routes that require authentication
        .route("/api/me", get(get_user_info))
        .route("/api/protected", get(protected_endpoint))
        .layer(auth_layer);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Starting server on {}", addr);
    tracing::info!("Swagger UI available at http://localhost:3000/swagger-ui");

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

