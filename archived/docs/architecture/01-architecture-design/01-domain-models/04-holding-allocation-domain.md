# Holding & Allocation Domain

> **Bounded Context:** Holdings enrichment, allocation calculation, and snapshot storage.
> **This domain bridges Account and Portfolio aggregates.**

---

## Domain Model

```mermaid
classDiagram
    class AccountHolding {
        <<Entity>>
        +String asset
        +String quantity
        +String available
        +String frozen
        +u8 decimals
        +enrich_with_price(price) AllocationItem
    }
    
    class AllocationItem {
        <<Value Object>>
        +String asset
        +String quantity
        +f64 price_usd
        +f64 value_usd
        +f64 weight
    }
    
    class AllocationData {
        <<Value Object>>
        +Vec~AllocationItem~ items
        +Decimal total_value_usd
        +calculate_weights()
        +find_by_asset(asset)
    }
    
    class UnpricedAsset {
        <<Value Object>>
        +String asset
        +String quantity
        +String reason
    }
    
    class SnapshotHolding {
        <<Entity>>
        +String asset
        +String quantity
        +f64 price_usd
        +f64 value_usd
        +f64 weight
    }
    
    class SnapshotData {
        <<Value Object>>
        +Vec~SnapshotHolding~ items
        +Decimal total_value_usd
        +SnapshotMetadata metadata
    }
    
    class SnapshotMetadata {
        <<Value Object>>
        +DateTime created_at
        +int account_count
        +int asset_count
    }
    
    AccountHolding --> AllocationItem : enrich
    AllocationItem --> SnapshotHolding : persist
    AllocationData --> PortfolioAllocation : store
```

---

## Data Flow

```mermaid
flowchart LR
    subgraph "Input"
        AH[AccountHolding<br/>quantity only]
        P[AssetPrice]
    end
    
    subgraph "Enrichment"
        AI[AllocationItem<br/>quantity + price + value + weight]
        AD[AllocationData<br/>aggregated]
    end
    
    subgraph "Output"
        SH[SnapshotHolding<br/>persisted]
        SD[SnapshotData<br/>with metadata]
    end
    
    AH --> AI
    P --> AI
    AI --> AD
    AD --> SH
    SH --> SD
```

---

## Allocation Calculation Flow

```mermaid
sequenceDiagram
    participant PortfolioService
    participant AccountService
    participant PriceService
    participant AllocationBuilder
    participant SnapshotRepository
    
    PortfolioService->>AccountService: Get holdings for accounts
    AccountService-->>PortfolioService: List of AccountHolding
    
    loop For each holding
        PortfolioService->>PriceService: Get price for asset
        PriceService-->>PortfolioService: Price in USD
    end
    
    PortfolioService->>AllocationBuilder: Build allocation(holdings, prices)
    AllocationBuilder->>AllocationBuilder: Calculate value_usd = quantity * price
    AllocationBuilder->>AllocationBuilder: Calculate weight = value / total
    AllocationBuilder-->>PortfolioService: AllocationData
    
    PortfolioService->>SnapshotRepository: Save snapshot
    Note over PortfolioService,SnapshotRepository: SnapshotData persisted
```

---

## Weight Calculation

```mermaid
flowchart TD
    subgraph "Allocation Weight Calculation"
        WC1[Start with AllocationData]
        WC2[Calculate total_value_usd<br/>sum of all value_usd]
        WC3[For each item:<br/>weight = value_usd / total_value_usd * 100]
        WC4[Validate weights sum to 100%]
        WC5[AllocationData ready]
        
        WC1 --> WC2 --> WC3 --> WC4 --> WC5
    end
```

### Example Calculation

| Asset | Quantity | Price USD | Value USD | Weight |
|-------|----------|-----------|-----------|--------|
| BTC | 0.5 | $50,000 | $25,000 | 50% |
| ETH | 5.0 | $3,000 | $15,000 | 30% |
| SOL | 50 | $200 | $10,000 | 20% |
| **Total** | - | - | **$50,000** | **100%** |

---

## Business Rules

| Rule | Description |
|------|-------------|
| Quantity normalization | All quantities are normalized (human-readable) decimal strings |
| Price lookup | If price not found, holding goes to UnpricedAsset list |
| Weight precision | Weights calculated to 4 decimal places |
| Snapshot immutability | Snapshots are never modified after creation |
| Metadata tracking | Account count and asset count stored with snapshot |

---

## Value Object Specifications

### AccountHolding (Input)

```json
{
  "asset": "BTC",
  "quantity": "1.5",
  "available": "1.2",
  "frozen": "0.3",
  "decimals": 8
}
```

### AllocationItem (Enriched)

```json
{
  "asset": "BTC",
  "quantity": "1.5",
  "price_usd": 50000.00,
  "value_usd": 75000.00,
  "weight": 0.45
}
```

### SnapshotHolding (Persisted)

```json
{
  "asset": "BTC",
  "quantity": "1.5",
  "price_usd": 50000.00,
  "value_usd": 75000.00,
  "weight": 0.45
}
```

---

## Invariants

| Invariant | Description |
|-----------|-------------|
| Weight sum | All weights in AllocationData must sum to 100% (or 1.0) |
| Positive values | quantity, price_usd, value_usd must be non-negative |
| Frozen ≤ Total | frozen quantity cannot exceed total quantity |
| Available + Frozen = Total | Available plus frozen should equal total quantity |

---

## Repository Interface

```mermaid
classDiagram
    class AllocationRepository {
        <<Interface>>
        +find_latest(portfolio_id) Option~AllocationData~
        +save(portfolio_id, allocation) AllocationData
    }
    
    class SnapshotRepository {
        <<Interface>>
        +find_by_portfolio(portfolio_id) List~Snapshot~
        +find_latest(portfolio_id) Option~Snapshot~
        +save(snapshot: Snapshot) Snapshot
    }
```