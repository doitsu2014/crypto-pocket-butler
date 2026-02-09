# backend-rust

Rust backend service for Crypto Pocket Butler:
- Axum HTTP API with Keycloak JWT authentication
- Scheduled workers for syncing accounts (planned)
- Postgres for normalized holdings + snapshots (planned)

## Features

### Keycloak JWT Authentication Middleware

The backend implements a robust JWT validation middleware for Keycloak integration:

- **JWKS Validation**: Fetches and caches Keycloak's public keys (JWKS) for JWT signature verification
- **Issuer & Audience Enforcement**: Validates that tokens are issued by the correct Keycloak realm and intended for this application
- **User Context Extraction**: Extracts user identity (`sub` claim as `user_id`) and adds it to request context
- **Automatic Token Refresh**: Caches JWKS for 1 hour and refreshes as needed

## Usage

### Environment Variables

Configure Keycloak connection using environment variables:

```bash
export KEYCLOAK_ISSUER="https://keycloak.example.com/realms/myrealm"
export KEYCLOAK_AUDIENCE="account"  # or your client_id
```

### Running the Server

```bash
cargo run
```

The server will start on `http://0.0.0.0:3000` with the following endpoints:

- `GET /` - Public root endpoint
- `GET /health` - Public health check
- `GET /api/me` - Protected endpoint that returns authenticated user info
- `GET /api/protected` - Example protected endpoint

### Using the Middleware in Your Code

```rust
use crypto_pocket_butler_backend::{auth, AuthUser, JwtValidator, KeycloakConfig};
use axum::{extract::Extension, middleware, routing::get, Router};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    // Configure Keycloak
    let keycloak_config = KeycloakConfig::new(
        "https://keycloak.example.com/realms/myrealm".to_string(),
        "account".to_string(),
    );

    // Create JWT validator
    let jwt_validator = Arc::new(JwtValidator::new(keycloak_config));

    // Build application with protected routes
    let app = Router::new()
        .route("/api/protected", get(protected_handler))
        // Add the auth middleware to protect routes
        .route_layer(middleware::from_fn_with_state(
            jwt_validator.clone(),
            auth::require_auth,
        ))
        .with_state(jwt_validator);

    // ... start server
}

// Access authenticated user in handlers
async fn protected_handler(Extension(user): Extension<AuthUser>) -> String {
    format!("Hello, user {}!", user.user_id)
}
```

## Architecture

### Authentication Flow (Expected Integration)

This backend provides the server-side JWT validation. The complete authentication flow would work as follows:

1. **Frontend → Keycloak**: User authenticates via OIDC Authorization Code + PKCE flow (frontend implementation not included)
2. **Frontend → Backend**: Sends `Authorization: Bearer <access_token>` with each request
3. **Backend Middleware**: 
   - Extracts token from Authorization header
   - Fetches JWKS from Keycloak (cached)
   - Validates token signature, issuer, audience, and expiry
   - Extracts user identity from `sub` claim
   - Adds `AuthUser` to request extensions for use in handlers

**Note**: This PR implements only the backend middleware. Frontend OIDC integration is a separate concern.

### Module Structure

```
src/
├── auth/
│   ├── mod.rs          - Public API exports
│   ├── config.rs       - Keycloak configuration
│   ├── jwt.rs          - JWT validation logic and JWKS handling
│   └── middleware.rs   - Axum middleware implementation
├── lib.rs              - Library exports
└── main.rs             - Example application
```

### AuthUser Context

The middleware adds an `AuthUser` struct to request extensions containing:

```rust
pub struct AuthUser {
    pub user_id: String,              // From JWT 'sub' claim
    pub username: Option<String>,     // From 'preferred_username'
    pub email: Option<String>,        // From 'email'
    pub claims: Claims,               // Full JWT claims
}
```

## Testing

Run the test suite:

```bash
cargo test
```

## Next Steps

- Define DB schema (accounts, assets, holdings, snapshots)
- Implement OKX read-only connector
- Add authorization checks (roles, resource ownership)
