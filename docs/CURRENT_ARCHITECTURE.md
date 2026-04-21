# Current Architecture

Visual diagram of the crypto-pocket-butler system architecture.

## System Overview

```mermaid
C4Context
    title Crypto Pocket Butler - System Context Diagram
    Person(user, "User", "Portfolio manager accessing crypto holdings")
    System_Boundary(crypto_app, "Crypto Pocket Butler") {
        Container(frontend, "Next.js Frontend", "Next.js 14 + Tailwind CSS 4", "Web UI for portfolio management")
        Container(api, "Rust API", "Axum 0.8 + SeaORM", "REST API for portfolio operations")
        ContainerDb(db, "PostgreSQL", "PostgreSQL", "Portfolio and user data storage")
    }
    System_Ext(keycloak, "Keycloak", "OIDC Identity Provider", "Authentication & authorization")
    System_Ext(okx, "OKX Exchange", "Crypto Exchange", "Read-only balance fetching")
    System_Ext(evm, "EVM Networks", "Ethereum/Polygon", "Wallet balance queries via RPC")
    System_Ext(solana, "Solana", "Solana Network", "Wallet balance queries via RPC")
    System_Ext(coinpaprika, "CoinPaprika", "Price API", "Asset pricing data")

    Rel(user, frontend, "Uses")
    Rel(frontend, api, "HTTP JSON API")
    Rel(api, db, "SQL queries")
    Rel(api, keycloak, "JWT validation")
    Rel(api, okx, "Read-only trading API")
    Rel(api, evm, "ETH RPC calls")
    Rel(api, solana, "Solana RPC calls")
    Rel(api, coinpaprika, "Price data fetching")
```

## Component Architecture

```mermaid
flowchart TB
    subgraph Frontend["Frontend Layer"]
        direction LR
        nextjs["Next.js 14<br/>App Router"]
        nextauth["NextAuth.js v5<br/>Keycloak OIDC"]
        components["React Components<br/>Tailwind CSS"]
        tanstack["TanStack Query<br/>Data Fetching"]
        
        nextjs --> nextauth
        nextjs --> components
        components --> tanstack
    end

    subgraph Backend["Backend Layer (Rust)"]
        direction LR
        axum["Axum 0.8<br/>HTTP Server"]
        handlers["Handlers<br/>Routes"]
        services["Application<br/>Services"]
        domain["Domain<br/>Entities"]
        infra["Infrastructure<br/>SeaORM"]
        auth["axum-keycloak-auth<br/>JWT Validation"]
        jobs["Apalis<br/>Background Jobs"]
        
        axum --> auth
        axum --> handlers
        handlers --> services
        services --> domain
        services --> infra
        infra --> jobs
    end

    subgraph Data["Database Layer"]
        direction LR
        pg["PostgreSQL"]
        seaorm["SeaORM<br/>Entities"]
        migrations["Migrations"]
    end

    subgraph External["External Integrations"]
        direction LR
        keycloak["Keycloak<br/>OIDC"]
        okx["OKX Exchange<br/>API"]
        evm["EVM RPC<br/>Providers"]
        sol["Solana RPC<br/>Providers"]
        cp["CoinPaprika<br/>Price API"]
    end

    Frontend -->|HTTP JSON| Backend
    Backend -->|SQL| Data
    Backend -->|JWT| External
```

## Data Flow Architecture

```mermaid
sequenceDiagram
    participant User
    participant Frontend
    participant API
    participant Keycloak
    participant DB
    participant External

    User->>Frontend: Access portfolio dashboard
    Frontend->>Keycloak: OIDC login (PKCE)
    Keycloak-->>Frontend: JWT session
    Frontend->>API: GET /api/portfolios (Bearer JWT)
    API->>Keycloak: Validate JWT token
    Keycloak-->>API: Token valid
    
    rect rgb(240, 248, 255)
        Note over API,DB: Authenticated request path
        API->>DB: Query portfolios
        DB-->>API: Portfolio data
    end
    
    rect rgb(255, 248, 240)
        Note over API,External: Data enrichment
        API->>External: Fetch prices/balances
        External-->>API: Price + balance data
    end
    
    API-->>Frontend: JSON response
    Frontend-->>User: Render dashboard
```

## Database Schema (Core Entities)

```mermaid
erDiagram
    USERS ||--o{ PORTFOLIOS : owns
    PORTFOLIOS ||--o{ PORTFOLIO_ACCOUNTS : contains
    PORTFOLIOS ||--o{ PORTFOLIO_SNAPSHOTS : tracks
    PORTFOLIOS ||--o{ ALLOCATIONS : manages
    PORTFOLIO_ACCOUNTS ||--o{ HOLDINGS : tracks
    HOLDINGS ||--o{ ASSET_PRICES : values

    USERS {
        uuid id PK
        string email
        string name
        string role
        timestamptz created_at
    }

    PORTFOLIOS {
        uuid id PK
        uuid user_id FK
        string name
        jsonb target_allocation
        timestamptz created_at
    }

    PORTFOLIO_ACCOUNTS {
        uuid id PK
        uuid portfolio_id FK
        string account_type
        string account_name
        text encrypted_credentials
        boolean is_active
    }

    HOLDINGS {
        uuid id PK
        uuid account_id FK
        string asset_symbol
        decimal balance
        decimal value_usd
    }

    ASSET_PRICES {
        string asset_id PK
        decimal price_usd
        timestamptz fetched_at
    }
```

## Technology Stack Summary

| Layer | Technology | Purpose |
|-------|------------|---------|
| Frontend Framework | Next.js 14 | App Router, SSR/RSC |
| Frontend Styling | Tailwind CSS 4 | Pure Tailwind (no UI libs) |
| Frontend Auth | NextAuth.js v5 | Keycloak OIDC + PKCE |
| Backend Framework | Axum 0.8 | HTTP server |
| ORM | SeaORM | Database models |
| Database | PostgreSQL | Persistent storage |
| Auth | Keycloak | JWT validation |
| API Docs | utoipa | OpenAPI/Swagger |
| Background Jobs | Apalis | Async task processing |
| External APIs | OKX, EVM RPC, Solana, CoinPaprika | Market data |