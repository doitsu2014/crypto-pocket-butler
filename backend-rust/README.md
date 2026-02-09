# backend-rust

Rust backend service for Crypto Pocket Butler:
- Axum HTTP API with Keycloak JWT authentication using `axum-keycloak-auth`
- OpenAPI documentation with Swagger UI using `utoipa`
- Scheduled workers for syncing accounts (planned)
- Postgres for normalized holdings + snapshots (planned)

## Features

### Keycloak JWT Authentication Middleware

The backend uses the [`axum-keycloak-auth`](https://github.com/lpotthast/axum-keycloak-auth) library for robust JWT validation:

- **OIDC Discovery**: Automatically discovers Keycloak configuration via OIDC endpoints
- **JWKS Validation**: Fetches and caches Keycloak's public keys (JWKS) for JWT signature verification
- **Issuer & Audience Enforcement**: Validates that tokens are issued by the correct Keycloak realm and intended for this application
- **User Context Extraction**: Extracts user identity (`sub` claim as `user_id`) and adds it to request context
- **Role-Based Access Control**: Support for required role checking (optional)
- **Automatic Key Rotation**: Handles JWKS key rotation automatically

### OpenAPI Documentation

The backend uses [`utoipa`](https://github.com/juhaku/utoipa) for compile-time OpenAPI documentation generation:

- **Automatic Schema Generation**: Generate OpenAPI schemas from Rust types
- **Interactive Swagger UI**: Browse and test API endpoints at `/swagger-ui`
- **Type-Safe Documentation**: Documentation stays in sync with code
- **OpenAPI 3.0 Spec**: Available at `/api-docs/openapi.json`

## Usage

### Environment Variables

Configure Keycloak connection using environment variables:

```bash
export KEYCLOAK_SERVER="https://keycloak.example.com"
export KEYCLOAK_REALM="myrealm"
export KEYCLOAK_AUDIENCE="account"  # or your client_id
```

### Running the Server

```bash
cargo run
```

The server will start on `http://0.0.0.0:3000` with the following endpoints:

- `GET /` - Root endpoint (requires authentication)
- `GET /health` - Health check (requires authentication)
- `GET /api/me` - Protected endpoint that returns authenticated user info
- `GET /api/protected` - Example protected endpoint
- `GET /swagger-ui` - **Interactive Swagger UI** (publicly accessible)
- `GET /api-docs/openapi.json` - OpenAPI specification (publicly accessible)

**Note**: In the current setup, most routes require authentication except for Swagger UI and OpenAPI spec. To have truly public endpoints, create a separate router without the auth layer and merge it with the protected router.

### Using the Library in Your Code

```rust
use axum::{extract::Extension, routing::get, Router};
use axum_keycloak_auth::{
    decode::KeycloakToken,
    instance::{KeycloakAuthInstance, KeycloakConfig},
    layer::KeycloakAuthLayer,
    PassthroughMode,
};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// Define response schema
#[derive(Serialize, Deserialize, ToSchema)]
struct UserInfo {
    user_id: String,
    username: Option<String>,
}

// Document endpoint with utoipa
#[utoipa::path(
    get,
    path = "/api/me",
    responses(
        (status = 200, description = "User information", body = UserInfo),
        (status = 401, description = "Unauthorized")
    ),
    security(
        ("bearer_auth" = [])
    )
)]
async fn get_user_info(
    Extension(token): Extension<KeycloakToken<String>>,
) -> Json<UserInfo> {
    Json(UserInfo {
        user_id: token.subject,
        username: Some(token.extra.profile.preferred_username),
    })
}

#[derive(OpenApi)]
#[openapi(
    paths(get_user_info),
    components(schemas(UserInfo)),
    modifiers(&SecurityAddon)
)]
struct ApiDoc;

use utoipa::Modify;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}

#[tokio::main]
async fn main() {
    // Build Keycloak configuration
    let keycloak_config = KeycloakConfig {
        server: "https://keycloak.example.com".parse().unwrap(),
        realm: "myrealm".to_string(),
        retry: (5, 1), // 5 retries with 1 second delay
    };

    // Initialize Keycloak auth instance with OIDC discovery
    let keycloak_auth_instance = Arc::new(KeycloakAuthInstance::new(keycloak_config));

    // Build the auth layer
    let auth_layer = KeycloakAuthLayer::<String>::builder()
        .instance(keycloak_auth_instance.clone())
        .passthrough_mode(PassthroughMode::Block)
        .expected_audiences(vec!["account".to_string()])
        .required_roles(vec![]) // Optional: add required roles
        .build();

    // Build application with protected routes
    let app = Router::new()
        .route("/api/protected", get(protected_handler))
        .layer(auth_layer);

    // ... start server
}

// Access authenticated user in handlers via KeycloakToken
async fn protected_handler(
    Extension(token): Extension<KeycloakToken<String>>,
) -> String {
    format!("Hello, user {}!", token.subject)
}
```

## Architecture

### Authentication Flow (Expected Integration)

This backend provides the server-side JWT validation using the `axum-keycloak-auth` library. The complete authentication flow works as follows:

1. **Frontend → Keycloak**: User authenticates via OIDC Authorization Code + PKCE flow (frontend implementation not included)
2. **Frontend → Backend**: Sends `Authorization: Bearer <access_token>` with each request
3. **Backend Middleware** (`axum-keycloak-auth`):
   - Extracts token from Authorization header
   - Performs OIDC discovery to get Keycloak configuration
   - Fetches JWKS from Keycloak (cached with automatic rotation)
   - Validates token signature, issuer, audience, and expiry
   - Extracts user identity from `sub` claim
   - Adds `KeycloakToken` to request extensions for use in handlers

**Note**: This implementation uses the well-maintained `axum-keycloak-auth` library instead of a custom implementation, providing production-ready JWT validation with automatic OIDC discovery and JWKS rotation.

### Module Structure

```
src/
├── lib.rs              - Library exports (re-exports axum-keycloak-auth)
└── main.rs             - Example application with Keycloak auth
```

### KeycloakToken Context

The middleware adds a `KeycloakToken<String>` struct to request extensions containing:

```rust
pub struct KeycloakToken<R> {
    pub subject: String,              // From JWT 'sub' claim (user ID)
    pub extra: ProfileAndEmail,       // Standard JWT claims
    pub roles: Vec<R>,                // User roles
    // ... other fields
}

pub struct ProfileAndEmail {
    pub profile: Profile,             // User profile (username, name, etc.)
    pub email: Email,                 // Email and verification status
}

pub struct Profile {
    pub preferred_username: String,   // Username
    pub given_name: Option<String>,   // First name
    pub family_name: Option<String>,  // Last name
    pub full_name: Option<String>,    // Full name
}

pub struct Email {
    pub email: String,                // Email address
    pub email_verified: bool,         // Email verification status
}
```

## Testing

Run the test suite:

```bash
cargo test
```

## Database Schema

The backend uses SeaORM with PostgreSQL for data persistence. The schema includes:

### Tables

1. **users** - User accounts linked to Keycloak
   - `id` (UUID, PK)
   - `keycloak_user_id` (String, unique) - Links to Keycloak user
   - `email` (String, optional)
   - `preferred_username` (String, optional)
   - `created_at`, `updated_at` (Timestamptz)

2. **accounts** - Exchange accounts, wallets, or DeFi protocols
   - `id` (UUID, PK)
   - `user_id` (UUID, FK to users)
   - `name` (String) - User-defined name
   - `account_type` (String) - "exchange", "wallet", "defi"
   - `exchange_name` (String, optional) - e.g., "okx", "binance"
   - `api_key_encrypted`, `api_secret_encrypted`, `passphrase_encrypted` (String, optional) - Encrypted credentials
   - `wallet_address` (String, optional)
   - `is_active` (Boolean)
   - `last_synced_at` (Timestamptz, optional)
   - `created_at`, `updated_at` (Timestamptz)

3. **portfolios** - User-defined portfolio groupings
   - `id` (UUID, PK)
   - `user_id` (UUID, FK to users)
   - `name` (String)
   - `description` (Text, optional)
   - `is_default` (Boolean) - Only one default portfolio per user
   - `created_at`, `updated_at` (Timestamptz)

4. **portfolio_accounts** - Join table for many-to-many relationship
   - `id` (UUID, PK)
   - `portfolio_id` (UUID, FK to portfolios)
   - `account_id` (UUID, FK to accounts)
   - `added_at` (Timestamptz)

5. **snapshots** - Point-in-time portfolio snapshots (EOD, manual, etc.)
   - `id` (UUID, PK)
   - `portfolio_id` (UUID, FK to portfolios)
   - `snapshot_date` (Date)
   - `snapshot_type` (String) - "eod", "manual", "hourly"
   - `total_value_usd` (Decimal)
   - `holdings` (JSON) - Array of asset holdings
   - `metadata` (JSON, optional) - Exchange rates, etc.
   - `created_at` (Timestamptz)

### Running Migrations

Set up your database connection:

```bash
export DATABASE_URL="postgres://username:password@localhost/crypto_pocket_butler"
```

Run migrations:

```bash
cd migration
cargo run
```

Or use the migration CLI:

```bash
# Run all pending migrations
cargo run -- up

# Rollback last migration
cargo run -- down

# Check migration status
cargo run -- status

# Refresh (down + up)
cargo run -- refresh

# Reset database (down all + up all)
cargo run -- reset
```

### Using Entities in Code

```rust
use crypto_pocket_butler_backend::entities::{users, accounts, portfolios};
use sea_orm::*;

// Query user by Keycloak ID
let user = users::Entity::find()
    .filter(users::Column::KeycloakUserId.eq("keycloak-user-id"))
    .one(&db)
    .await?;

// Query user's accounts
let accounts = accounts::Entity::find()
    .filter(accounts::Column::UserId.eq(user_id))
    .all(&db)
    .await?;

// Query portfolios with their accounts (many-to-many)
let portfolios_with_accounts = portfolios::Entity::find()
    .filter(portfolios::Column::UserId.eq(user_id))
    .find_with_related(accounts::Entity)
    .all(&db)
    .await?;
```

## Library Documentation

For complete documentation on `axum-keycloak-auth`, see:
- [Crate documentation](https://docs.rs/axum-keycloak-auth)
- [GitHub repository](https://github.com/lpotthast/axum-keycloak-auth)

For SeaORM documentation:
- [SeaORM Documentation](https://www.sea-ql.org/SeaORM/)
- [SeaORM Cookbook](https://www.sea-ql.org/sea-orm-cookbook/)

## Next Steps

- ✅ Define DB schema (accounts, portfolios, snapshots)
- Implement OKX read-only connector
- Add authorization checks (roles, resource ownership)
- Split routes into public and protected routers
- Implement API endpoints for CRUD operations on entities
