# Architecture — Crypto Pocket Butler

> Consolidated reference for system design, service topology, data models, and request flows.

---

## Table of Contents

1. [System Overview](#1-system-overview)
2. [High-Level Topology](#2-high-level-topology)
3. [Technology Stack](#3-technology-stack)
4. [Service Breakdown](#4-service-breakdown)
5. [Authentication & Authorization](#5-authentication--authorization)
6. [Backend (API) Architecture](#6-backend-api-architecture)
7. [Frontend (Web) Architecture](#7-frontend-web-architecture)
8. [Database Schema](#8-database-schema)
9. [Background Job System](#9-background-job-system)
10. [External Connectors](#10-external-connectors)
11. [Infrastructure](#11-infrastructure)

---

## 1. System Overview

Crypto Pocket Butler is a **crypto portfolio management system** that:

- Aggregates balances from exchange accounts (OKX) and EVM wallets (Ethereum, Arbitrum, Optimism, Base, BSC)
- Normalizes holdings into a unified schema (asset, quantity, USD value)
- Organizes holdings into user-defined **portfolios** with target allocations
- Takes **end-of-day (EOD) snapshots** for historical tracking
- Generates **rebalancing recommendations** based on drift from target allocations
- Provides an **admin layer** to configure EVM chains and token registries
- Collects market data (prices, rankings) from CoinGecko and CoinPaprika on a schedule

Base currency: **USD**. Spot-focused (occasional small futures exposure).

---

## 2. High-Level Topology

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Browser / Client                           │
└────────────────────────────┬────────────────────────────────────────┘
                             │  HTTPS
                             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    web  (Next.js  :3001)                            │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │  App Router Pages → React Components → TanStack Query Hooks  │  │
│  │  /api/backend/[...path]  →  proxy to API with Bearer token   │  │
│  └───────────────────────────────────────────────────────────────┘  │
└──────────────────────────────┬──────────────────────────────────────┘
                               │  HTTP Bearer JWT
          ┌────────────────────┴────────────────────┐
          ▼                                         ▼
┌──────────────────┐                  ┌─────────────────────────────┐
│  keycloak :8080  │  JWKS / OIDC     │   api  (Rust/Axum  :3000)   │
│  (OIDC provider) │◄─────────────────│   JWT validation middleware  │
└──────────────────┘                  │   Handlers → SeaORM → DB    │
         │                            │   Background Jobs (cron)    │
         │ PostgreSQL backend         └──────────────┬──────────────┘
         ▼                                           │
┌──────────────────┐                  ┌─────────────▼──────────────┐
│  postgres (KC)   │                  │  postgres  :5432            │
│  (keycloak DB)   │                  │  (app database)             │
└──────────────────┘                  └─────────────────────────────┘

External APIs (outbound only):
  OKX REST API        → exchange balances
  EVM RPC (per chain) → on-chain token balances
  CoinGecko API       → top coins & prices
  CoinPaprika API     → coin metadata & contract addresses
```

---

## 3. Technology Stack

### Backend (`api/`)

| Concern | Library / Version |
|---------|------------------|
| HTTP framework | Axum 0.8 |
| Async runtime | Tokio 1 (multi-threaded) |
| ORM | SeaORM 1.1 |
| Database | PostgreSQL 16 |
| Authentication | axum-keycloak-auth 0.8 (OIDC/JWT) |
| OpenAPI | utoipa 5 + utoipa-swagger-ui 9 |
| HTTP client | reqwest 0.12 |
| EVM interaction | alloy 1.6 |
| Job scheduling | tokio-cron-scheduler 0.13 |
| Decimal math | rust_decimal 1.35 |
| Caching | moka 0.12 (in-memory TTL) |
| Serialization | serde 1.0 + serde_json 1.0 |
| Error types | thiserror 2.0 |
| Logging | tracing + tracing-subscriber |

### Frontend (`web/`)

| Concern | Library / Version |
|---------|------------------|
| Framework | Next.js 16 (App Router) |
| UI library | React 19.2 |
| Language | TypeScript 5 |
| Authentication | NextAuth.js 5.0 (Keycloak OIDC) |
| Data fetching | TanStack React Query 5.90 |
| Styling | TailwindCSS 4 + PostCSS |
| HTTP client | Axios (via `lib/api-client.ts`) |

### Infrastructure

| Concern | Technology |
|---------|-----------|
| Containerization | Docker + Docker Compose |
| Identity provider | Keycloak 26.0 |
| Databases | PostgreSQL 16-alpine |

---

## 4. Service Breakdown

Docker Compose defines five services on a shared bridge network (`crypto-pocket-butler-network`):

| Service | Image / Source | Port | Role |
|---------|---------------|------|------|
| `postgres` | `postgres:16-alpine` | 5432 | App database |
| `keycloak` | `quay.io/keycloak/keycloak:26.0` | 8080 | OIDC identity provider |
| `keycloak-init` | `keycloak/` | — | One-shot realm/client bootstrapper |
| `api` | `api/Dockerfile` | 3000 | Rust/Axum backend |
| `web` | `web/Dockerfile` | 3001 | Next.js frontend |

Container naming convention: `crypto-pocket-butler-{service}` (e.g., `crypto-pocket-butler-api`).

---

## 5. Authentication & Authorization

### OIDC Flow

```
User browser
  │
  ├─1─► web (NextAuth.js)
  │        │  Authorization Code + PKCE redirect
  ├─2─────►│  Keycloak login page
  │        │◄─ access_token + id_token
  │◄───────┘
  │  (session stored in httpOnly cookie)
  │
  ├─3─► web page makes API call
  │        │  GET /api/backend/portfolios
  │        │  NextAuth appends Authorization: Bearer <access_token>
  ├─4─────►│  api  receives Bearer JWT
  │        │  axum-keycloak-auth middleware:
  │        │    1. OIDC discovery → Keycloak config
  │        │    2. JWKS fetch (cached, auto-rotated)
  │        │    3. Validate signature, issuer, audience, expiry
  │        │    4. Extract sub → user_id
  │        │    5. Inject KeycloakToken into request extensions
  │        │  Handler runs with authenticated user context
```

### Role-Based Access Control

- Standard routes: any authenticated user
- Admin routes (`/api/admin/*`): require `admin` Keycloak role (enforced in handler layer)
- Public routes: `/health`, `/api/chains`, `/swagger-ui`, `/api-docs/openapi.json`

### User Identity

Users are identified by their Keycloak `sub` claim. On first request the API performs a **get-or-create** in the `users` table (`api/src/helpers/auth.rs`), linking `keycloak_user_id` to an internal UUID.

---

## 6. Backend (API) Architecture

### Module Structure

```
api/src/
├── main.rs               # Server init, router assembly, job scheduler startup
├── lib.rs                # Library re-exports
├── db.rs                 # SeaORM connection pool setup
├── cache.rs              # Moka in-memory cache utilities
├── concurrency/          # Structured concurrency helpers
├── handlers/             # HTTP request handlers (one file per domain)
│   ├── portfolios.rs     # Portfolio CRUD + allocation construction
│   ├── accounts.rs       # Account CRUD + sync
│   ├── snapshots.rs      # Snapshot creation + retrieval
│   ├── recommendations.rs# Rebalancing recommendations
│   ├── evm_chains.rs     # EVM chain admin
│   ├── evm_tokens.rs     # EVM token admin
│   ├── chains.rs         # Public: list supported chains
│   ├── jobs.rs           # Manual job triggers
│   ├── migrations.rs     # Migration trigger endpoint
│   └── error.rs          # Centralized error response types
├── domain/               # Business logic / domain models
│   ├── allocation.rs     # Allocation computation & drift detection
│   ├── holdings.rs       # Holdings normalization
│   └── snapshot.rs       # Snapshot model definitions
├── entities/             # SeaORM auto-generated DB entities
│   ├── users.rs
│   ├── accounts.rs
│   ├── portfolios.rs
│   ├── portfolio_accounts.rs
│   ├── snapshots.rs
│   ├── assets.rs
│   ├── asset_contracts.rs
│   ├── asset_prices.rs
│   ├── evm_chains.rs
│   ├── evm_tokens.rs
│   └── recommendations.rs
├── connectors/           # External service clients
│   ├── okx.rs            # OKX exchange (HMAC-SHA256)
│   ├── evm.rs            # EVM RPC wallet balance fetching
│   ├── coingecko.rs      # CoinGecko price & coin data
│   └── coinpaprika.rs    # CoinPaprika metadata & contracts
├── helpers/
│   ├── asset_identity.rs # Asset matching / deduplication across exchanges
│   ├── balance_normalization.rs # Normalize raw API balances to standard form
│   └── auth.rs           # get-or-create user from Keycloak JWT
└── jobs/
    ├── runner.rs          # Job framework (scheduling, registration)
    ├── fetch_all_coins.rs # Fetch all coins from CoinPaprika + CoinGecko
    ├── price_collection.rs# Collect market prices (top N assets)
    ├── account_sync.rs    # Sync all active user accounts
    └── portfolio_snapshot.rs # Create EOD snapshots for all portfolios
```

### Request Processing Pipeline

```
Incoming HTTP Request
  │
  ├─ Public Router (no auth)
  │    /health, /swagger-ui, /api-docs/openapi.json, /api/chains
  │
  └─ Protected Router
       │
       KeycloakAuthLayer (JWT validation)
       │
       Handler function
         │
         ├─ Extract user_id from KeycloakToken
         ├─ Validate resource ownership
         ├─ SeaORM database operations
         └─ JSON response
```

### API Route Map

| Method | Path | Handler | Auth |
|--------|------|---------|------|
| GET | `/health` | health check | public |
| GET | `/api/chains` | list supported chains | public |
| GET | `/swagger-ui` | Swagger UI | public |
| GET | `/api/me` | authenticated user info | JWT |
| GET/POST/PUT/DELETE | `/api/portfolios/*` | portfolio management | JWT |
| GET/POST/PUT/DELETE | `/api/accounts/*` | account management | JWT |
| POST | `/api/accounts/:id/sync` | sync single account | JWT |
| POST | `/api/accounts/sync-all` | sync all accounts | JWT |
| GET/POST | `/api/portfolios/:id/snapshots` | snapshot management | JWT |
| GET/POST | `/api/portfolios/:id/recommendations` | recommendations | JWT |
| GET/POST/PUT/DELETE | `/api/admin/evm-chains/*` | EVM chain admin | JWT + admin role |
| GET/POST/PUT/DELETE | `/api/admin/evm-tokens/*` | EVM token admin | JWT + admin role |
| POST | `/api/jobs/*` | manual job triggers | JWT |
| POST | `/api/migrations` | run DB migrations | JWT |

---

## 7. Frontend (Web) Architecture

### Page Structure (App Router)

```
web/app/
├── layout.tsx              # Root layout (providers: session, query, toast)
├── page.tsx                # Home / landing page
├── auth/signin/            # Login page (redirects to Keycloak)
├── dashboard/              # Portfolio dashboard
├── accounts/
│   ├── page.tsx            # Account list
│   └── [id]/page.tsx       # Account detail
├── portfolios/
│   ├── page.tsx            # Portfolio list
│   └── [id]/
│       ├── page.tsx        # Portfolio detail + allocation view
│       ├── settings/       # Target allocation & guardrails
│       ├── snapshots/      # Historical snapshot browser
│       └── recommendations/# Rebalancing suggestions
├── admin/
│   ├── page.tsx            # Admin dashboard
│   ├── evm-chains/         # EVM chain CRUD
│   └── evm-tokens/         # EVM token CRUD
├── settings/               # User settings
└── api/
    ├── auth/[...nextauth]/  # NextAuth.js handler
    └── backend/[...path]/   # Proxy to Rust API (attaches Bearer token)
```

### Provider Stack

```
<SessionProviderWrapper>       ← NextAuth.js session
  <QueryClientProvider>        ← TanStack Query cache
    <ToastProvider>            ← Toast notifications
      <AppLayout>              ← Navigation, sidebar
        {children}             ← Page content
```

### Data Layer Pattern

```
Page / Component
  │  imports
  ▼
Custom TanStack Query Hook  (web/hooks/useAccounts.ts, usePortfolios.ts, …)
  │  uses
  ▼
api-client.ts               (web/lib/api-client.ts — Axios wrapper)
  │  HTTP POST/GET/…  to
  ▼
/api/backend/[...path]      (Next.js catch-all proxy route)
  │  attaches Bearer token, forwards to
  ▼
api service :3000           (Rust/Axum backend)
```

### Component Organization

```
web/components/
├── AppLayout.tsx            # Main shell (nav, sidebar, layout)
├── SessionProviderWrapper.tsx
├── Toast.tsx / ToastContext.tsx
├── Loading.tsx
├── ErrorAlert.tsx
├── EmptyState.tsx
└── portfolio/
    ├── AllocationPie.tsx    # Pie chart (allocation by asset)
    ├── AllocationBar.tsx    # Horizontal bar chart
    ├── HoldingsTable.tsx    # Holdings data table
    └── DriftBadge.tsx       # Drift % badge vs target allocation
```

---

## 8. Database Schema

### Entity Relationship Overview

```
users ──< accounts ──< portfolio_accounts >── portfolios
                                                  │
                                           ├── snapshots
                                           ├── recommendations
                                           └── portfolio_allocations

assets ──< asset_contracts
       ──< asset_prices

evm_chains ──< evm_tokens
```

### Core Tables

#### `users`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | Internal user ID |
| keycloak_user_id | TEXT UNIQUE | Maps to Keycloak `sub` claim |
| email | TEXT | From JWT claims |
| preferred_username | TEXT | From JWT claims |
| created_at / updated_at | TIMESTAMPTZ | |

#### `accounts`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| user_id | UUID FK → users | |
| name | TEXT | User-defined label |
| account_type | TEXT | `exchange` / `wallet` / `defi` |
| exchange_name | TEXT | `okx`, `binance`, … |
| api_key_encrypted | TEXT | AES-encrypted at rest |
| api_secret_encrypted | TEXT | AES-encrypted at rest |
| passphrase_encrypted | TEXT | OKX passphrase |
| wallet_address | TEXT | For EVM wallets |
| enabled_chains | JSONB | EVM chains enabled for this wallet |
| holdings | JSONB | Cached normalized holdings |
| last_synced_at | TIMESTAMPTZ | |
| is_active | BOOL | |

#### `portfolios`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| user_id | UUID FK → users | |
| name | TEXT | |
| description | TEXT | |
| target_allocation | JSONB | `[{symbol, target_pct}]` |
| guardrails | JSONB | Rebalancing guardrails |
| is_default | BOOL | One default per user |
| last_constructed_at | TIMESTAMPTZ | |

#### `portfolio_accounts` (M2M join)
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| portfolio_id | UUID FK → portfolios | |
| account_id | UUID FK → accounts | UNIQUE per portfolio |
| added_at | TIMESTAMPTZ | |

#### `snapshots`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| portfolio_id | UUID FK → portfolios | |
| snapshot_date | DATE | |
| snapshot_type | TEXT | `eod` / `manual` / `hourly` |
| total_value_usd | DECIMAL | |
| holdings | JSONB | Array of asset holdings at snapshot time |
| metadata | JSONB | Exchange rates, notes |
| created_at | TIMESTAMPTZ | |
> Unique constraint: `(portfolio_id, snapshot_date, snapshot_type)`

### Asset & Market Data Tables

#### `assets`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| symbol | TEXT | e.g., `BTC` |
| name | TEXT | e.g., `Bitcoin` |
| asset_type | TEXT | `cryptocurrency` / `token` / `stablecoin` |
| coingecko_id | TEXT | External ID (CoinGecko) |
| coinmarketcap_id | TEXT | External ID |
| is_active | BOOL | |
> Unique constraint: `(symbol, name)` — allows same symbol across different named assets

#### `asset_contracts`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| asset_id | UUID FK → assets | |
| chain | TEXT | `ethereum`, `bsc`, `polygon`, … |
| contract_address | TEXT | ERC-20 contract address |
| token_standard | TEXT | `ERC20`, `BEP20`, … |
| decimals | INT | |
| is_verified | BOOL | |
> Unique constraint: `(chain, contract_address)`

#### `asset_prices`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| asset_id | UUID FK → assets | |
| timestamp | TIMESTAMPTZ | |
| price_usd | DECIMAL | |
| volume_24h_usd | DECIMAL | |
| market_cap_usd | DECIMAL | |
| rank | INT | Market cap rank |
| source | TEXT | `coinpaprika`, `coingecko` |
> Unique constraint: `(asset_id, timestamp, source)`

### EVM Registry Tables

#### `evm_chains`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| chain_name | TEXT | e.g., `ethereum` |
| chain_id | INT | EVM chain ID |
| rpc_url | TEXT | Configurable RPC endpoint |
| is_active | BOOL | |

#### `evm_tokens`
| Column | Type | Notes |
|--------|------|-------|
| id | UUID PK | |
| chain_id | UUID FK → evm_chains | |
| symbol | TEXT | |
| contract_address | TEXT | |
| decimals | INT | |
| logo_url | TEXT | |
| is_active | BOOL | |
> Unique constraint: `(chain_id, contract_address)`

---

## 9. Background Job System

Jobs are registered in `api/src/jobs/runner.rs` using `tokio-cron-scheduler`. Each job runs in a background Tokio task without blocking the API.

| Job | File | Default Schedule | Purpose |
|-----|------|-----------------|---------|
| `fetch_all_coins` | `fetch_all_coins.rs` | `0 0 0 * * *` (daily midnight UTC) | Fetch all coins from CoinPaprika; upsert `assets` + `asset_contracts` |
| `price_collection` | `price_collection.rs` | `0 */15 * * * *` (every 15 min) | Collect spot prices for top-ranked assets; write `asset_prices` |
| `eod_snapshot` | `portfolio_snapshot.rs` | `0 0 23 * * *` (daily 11 PM UTC) | Create EOD snapshots for all active portfolios |

Jobs can also be **manually triggered** via HTTP:
- `POST /api/jobs/fetch-all-coins`
- `POST /api/migrations`

All jobs share the same SeaORM connection pool from `AppState`.

### Environment Variables (Jobs)

```bash
FETCH_ALL_COINS_ENABLED=true
FETCH_ALL_COINS_SCHEDULE="0 0 0 * * *"
PRICE_COLLECTION_ENABLED=true
PRICE_COLLECTION_SCHEDULE="0 */15 * * * *"
EOD_SNAPSHOT_ENABLED=true
EOD_SNAPSHOT_SCHEDULE="0 0 23 * * *"
```

---

## 10. External Connectors

### OKX Exchange (`connectors/okx.rs`)

- **Auth**: HMAC-SHA256 signature using `api_key`, `api_secret`, `passphrase`
- **Endpoint**: OKX REST API (read-only)
- **Data**: Spot account balances (`available` + `frozen` per asset)
- **Permission**: Read-only API keys only — withdrawals never enabled

### EVM Wallets (`connectors/evm.rs`)

- **Library**: `alloy` crate
- **Method**: JSON-RPC calls to configured chain RPC URLs
- **Tokens**: Scans `evm_tokens` registry for each enabled chain
- **Data**: Native coin balance + ERC-20 token balances
- **Chains**: Configured via `evm_chains` table (admin-configurable RPC URLs)

### CoinGecko (`connectors/coingecko.rs`)

- **Purpose**: Fetch top coins by market cap; collect price data
- **Auth**: API key (free or Pro tier)

### CoinPaprika (`connectors/coinpaprika.rs`)

- **Purpose**: Comprehensive coin metadata, contract addresses, historical data
- **Auth**: Public API (no key required for basic endpoints)
- **Rate limits**: Respected via configurable job batch sizes

### Asset Identity Resolution (`helpers/asset_identity.rs`)

Cross-exchange asset matching uses a priority-ordered strategy:
1. Contract address lookup (most precise)
2. Symbol + name exact match
3. Symbol-only match (with disambiguation for duplicates)

---

## 11. Infrastructure

### Docker Compose Services

```yaml
services:
  postgres:          # App DB — port 5432
  keycloak:          # OIDC — port 8080
  keycloak-init:     # One-shot realm setup
  api:               # Rust API — port 3000
  web:               # Next.js — port 3001
```

All services share `crypto-pocket-butler-network` (bridge).

### Environment Configuration

Root `.env` (shared across services):

```bash
KEYCLOAK_REALM=myrealm
KEYCLOAK_AUDIENCE=crypto-pocket-butler
KEYCLOAK_ISSUER=http://keycloak:8080/realms/myrealm
KEYCLOAK_CLIENT_ID=crypto-pocket-butler
KEYCLOAK_CLIENT_SECRET=<secret>
NEXTAUTH_SECRET=<secret>
NEXTAUTH_URL=http://localhost:3000
WEB_ROOT_URL=http://localhost:3001
NEXT_PUBLIC_BACKEND_URL=http://localhost:3001
RUST_LOG=crypto_pocket_butler_backend=info
```

API-specific `.env`:

```bash
DATABASE_URL=postgres://postgres:postgres@localhost/crypto_pocket_butler
DB_MAX_CONNECTIONS=100
KEYCLOAK_SERVER=http://keycloak:8080
KEYCLOAK_REALM=myrealm
KEYCLOAK_AUDIENCE=crypto-pocket-butler
```

### Database Migrations

Migrations live in `api/migration/src/` and use the SeaORM migration CLI:

```bash
cd api/migration
cargo run -- up      # Apply pending migrations
cargo run -- down    # Roll back last migration
cargo run -- status  # Show migration status
```

Migrations can also be triggered at runtime via `POST /api/migrations`.

---

*For detailed setup instructions see [../setup/DOCKER_SETUP.md](../setup/DOCKER_SETUP.md).*
*For coding conventions see [../coding-guidelines/CODING_GUIDELINES.md](../coding-guidelines/CODING_GUIDELINES.md).*
*For user workflows see [../use-cases/USE_CASES.md](../use-cases/USE_CASES.md).*
