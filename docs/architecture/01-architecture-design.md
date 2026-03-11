# Crypto Pocket Butler Backend API Architecture

## Domain-Driven Architecture

### Layered Architecture (Domain Model)

```mermaid
graph TB
    API Layer
    Domain Layer
    Entity Layer
    External
    API --> Domain
    Domain --> Entity
    Entity --> PostgreSQL
    Domain --> Connectors
    Connectors --> APIs
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
    }
    
    class PortfolioAccount {
        +Uuid portfolio_id
        +Uuid account_id
    }
    
    class PortfolioAllocation {
        +Uuid id
        +Uuid portfolio_id
        +AllocationData data
    }
    
    class Snapshot {
        +Uuid id
        +Uuid portfolio_id
        +SnapshotData data
    }
    
    class Account {
        +Uuid id
        +Uuid user_id
        +String name
        +String type
        +Json holdings
    }
    
    Portfolio "1" *-- "0..*" PortfolioAccount
    Portfolio "1" *-- "0..*" PortfolioAllocation
    Portfolio "1" *-- "0..*" Snapshot
    PortfolioAccount *-- Account
    Portfolio *-- User
```

---

### Asset Domain

```mermaid
classDiagram
    class Asset {
        +String symbol
        +String name
        +String rank
    }
    
    class AssetPrice {
        +String asset
        +String chain
        +Decimal price_usd
    }
    
    class AssetContract {
        +String asset
        +String chain
        +String contract_address
    }
    
    class EvmToken {
        +Uuid id
        +String symbol
        +String chain
    }
    
    class SolanaToken {
        +Uuid id
        +String mint_address
        +String symbol
    }
    
    Asset "1" *-- "0..*" AssetPrice
    Asset "1" *-- "0..*" AssetContract
    Asset "1" *-- "0..*" EvmToken
    Asset "1" *-- "0..*" SolanaToken
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
    }
    
    class AllocationData {
        +Vec items
        +Decimal total_value_usd
    }
    
    class SnapshotHolding {
        +String asset
        +String quantity
        +f64 price_usd
        +f64 value_usd
        +f64 weight
    }
    
    AccountHolding --> AllocationItem : enrich with prices
    AllocationItem --> SnapshotHolding : persist snapshot
    AllocationData --> PortfolioAllocation : store
    SnapshotData --> Snapshot : store
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
    }
    
    class ChainContract {
        +String asset
        +String chain
        +String contract_address
        +u8 decimals
    }
    
    EvmChain "1" *-- "0..*" ChainContract
```

---

## API Endpoint Architecture

```mermaid
graph TD
    PublicRoutes[Public Routes]
    ProtectedRoutes[Protected Routes]
    PortfolioRoutes[Portfolio Routes]
    AccountRoutes[Account Routes]
    AdminRoutes[Admin Routes]
    
    PublicRoutes --> Health
    ProtectedRoutes --> Auth
    PortfolioRoutes --> PortfolioHandlers
    AccountRoutes --> AccountHandlers
    AdminRoutes --> AdminAuth
    Auth --> PortfolioRoutes
    Auth --> AccountRoutes
    AdminAuth --> AdminRoutes
```

---

## Data Flow: Create Portfolio

```mermaid
sequenceDiagram
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
    Request --> Parse
    Parse --> Validate
    Validate --> Valid
    Valid --> Domain
    Domain --> Business
    Business --> Pass
    Pass --> Transform
    Transform --> DB
    DB --> Success
    Validate --> Invalid
    Invalid --> Error1
    Business --> Fail
    Fail --> Error2
    DB --> Conflict
    Error1 --> 400
    Error2 --> 422
    Conflict --> 409
    Success --> Response
```

---

## Business Services Layer

```mermaid
graph TD
    BusinessServices[Business Services Layer]
    PortfolioService[PortfolioService]
    AssetService[AssetService]
    HoldingService[HoldingService]
    SnapshotService[SnapshotService]
    AllocationService[AllocationService]
    BusinessServices --> PortfolioService
    BusinessServices --> AssetService
    BusinessServices --> HoldingService
    BusinessServices --> SnapshotService
    BusinessServices --> AllocationService
```

---

## Business Logic Layer

```mermaid
graph TD
    PortfolioBusinessLogic[Portfolio Business Logic]
    AssetBusinessLogic[Asset Business Logic]
    HoldingBusinessLogic[Holding Business Logic]
    AllocationBusinessLogic[Allocation Business Logic]
    SnapshotBusinessLogic[Snapshot Business Logic]
    
    PortfolioBusinessLogic --> ValidateName
    PortfolioBusinessLogic --> CheckDefault
    PortfolioBusinessLogic --> ValidateGuardrails
    PortfolioBusinessLogic --> CheckOwnership
    
    AssetBusinessLogic --> LookUpPrices
    AssetBusinessLogic --> NormalizeSymbols
    AssetBusinessLogic --> CheckChainMapping
    
    HoldingBusinessLogic --> NormalizeQuantities
    HoldingBusinessLogic --> CalculateAvailable
    HoldingBusinessLogic --> MergeAccounts
    
    AllocationBusinessLogic --> AggregateHoldings
    AllocationBusinessLogic --> EnrichWithPrices
    AllocationBusinessLogic --> CalculateValues
    AllocationBusinessLogic --> ComputeWeights
    
    SnapshotBusinessLogic --> CreateCopy
    SnapshotBusinessLogic --> PreserveState
    SnapshotBusinessLogic --> GenerateMetadata
```

---

## High-Level Business Flow: Portfolio Construction

```mermaid
sequenceDiagram
    User->>API: POST /api/portfolios/{id}/construct
    API->>PortfolioService: validate_permission
    PortfolioService->>PortfolioService: check_portfolio_ownership
    PortfolioService->>HoldingService: fetch_all_holdings
    HoldingService-->>PortfolioService: Vec AccountHolding
    PortfolioService->>AssetService: get_prices
    AssetService->>CoinPaprika: GET /coins
    CoinPaprika-->>AssetService: Asset prices
    AssetService-->>PortfolioService: HashMap asset, price
    PortfolioService->>AllocationService: build_allocation
    AllocationService->>AllocationService: aggregate_quantities
    AllocationService->>AllocationService: enrich_with_prices
    AllocationService->>AllocationService: calculate_values
    AllocationService->>AllocationService: compute_weights
    AllocationService-->>PortfolioService: AllocationData
    PortfolioService->>PortfolioEntity: save_allocation
    PortfolioEntity-->>PortfolioService: PortfolioAllocation
    PortfolioService-->>API: AllocationResponse
    API-->>User: 200 OK + JSON
```

---

## Business Rules Validation

```mermaid
flowchart TD
    Start --> CheckName
    CheckName --> Invalid --> Error1
    CheckName --> Valid --> CheckDefault
    CheckDefault --> SetDefault --> UnsetOther
    CheckDefault --> NotDefault --> CheckGuardrails
    UnsetOther --> CheckGuardrails
    CheckGuardrails --> Invalid --> Error2
    CheckGuardrails --> Valid --> Save
    Save --> Success
    Save --> Conflict
    Error1 --> 400
    Error2 --> 400
    Success --> 201
    Conflict --> 409
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
                Portfolio
                Account
                Asset
            Value Objects
                AllocationItem
                SnapshotHolding
                AccountHolding
                AllocationData
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
    api_src[api/src/]
    main[main.rs]
    lib[lib.rs]
    db[db.rs]
    cache[cache.rs]
    handlers[handlers/]
    domain[domain/]
    entities[entities/]
    connectors[connectors/]
    helpers[helpers/]
    jobs[jobs/]
    concurrency[concurrency/]
    
    handlers --> handlers_files
    handlers_files --> portfolios
    handlers_files --> accounts
    handlers_files --> snapshots
    handlers_files --> recommendations
    handlers_files --> evm_chains
    handlers_files --> evm_tokens
    
    domain --> domain_files
    domain_files --> allocation
    domain_files --> holdings
    domain_files --> snapshot
    
    entities --> entities_files
    entities_files --> users
    entities_files --> accounts_entities
    entities_files --> portfolios_entities
    entities_files --> portfolio_accounts
    entities_files --> snapshots_entities
    
    handlers --> domain
    handlers --> entities
    handlers --> connectors
    handlers --> helpers
    
    domain --> entities
    domain --> connectors
    
    entities --> db
    connectors --> helpers
    helpers --> db
    jobs --> db
    jobs --> entities
    jobs --> domain
```

---

## Module Dependency Flow

```mermaid
graph LR
    lib --> main
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
    jobs --> db
    jobs --> entities
    jobs --> domain
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
    
    portfolios --> portfolios_routes
    portfolios --> portfolio_handlers
    portfolios --> portfolio_dto
    
    accounts --> accounts_routes
    accounts --> account_handlers
    accounts --> account_dto
    
    portfolios_routes --> portfolio_handlers
    portfolio_handlers --> portfolio_service
    portfolio_service --> portfolio_domain
    portfolio_domain --> portfolio_entities
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
```
