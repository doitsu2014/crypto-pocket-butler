# API

Rust service for Crypto Pocket Butler:
- Axum HTTP API with Keycloak JWT authentication using `axum-keycloak-auth`
- OpenAPI documentation with Swagger UI using `utoipa`
- Scheduled workers for syncing accounts (planned)
- Postgres for normalized holdings + snapshots (planned)

## Features

### Concurrency & Performance

The API is designed for high-performance concurrent request handling:

- **Thread Pool**: Tokio multi-threaded runtime with work-stealing scheduler
- **Connection Pool**: Configurable database connection pool (default: 100 max connections)
- **Non-blocking I/O**: All operations are asynchronous using async/await
- **Scalability**: Handles thousands of concurrent requests efficiently

See [CONCURRENCY.md](./CONCURRENCY.md) for detailed information on thread pools, connection pools, and performance tuning.

### Keycloak JWT Authentication Middleware

The API uses the [`axum-keycloak-auth`](https://github.com/lpotthast/axum-keycloak-auth) library for robust JWT validation:

- **OIDC Discovery**: Automatically discovers Keycloak configuration via OIDC endpoints
- **JWKS Validation**: Fetches and caches Keycloak's public keys (JWKS) for JWT signature verification
- **Issuer & Audience Enforcement**: Validates that tokens are issued by the correct Keycloak realm and intended for this application
- **User Context Extraction**: Extracts user identity (`sub` claim as `user_id`) and adds it to request context
- **Role-Based Access Control**: Support for required role checking (optional)
- **Automatic Key Rotation**: Handles JWKS key rotation automatically

### OpenAPI Documentation

The API uses [`utoipa`](https://github.com/juhaku/utoipa) for compile-time OpenAPI documentation generation:

- **Automatic Schema Generation**: Generate OpenAPI schemas from Rust types
- **Interactive Swagger UI**: Browse and test API endpoints at `/swagger-ui`
- **Type-Safe Documentation**: Documentation stays in sync with code
- **OpenAPI 3.0 Spec**: Available at `/api-docs/openapi.json`

## Usage

### Environment Variables

The API supports configuration via environment variables or a `.env` file. For local development, create a `.env` file in the `api/` directory:

```bash
cp .env.example .env
# Edit .env with your actual configuration
```

The `.env` file will be automatically loaded when the application starts. You can also set environment variables directly in your shell.

**Configuration options:**

```bash
# Keycloak Configuration
export KEYCLOAK_SERVER="https://keycloak.example.com"
export KEYCLOAK_REALM="myrealm"
export KEYCLOAK_AUDIENCE="account"  # or your client_id

# Database Configuration
export DATABASE_URL="postgres://username:password@localhost/crypto_pocket_butler"

# Connection Pool (Optional - defaults shown)
export DB_MAX_CONNECTIONS=100        # Maximum concurrent database connections
export DB_MIN_CONNECTIONS=5          # Minimum idle connections
export DB_CONNECT_TIMEOUT_SECS=30    # Connection timeout
export DB_ACQUIRE_TIMEOUT_SECS=30    # Pool acquisition timeout
export DB_IDLE_TIMEOUT_SECS=600      # Idle connection timeout (10 min)
export DB_MAX_LIFETIME_SECS=1800     # Max connection lifetime (30 min)
```

**Note:** Environment variables set in your shell take precedence over values in the `.env` file.

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

1. **Web → Keycloak**: User authenticates via OIDC Authorization Code + PKCE flow (web implementation not included)
2. **Web → API**: Sends `Authorization: Bearer <access_token>` with each request
3. **API Middleware** (`axum-keycloak-auth`):
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
- ✅ Implement OKX read-only connector
- ✅ Implement account sync jobs
- ✅ Add API endpoints for manual account sync
- ✅ Implement portfolio snapshot jobs
- ✅ Add API endpoints for manual snapshot creation
- Add authorization checks (roles, resource ownership)
- Split routes into public and protected routers
- Implement API endpoints for CRUD operations on entities
- Add automatic scheduled syncing (cron-based)
- Add automatic scheduled EOD snapshots (cron-based)
- Add price data fetching
- Calculate total portfolio values in USD

## Account Sync Feature

The backend now includes a complete account synchronization system for fetching balances from exchanges.

### OKX Connector

A read-only connector for OKX exchange that:
- Uses API keys with read-only permissions
- Implements HMAC-SHA256 signature authentication
- Fetches spot balances from trading accounts
- Returns balance data including available and frozen amounts

See [connectors.md](connectors.md) for detailed documentation.

### Sync Jobs

Background jobs for synchronizing account balances:
- `sync_account(account_id)`: Sync a single account
- `sync_user_accounts(user_id)`: Sync all active accounts for a user

### API Endpoints

New endpoints for triggering account syncs:

- **POST /api/v1/accounts/{account_id}/sync**: Sync a specific account
- **POST /api/v1/accounts/sync-all**: Sync all accounts for the authenticated user

Both endpoints require authentication and return detailed sync results including:
- Number of holdings fetched
- Success/failure status
- Error messages if any

Example response:
```json
{
  "total": 3,
  "successful": 2,
  "failed": 1,
  "results": [
    {
      "account_id": "uuid",
      "success": true,
      "holdings_count": 5
    }
  ]
}
```

## Portfolio Snapshot Feature

The backend includes a comprehensive portfolio snapshot system for capturing point-in-time portfolio composition and valuation.

### Snapshot Jobs

Background jobs for creating portfolio snapshots:
- `create_portfolio_snapshot(portfolio_id, snapshot_date, snapshot_type)`: Create snapshot for a single portfolio
- `create_all_portfolio_snapshots(snapshot_date)`: Create EOD snapshots for all portfolios

### Snapshot Functionality

The snapshot system:
- Aggregates holdings from all accounts linked to a portfolio
- Calculates total portfolio value in USD
- Stores snapshot composition with metadata
- Supports different snapshot types: "eod" (End of Day), "manual", "hourly"
- Prevents duplicate snapshots for the same portfolio/date/type combination

### API Endpoints

New endpoints for creating snapshots:

- **POST /api/v1/portfolios/{portfolio_id}/snapshots**: Create a snapshot for a specific portfolio
- **POST /api/v1/snapshots/create-all**: Create snapshots for all portfolios owned by the authenticated user

Request body:
```json
{
  "snapshot_type": "manual",
  "snapshot_date": "2024-01-15"
}
```

Response:
```json
{
  "portfolio_id": "uuid",
  "snapshot_id": "uuid",
  "success": true,
  "holdings_count": 10,
  "total_value_usd": "50000.00"
}
```

### Scheduled EOD Snapshots

The snapshot system is designed to support scheduled execution for automated EOD (End of Day) snapshots:

1. **Job Function**: `create_all_portfolio_snapshots()` creates snapshots for all portfolios
2. **Cutover Time**: Can be configured via environment variable (future enhancement)
3. **Execution**: Designed to be called by a cron job or task scheduler

**Future Enhancement**: Add cron-based scheduling to automatically trigger EOD snapshots at configured time (e.g., using `tokio-cron-scheduler` crate).

## Top 100 Coins Collection Job

The backend includes an automated job system for collecting and tracking the top 100 cryptocurrencies by market cap.

### Features

- **Automated Collection**: Scheduled job that fetches top coins from CoinPaprika API
- **Asset Management**: Automatically creates or updates asset metadata in the `assets` table
- **Ranking Snapshots**: Stores daily ranking data in the `asset_rankings` table
- **Configurable Schedule**: Fully configurable via environment variables
- **Manual Trigger**: REST API endpoint for on-demand collection

### Configuration

Configure the job via environment variables (see `.env.example`):

```bash
# Enable/disable the job (default: true)
TOP_COINS_COLLECTION_ENABLED=true

# Cron schedule (default: daily at midnight UTC)
# Format: "sec min hour day_of_month month day_of_week year"
TOP_COINS_COLLECTION_SCHEDULE=0 0 0 * * *

# Number of top coins to collect (default: 100, max: 250)
TOP_COINS_COLLECTION_LIMIT=100
```

### API Endpoint

Manually trigger the collection job:

**POST /api/v1/jobs/collect-top-coins**

Request body:
```json
{
  "limit": 100
}
```

Response:
```json
{
  "success": true,
  "coins_collected": 100,
  "assets_created": 50,
  "assets_updated": 50,
  "rankings_created": 100
}
```

### Job Scheduler

The backend uses `tokio-cron-scheduler` for robust job scheduling:

- **Non-blocking**: Jobs run in the background without affecting API performance
- **Cron Expression Support**: Standard cron syntax for flexible scheduling
- **Automatic Retry**: Failed jobs are logged for monitoring
- **Database Connection Pooling**: Efficient concurrent database access

### Data Storage

The job stores data in two tables:

1. **assets**: Asset metadata (symbol, name, CoinPaprika ID, etc.)
2. **asset_rankings**: Daily ranking snapshots with market data (rank, price, market cap, volume, etc.)

### Use Cases

- **Market Reference**: Provides normalized asset data for portfolio tracking
- **UX Selection**: Powers asset selection dropdowns and autocomplete
- **Price Discovery**: Stores current prices for portfolio valuation
- **Trend Analysis**: Historical ranking data for market insights

## Contract Addresses Collection Job

The backend includes an automated job system for collecting and maintaining contract addresses for supported blockchain networks.

### Features

- **Automated Collection**: Scheduled job that fetches contract addresses from CoinPaprika API
- **Multi-Chain Support**: Supports Ethereum, BSC, Polygon, Arbitrum, Optimism, Avalanche, Base, Solana, and more
- **Address Mapping**: Maps wallet token balances to canonical assets using contract addresses
- **Smart Deduplication**: Prevents duplicate contract addresses with unique constraints
- **Configurable Schedule**: Fully configurable via environment variables
- **Manual Trigger**: REST API endpoint for on-demand collection
- **Rate Limiting**: Respects CoinPaprika API rate limits

### Configuration

Configure the job via environment variables (see `.env.example`):

```bash
# Enable/disable the job (default: true)
CONTRACT_ADDRESSES_COLLECTION_ENABLED=true

# Cron schedule (default: daily at 1:00 AM UTC)
# Format: "sec min hour day_of_month month day_of_week year"
CONTRACT_ADDRESSES_COLLECTION_SCHEDULE=0 0 1 * * *

# Optional limit on number of assets to process per run
# (omit or set to 0 for no limit)
# CONTRACT_ADDRESSES_COLLECTION_LIMIT=100
```

### How It Works

1. **Fetch Assets**: Retrieves all active assets that have a `coinpaprika_id` (stored in coingecko_id field for legacy reasons)
2. **Get Details**: For each asset, fetches detailed coin information from CoinPaprika's `/coins/{id}` endpoint
3. **Extract Addresses**: Parses the `contracts` field to extract contract addresses for each blockchain
4. **Normalize**: Converts CoinPaprika platform types to standard chain identifiers (e.g., `BEP20` → `bsc`)
5. **Store**: Upserts contract addresses into the `asset_contracts` table with metadata (token standard, decimals, verification status)

### Chain Mappings

CoinPaprika provides platform types in their contracts API that are normalized to standard chain identifiers used throughout the application:

| CoinPaprika Type | First Normalization | Final Chain ID |
|------------------|---------------------|----------------|
| `ERC20`          | `ethereum`          | `ethereum`     |
| `BEP20`          | `binance-smart-chain` | `bsc`        |
| `polygon`        | `polygon-pos`       | `polygon`      |

The normalization happens in two steps:
1. Connector normalizes CoinPaprika types → intermediate platform names
2. Job processing normalizes platform names → final chain identifiers
- `avalanche` → `avalanche`
- `base` → `base`
- `solana` → `solana`

### API Endpoint

Manually trigger the collection job:

**POST /api/v1/jobs/collect-contract-addresses**

Request body:
```json
{
  "limit": 100  // Optional: limit number of assets to process
}
```

Response:
```json
{
  "success": true,
  "assets_processed": 95,
  "contracts_created": 450,
  "contracts_updated": 23,
  "assets_skipped": 5
}
```

### Data Storage

The job stores contract addresses in the `asset_contracts` table:

- `asset_id`: Foreign key to the assets table
- `chain`: Blockchain network identifier (e.g., "ethereum", "bsc")
- `contract_address`: Token contract address on the chain
- `token_standard`: Token standard (e.g., "ERC20", "BEP20", "SPL")
- `decimals`: Token decimals (can override asset default)
- `is_verified`: Whether the contract is verified (always `true` for CoinPaprika data)

### Use Cases

- **Wallet Integration**: Enables mapping of on-chain token balances to known assets
- **Multi-Chain Support**: Tracks the same asset across different blockchains
- **Portfolio Sync**: Powers wallet balance synchronization for EVM and other chains
- **Asset Discovery**: Identifies tokens by their contract addresses
