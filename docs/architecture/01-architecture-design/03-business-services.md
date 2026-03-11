# Backend Architecture - Business Services & Logic

## Business Services Layer

```mermaid
graph TD
    BS[Business Services Layer]
    PS[PortfolioService]
    AS[AssetService]
    HS[HoldingService]
    SS[SnapshotService]
    ALS[AllocationService]
    BS --> PS
    BS --> AS
    BS --> HS
    BS --> SS
    BS --> ALS
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

## Domain-Driven Design Principles

| Layer | Components |
|-------|------------|
| **Domain Layer** | Portfolio, Asset, Account, Snapshot domains |
| **Value Objects** | AllocationItem, SnapshotHolding, AccountHolding |
| **Business Services** | PortfolioService, AssetService, HoldingService |
| **Business Logic** | Validation, Pricing, Normalization, Calculation |
| **Infrastructure** | SeaORM Entities, PostgreSQL, Keycloak |
| **Anticorruption** | EVM Connector, OKX Connector |