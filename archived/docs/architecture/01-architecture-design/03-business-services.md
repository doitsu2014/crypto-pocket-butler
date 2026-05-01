# Backend Architecture - Business Services & Logic

## Application Layer Overview

```mermaid
graph TD
    AL[Application Layer]
    SVC[services/]
    UC[usecases/]
    JOBS[jobs/]
    DTO[dto/]
    CONC[concurrency/]
    REPO[repositories/]

    AL --> SVC
    AL --> UC
    AL --> JOBS
    AL --> DTO
    AL --> CONC
    AL --> REPO

    SVC --> PS[PortfolioService]
    SVC --> AS[AccountService]

    UC --> AU[AccountUseCases]
    UC --> CU[ChainUseCases]
    UC --> PU[PortfolioUseCases]
    UC --> RU[RecommendationUseCases]
    UC --> SU[SnapshotUseCases]

    REPO --> AR[AccountRepository trait]
    REPO --> PR[PortfolioRepository trait]
    REPO --> ASR[AssetRepository trait]
```

---

## Business Logic Layer

```mermaid
graph TD
    PBL[Portfolio Business Logic]
    ABL[Asset Business Logic]
    HBL[Holding Business Logic]
    ALBL[Allocation Business Logic]
    SBL[Snapshot Business Logic]
    
    VN[ValidateName]
    CD[CheckDefault]
    VG[ValidateGuardrails]
    CO[CheckOwnership]
    
    LP[LookUpPrices]
    NS[NormalizeSymbols]
    CM[CheckChainMapping]
    
    NQ[NormalizeQuantities]
    CA[CalculateAvailable]
    MA[MergeAccounts]
    
    AH[AggregateHoldings]
    EP[EnrichWithPrices]
    CV[CalculateValues]
    CW[ComputeWeights]
    
    CC[CreateCopy]
    PS[PreserveState]
    GM[GenerateMetadata]
    
    PBL --> VN
    PBL --> CD
    PBL --> VG
    PBL --> CO
    
    ABL --> LP
    ABL --> NS
    ABL --> CM
    
    HBL --> NQ
    HBL --> CA
    HBL --> MA
    
    ALBL --> AH
    ALBL --> EP
    ALBL --> CV
    ALBL --> CW
    
    SBL --> CC
    SBL --> PS
    SBL --> GM
```

---

## High-Level Business Flow: Portfolio Construction

```mermaid
sequenceDiagram
    participant U as User
    participant A as API
    participant PS as PortfolioService
    participant HS as HoldingService
    participant ALS as AllocationService
    participant AS as AssetService
    participant PE as PortfolioEntity
    participant CP as CoinPaprika

    U->>A: POST /api/portfolios/id/construct
    A->>PS: validate_permission
    PS->>PS: check_portfolio_ownership
    PS->>HS: fetch_all_holdings
    HS-->>PS: Vec AccountHolding
    PS->>AS: get_prices
    AS->>CP: GET /coins
    CP-->>AS: Asset prices
    AS-->>PS: HashMap asset price
    PS->>ALS: build_allocation
    ALS->>ALS: aggregate_quantities
    ALS->>ALS: enrich_with_prices
    ALS->>ALS: calculate_values
    ALS->>ALS: compute_weights
    ALS-->>PS: AllocationData
    PS->>PE: save_allocation
    PE-->>PS: PortfolioAllocation
    PS-->>A: AllocationResponse
    A-->>U: 200 OK + JSON
```

---

## Clean Architecture Layer Map

| Layer | Components |
|-------|------------|
| **Domain Layer** | Account, Allocation, Asset, Chain, Portfolio domains |
| **Application Layer** | AccountUseCases, ChainUseCases, PortfolioUseCases, SnapshotUseCases, RecommendationUseCases |
| **Application Services** | PortfolioService, AccountService |
| **Repository Traits** | AccountRepository, PortfolioRepository, AssetRepository |
| **Infrastructure** | SeaORM Entities, PostgreSQL, Keycloak, EVM/OKX/Solana connectors |
| **Transport** | Axum HTTP handlers, routes, error mapping |