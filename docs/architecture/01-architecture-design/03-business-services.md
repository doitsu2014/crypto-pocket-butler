# Backend Architecture - Business Services & Logic

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
