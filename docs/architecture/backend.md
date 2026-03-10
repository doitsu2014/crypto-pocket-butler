# Crypto Pocket Butler Backend API Architecture

## Domain-Driven Architecture

### Layered Architecture (Domain Model)

```mermaid
graph TB
    subgraph "API Layer"
        H1[Handlers<br/>handlers/]
        H2[Routes<br/>routes]
    end
    
    subgraph "Domain Layer"
        D1[Domain Models<br/>domain/]
        D2[Business Logic<br/>handlers/]
    end
    
    subgraph "Entity Layer"
        E1[Entities<br/>entities/]
        E2[SeaORM DB Access]
    end
    
    subgraph "External"
        E3[Connectors<br/>connectors/]
        E4[External APIs]
    end
    
    H1 -->|Use| D1
    D1 -->|Use| E1
    E1 -->|Query| E2
    H1 -->|Call| D2
    H2 -->|Route| H1
    D2 -->|Call| D1
    D2 -->|Write| E2
    
    E2 -->|SQL| PG[(PostgreSQL)]
    
    D1 -->|Normalize| E3
    E3 -->|HTTP| E4
```

---

## Domain Model Structure

### Portfolio Domain

```mermaid
classDiagram
    class Portfolio {
        +Uuid id
        +Uuid user_id
        +String name
        +String description
        +bool is_default
        +Json target_allocation
        +Json guardrails
        +DateTime last_constructed_at
    }
    
    class PortfolioAccount {
        +Uuid portfolio_id
        +Uuid account_id
    }
    
    class PortfolioAllocation {
        +Uuid id
        +Uuid portfolio_id
        +AllocationData data
        +DateTime created_at
    }
    
    class Snapshot {
        +Uuid id
        +Uuid portfolio_id
        +SnapshotData data
        +DateTime created_at
    }
    
    class Account {
        +Uuid id
        +Uuid user_id
        +String name
        +String type
        +Json holdings
    }
    
    Portfolio "1" *-- "0..*" PortfolioAccount : has >
    Portfolio "1" *-- "0..*" PortfolioAllocation : has >
    Portfolio "1" *-- "0..*" Snapshot : has >
    PortfolioAccount *-- Account : links to >
    Portfolio *-- User : belongs to >
```

---

### Asset Domain

```mermaid
classDiagram
    class Asset {
        +String symbol
        +String name
        +String rank
        +Json market_data
    }
    
    class AssetPrice {
        +String asset
        +String chain
        +Decimal price_usd
        +DateTime timestamp
    }
    
    class AssetContract {
        +String asset
        +String chain
        +String contract_address
    }
    
    class EvmToken {
        +Uuid id
        +String symbol
        +String name
        +String contract_address
        +String chain
        +bool is_active
    }
    
    class SolanaToken {
        +Uuid id
        +String mint_address
        +String symbol
        +String name
        +bool is_active
    }
    
    Asset "1" *-- "0..*" AssetPrice : has price history >
    Asset "1" *-- "0..*" AssetContract : has contracts >
    Asset "1" *-- "0..*" EvmToken : evm versions >
    Asset "1" *-- "0..*" SolanaToken : solana versions >
```

---

### Holding & Allocation Domain

```mermaid
classDiagram
    class AccountHolding {
        +String asset
        +String quantity
        +String available
        +String frozen
        +u8 decimals
    }
    
    class AllocationItem {
        +String asset
        +String quantity
        +f64 price_usd
        +f64 value_usd
        +f64 weight
        +bool unpriced
    }
    
    class AllocationData {
        +Vec~AllocationItem~ items
        +Decimal total_value_usd
        +DateTime as_of
    }
    
    class SnapshotHolding {
        +String asset
        +String quantity
        +f64 price_usd
        +f64 value_usd
        +f64 weight
        +bool unpriced
    }
    
    class SnapshotData {
        +Vec~SnapshotHolding~ holdings
        +SnapshotMetadata metadata
    }
    
    AccountHolding --> AllocationItem : enrich with prices >
    AllocationItem --> SnapshotHolding : persist snapshot >
    AllocationData --> PortfolioAllocation : store >
    SnapshotData --> Snapshot : store >
```

---

### Chain & Token Domain

```mermaid
classDiagram
    class EvmChain {
        +Uuid id
        +String name
        +String chain_id
        +String rpc_url
        +String block_explorer
        +bool is_active
    }
    
    class ChainContract {
        +String asset
        +String chain
        +String contract_address
        +u8 decimals
    }
    
    EvmChain "1" *-- "0..*" ChainContract : supports >
    
    note right of EvmChain
        EVM chains:
        - Ethereum
        - BSC
        - Polygon
        - Arbitrum
        - Optimism
        - Base
    end note
```

---

## API Endpoint Architecture

```mermaid
graph TB
    subgraph "Public Routes"
        P1[GET /<br/>Root]
        P2[GET /health<br/>Health Check]
    end
    
    subgraph "Protected Routes (Any Auth)"
        A1[GET /api/me<br/>User Info]
        A2[GET /api/protected<br/>Protected]
    end
    
    subgraph "Portfolio Routes"
        PF1[GET /api/portfolios<br/>List]
        PF2[GET /api/portfolios/{id}<br/>Detail]
        PF3[POST /api/portfolios<br/>Create]
        PF4[PUT /api/portfolios/{id}<br/>Update]
        PF5[DELETE /api/portfolios/{id}<br/>Delete]
    end
    
    subgraph "Account Routes"
        ACC1[GET /api/accounts<br/>List]
        ACC2[GET /api/accounts/{id}<br/>Detail]
        ACC3[POST /api/accounts<br/>Create]
        ACC4[PUT /api/accounts/{id}<br/>Update]
        ACC5[POST /api/accounts/{id}/sync<br/>Sync]
    end
    
    subgraph "Admin Routes (Admin Role)"
        AD1[GET /admin/jobs<br/>apalis-board]
        AD2[POST /api/v1/jobs/fetch-all-coins<br/>Manual Job]
    end
    
    P1 --> H[Handler]
    P2 --> H
    A1 --> K[Keycloak Auth]
    A2 --> K
    PF1 --> K
    PF2 --> K
    PF3 --> K
    PF4 --> K
    PF5 --> K
    ACC1 --> K
    ACC2 --> K
    ACC3 --> K
    ACC4 --> K
    ACC5 --> K
    AD1 --> A[Admin Auth]
    AD2 --> A
    
    K --> RBAC[Role Check]
    A --> RBAC
    RBAC -->|User Role| PF1
    RBAC -->|User Role| ACC1
    RBAC -->|Admin Role| AD1
```

---

## Data Flow: Create Portfolio

```mermaid
sequenceDiagram
    participant Client
    participant API
    participant Auth
    participant PortfolioHandler
    participant PortfolioDomain
    participant PortfolioEntity
    participant DB

    Client->>API: POST /api/portfolios
    API->>Auth: Validate JWT Token
    Auth-->>API: Decoded Claims
    
    API->>PortfolioHandler: CreatePortfolioRequest
    
    PortfolioHandler->>PortfolioDomain: portfolio_to_domain(request)
    PortfolioDomain->>PortfolioDomain: validate_name(name)
    PortfolioDomain->>PortfolioDomain: validate_guardrails(guardrails)
    
    PortfolioDomain->>PortfolioEntity: new(portfolio)
    PortfolioEntity->>DB: INSERT INTO portfolios
    DB-->>PortfolioEntity: Return new UUID
    
    PortfolioEntity-->>PortfolioDomain: Portfolio { id, ... }
    PortfolioDomain-->>PortfolioHandler: Domain Portfolio
    
    PortfolioHandler-->>API: PortfolioResponse
    API-->>Client: 201 Created + JSON
```

---

## Domain Model Validation Flow

```mermaid
graph TD
    Request[API Request] --> Parse[Parse JSON]
    Parse --> Validate[Validate Fields]
    Validate -->|Valid| Domain[Create Domain Model]
    Validate -->|Invalid| Error[Validation Error]
    Error --> 400[400 Bad Request]
    
    Domain --> Business[Business Rules Check]
    Business -->|Pass| Transform[Transform to Entity]
    Business -->|Fail| Error2[Business Error]
    Error2 --> 422[422 Unprocessable Entity]
    
    Transform --> DB[Database Insert]
    DB -->|Success| Response[Return Response]
    DB -->|Conflict| 409[409 Conflict]
```

---

## Domain-Driven Design Principles Applied

```mermaid
mindmap
  root((Crypto Pocket Butler DDD))
    Domain Layer
      Boundaries
        Portfolio Domain
        Asset Domain
        Account Domain
        Snapshot Domain
      Entities
        Portfolio, Account, Asset
      Value Objects
        AllocationItem, SnapshotHolding
        AccountHolding, AllocationData
    Infrastructure Layer
      Persistence
        SeaORM Entities
        PostgreSQL Schema
      External Services
        Keycloak Auth
        CoinPaprika API
        OKX API
    Anticorruption Layer
      Connectors
        EVM Connector
        OKX Connector
      normalize_token_balance
      normalize_holdings
```