use axum::{
    extract::Extension,
    middleware,
    response::Json,
    routing::get,
    Router,
};
use crypto_pocket_butler_backend::{auth, AuthUser, JwtValidator, KeycloakConfig};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "crypto_pocket_butler_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Configure Keycloak (these would typically come from environment variables)
    let keycloak_config = KeycloakConfig::new(
        std::env::var("KEYCLOAK_ISSUER")
            .unwrap_or_else(|_| "https://keycloak.example.com/realms/myrealm".to_string()),
        std::env::var("KEYCLOAK_AUDIENCE").unwrap_or_else(|_| "account".to_string()),
    );

    // Create JWT validator
    let jwt_validator = Arc::new(JwtValidator::new(keycloak_config));

    // Build application with protected routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        // Protected routes that require authentication
        .route("/api/me", get(get_user_info))
        .route("/api/protected", get(protected_endpoint))
        .route_layer(middleware::from_fn_with_state(
            jwt_validator.clone(),
            auth::require_auth,
        ))
        .with_state(jwt_validator);

    // Run the server
    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Starting server on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

// Public endpoint (no authentication required)
async fn root() -> &'static str {
    "Crypto Pocket Butler API"
}

// Public health check endpoint
async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "crypto-pocket-butler-backend"
    }))
}

// Protected endpoint - returns authenticated user information
async fn get_user_info(Extension(user): Extension<AuthUser>) -> Json<Value> {
    Json(json!({
        "user_id": user.user_id,
        "username": user.username,
        "email": user.email,
    }))
}

// Example protected endpoint
async fn protected_endpoint(Extension(user): Extension<AuthUser>) -> Json<Value> {
    Json(json!({
        "message": "This is a protected endpoint",
        "user_id": user.user_id,
    }))
}
