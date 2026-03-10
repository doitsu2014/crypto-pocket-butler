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

## Business Services Layer

```mermaid
graph TD
    subgraph "Business Services"
        PS[PortfolioService]
        AS[AssetService]
        HS[HoldingService]
        SN[SnapshotService]
        AL[AllocationService]
    end
    
    subgraph "Domain Models"
        PM[Portfolio]
        AM[Asset]
        HM[AccountHolding]
        SM[SnapshotHolding]
        AIM[AllocationItem]
    end
    
    PS -->|manages| PM
    AS -->|manages| AM
    HS -->|manages| HM
    SN -->|manages| SM
    AL -->|manages| AIM
    
    PS -->|creates| AL
    HS -->|creates| HM
    AL -->|uses| AIM
```

---

## Business Logic Layer

```mermaid
graph TD
    subgraph "Portfolio Business Logic"
        PL1[Validate Portfolio Name]
        PL2[Check Default Portfolio Only One]
        PL3[Validate Guardrails Structure]
        PL4[Check Portfolio Ownership]
    end
    
    subgraph "Asset Business Logic"
        AL1[Look Up Asset Prices]
        AL2[Normalize Asset Symbols]
        AL3[Check Asset Chain Mapping]
    end
    
    subgraph "Holding Business Logic"
        HL1[Normalize Quantities]
        HL2[Calculate Available/Frozen]
        HL3[Merge Holdings Across Accounts]
    end
    
    subgraph "Allocation Business Logic"
        ALG1[Aggregate Holdings]
        ALG2[Enrich with Prices]
        ALG3[Calculate USD Values]
        ALG4[Compute Portfolio Weights]
    end
    
    subgraph "Snapshot Business Logic"
        SL1[Create Point-in-Time Copy]
        SL2[Preserve Allocation State]
        SL3[Generate Snapshot Metadata]
    end
    
    PL1 --> Portfolio[Portfolio Operations]
    PL2 --> Portfolio
    PL3 --> Portfolio
    PL4 --> Portfolio
    
    AL1 --> Asset[Asset Operations]
    AL2 --> Asset
    AL3 --> Asset
    
    HL1 --> Holding[Holding Operations]
    HL2 --> Holding
    HL3 --> Holding
    
    ALG1 --> Allocation[Allocation Operations]
    ALG2 --> Allocation
    ALG3 --> Allocation
    ALG4 --> Allocation
    
    SL1 --> Snapshot[Snapshot Operations]
    SL2 --> Snapshot
    SL3 --> Snapshot
```

---

## High-Level Business Flow: Portfolio Construction

```mermaid
sequenceDiagram
    participant User
    participant API
    participant PortfolioService
    participant HoldingService
    participant AllocationService
    participant AssetService
    participant PortfolioEntity
    participant SnapshotService
    participant SnapshotEntity

    User->>API: POST /api/portfolios/{id}/construct
    API->>PortfolioService: validate_permission(user, portfolio_id)
    
    PortfolioService->>PortfolioService: check_portfolio_ownership()
    PortfolioService->>HoldingService: fetch_all_holdings(portfolio_id)
    
    HoldingService->>API: User accounts from portfolio
    API->>API: For each account, fetch holdings
    
    HoldingService-->>PortfolioService: Vec~AccountHolding~
    
    PortfolioService->>AssetService: get_prices(holdings.assets)
    AssetService->>CoinPaprika: GET /coins
    CoinPaprika-->>AssetService: Asset prices
    
    AssetService-->>PortfolioService: HashMap~asset, price~
    
    PortfolioService->>AllocationService: build_allocation(holdings, prices)
    
    AllocationService->>AllocationService: aggregate_quantities();
    AllocationService->>AllocationService: enrich_with_prices(prices);
    AllocationService->>AllocationService: calculate_values();
    AllocationService->>AllocationService: compute_weights();
    
    AllocationService-->>PortfolioService: AllocationData
    
    PortfolioService->>PortfolioEntity: save_allocation(allocation)
    PortfolioEntity-->>PortfolioService: PortfolioAllocation
    
    Optional: Create snapshot if EOD
    PortfolioService->>SnapshotService: create_snapshot(allocation)
    SnapshotService->>SnapshotEntity: persist_snapshot()
    
    PortfolioService-->>API: AllocationResponse
    API-->>User: 200 OK + JSON
```

---

## Business Rules Validation

```mermaid
flowchart TD
    Start[New Portfolio Request] --> CheckName[Validate Name]
    CheckName -->|Invalid| Error1[400 Bad Request]
    CheckName -->|Valid| CheckDefault[Default Check]
    
    CheckDefault -->|Set Default| UnsetOther[Unset Other Defaults]
    CheckDefault -->|Not Default| CheckGuardrails
    
    UnsetOther --> CheckGuardrails
    CheckGuardrails -->|Invalid Format| Error2[400 Bad Request]
    CheckGuardrails -->|Valid| Save[Save to DB]
    
    Save -->|Success| Success[201 Created]
    Save -->|Conflict| Error3[409 Conflict]
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
    Business Services Layer
      PortfolioService
      AssetService
      HoldingService
      SnapshotService
      AllocationService
    Business Logic Layer
      Portfolio Validation
      Asset Pricing
      Holding Normalization
      Allocation Calculation
      Snapshot Persistence
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

---

## Code Structure Design

```mermaid
graph TB
    subgraph "api/src/"
        main[main.rs<br/>Server init, router, jobs]
        lib[lib.rs<br/>Library exports]
        db[db.rs<br/>Database pool]
        cache[cache.rs<br/>Moka cache]
        
        handlers[handlers/<br/>HTTP handlers]
        domain[domain/<br/>Business models]
        entities[entities/<br/>SeaORM models]
        connectors[connectors/<br/>External services]
        helpers[helpers/<br/>Utilities]
        jobs[jobs/<br/>Background tasks]
        concurrency[concurrency/<br/>Async helpers]
    end
    
    subgraph "handlers/"
        h_portfolios[portfolios.rs]
        h_accounts[accounts.rs]
        h_snapshots[snapshots.rs]
        h_recs[recommendations.rs]
        h_evm_chains[evm_chains.rs]
        h_evm_tokens[evm_tokens.rs]
        h_jobs[jobs.rs]
        h_error[error.rs]
    end
    
    subgraph "domain/"
        d_allocation[allocation.rs<br/>AllocationData]
        d_holdings[holdings.rs<br/>AccountHolding]
        d_snapshot[snapshot.rs<br/>SnapshotHolding]
        d_mod[mod.rs]
    end
    
    subgraph "entities/"
        e_users[users.rs]
        e_accounts[accounts.rs]
        e_portfolios[portfolios.rs]
        e_portfolio_accs[portfolio_accounts.rs]
        e_snapshots[snapshots.rs]
        e_assets[assets.rs]
        e_contracts[asset_contracts.rs]
        e_prices[asset_prices.rs]
        e_chains[evm_chains.rs]
        e_tokens[evm_tokens.rs]
        e_sol_tokens[solana_tokens.rs]
    end
    
    subgraph "connectors/"
        c_okx[okx.rs<br/>OKX exchange]
        c_evm[evm.rs<br/>EVM wallets]
        c_coingecko[coingecko.rs]
        c_paprika[coinpaprika.rs]
    end
    
    subgraph "helpers/"
        h_asset_id[asset_identity.rs]
        h_balance[balance_normalization.rs]
        h_auth[auth.rs]
    end
    
    subgraph "jobs/"
        j_runner[runner.rs<br/>Scheduling]
        j_fetch[fetch_all_coins.rs]
        j_price[price_collection.rs]
        j_sync[account_sync.rs]
        j_snapshot[portfolio_snapshot.rs]
    end
    
    main --> lib
    main --> db
    main --> cache
    main --> handlers
    main --> domain
    main --> entities
    main --> jobs
    
    handlers --> domain
    handlers --> entities
    handlers --> connectors
    handlers --> helpers
    
    domain --> entities
    domain --> connectors
    
    entities --> db
    connectors --> helpers
    handlers --> h_error
    
    jobs --> db
    jobs --> entities
    j_runner --> j_fetch
    j_runner --> j_price
    j_runner --> j_sync
    j_runner --> j_snapshot
    
    style main fill:#e1f5fe,stroke:#2196f3
    style handlers fill:#e8f5e9,stroke:#4caf50
    style domain fill:#fff3e0,stroke:#ff9800
    style entities fill:#f3e5f5,stroke:#9c27b0
    style connectors fill:#fce4ec,stroke:#e91e63
    style helpers fill:#e0f2f1,stroke:#009688
    style jobs fill:#fffde7,stroke:#ffc107
```

---

## Module Dependency Flow

```mermaid
graph LR
    lib[lib.rs] --> main[main.rs]
    
    main --> handlers
    main --> db
    main --> jobs
    main --> cache
    
    handlers --> domain
    handlers --> entities
    handlers --> connectors
    handlers --> helpers
    
    domain --> entities
    domain --> connectors
    
    entities --> db
    
    connectors --> helpers
    connectors --> db
    
    helpers --> db
    helpers --> cache
    
    jobs --> db
    jobs --> entities
    jobs --> domain
    
    style lib fill:#bbdefb,stroke:#1565c0
    style main fill:#a5d6a7,stroke:#2e7d32
```

---

## API Handler Structure

```mermaid
graph TD
    handlers[handlers/]
    
    handlers --> portfolios
    handlers --> accounts
    handlers --> snapshots
    handlers --> recommendations
    handlers --> evm_chains
    handlers --> evm_tokens
    handlers --> solana_tokens
    handlers --> chains
    handlers --> jobs
    handlers --> migrations
    handlers --> error
    
    portfolios --> portfolios_routes
    portfolios --> portfolio_handlers
    portfolios --> portfolio_dto
    
    accounts --> accounts_routes
    accounts --> account_handlers
    accounts --> account_dto
    
    snapshots --> snapshots_routes
    snapshots --> snapshot_handlers
    snapshots --> snapshot_dto
    
    portfolios_routes --> portfolios_handlers
    portfolios_handlers --> portfolio_service
    portfolios_service --> portfolio_domain
    portfolio_domain --> portfolio_entities
    
    style handlers fill:#e1f5fe,stroke:#2196f3
    style portfolios_handlers fill:#e8f5e9,stroke:#4caf50
    style portfolio_domain fill:#fff3e0,stroke:#ff9800
    style portfolio_entities fill:#f3e5f5,stroke:#9c27b0
```

---

## Database Entity Relationships

```mermaid
erDiagram
    users ||--o{ accounts : "has"
    users ||--o{ portfolios : "has"
    accounts ||--o{ portfolio_accounts : "joined through"
    portfolios ||--o{ portfolio_accounts : "has"
    
    portfolios ||--o{ portfolio_allocations : "has"
    portfolios ||--o{ snapshots : "has"
    
    accounts ||--|| holdings : "contains JSONB"
    
    assets ||--o{ asset_contracts : "has"
    assets ||--o{ asset_prices : "has"
    
    evm_chains ||--o{ evm_tokens : "supports"
    evm_chains ||--o{ asset_contracts : "has"
    
    portfolios ||--|| target_allocation : "JSONB"
    portfolios ||--|| guardrails : "JSONB"
```
