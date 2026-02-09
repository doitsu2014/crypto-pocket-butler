use axum::{extract::Extension, response::Json, routing::get, Router};
use axum_keycloak_auth::{
    decode::KeycloakToken, instance::KeycloakAuthInstance, instance::KeycloakConfig,
    layer::KeycloakAuthLayer, PassthroughMode,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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
        .route("/", get(root))
        .route("/health", get(health))
        // Protected routes that require authentication
        .route("/api/me", get(get_user_info))
        .route("/api/protected", get(protected_endpoint))
        .layer(auth_layer);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Starting server on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Root endpoint - currently requires authentication due to the auth layer
// To make this public, move it to a separate router without the auth layer
async fn root() -> &'static str {
    "Crypto Pocket Butler API"
}

// Health check endpoint - currently requires authentication due to the auth layer
// To make this public, move it to a separate router without the auth layer
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "crypto-pocket-butler-backend"
    }))
}

// Protected endpoint - returns authenticated user information
async fn get_user_info(Extension(token): Extension<KeycloakToken<String>>) -> Json<Value> {
    Json(json!({
        "user_id": token.subject,
        "preferred_username": token.extra.profile.preferred_username,
        "email": token.extra.email.email,
        "email_verified": token.extra.email.email_verified,
    }))
}

// Example protected endpoint
async fn protected_endpoint(Extension(token): Extension<KeycloakToken<String>>) -> Json<Value> {
    Json(json!({
        "message": "This is a protected endpoint",
        "user_id": token.subject,
    }))
}

