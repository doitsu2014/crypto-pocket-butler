# Backend Architecture - Domain-Driven Design

## Layered Architecture (Domain Model)

```mermaid
graph TB
    A[API Layer]
    D[Domain Layer]
    E[Entity Layer]
    X[External]
    A --> D
    D --> E
    E --> P[(PostgreSQL)]
    D --> C[Connectors]
    C --> X
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
    
    Portfolio "1" --> "0..*" PortfolioAccount
    Portfolio "1" --> "0..*" PortfolioAllocation
    Portfolio "1" --> "0..*" Snapshot
    PortfolioAccount --> Account
    Portfolio --> User
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
    
    Asset "1" --> "0..*" AssetPrice
    Asset "1" --> "0..*" AssetContract
    Asset "1" --> "0..*" EvmToken
    Asset "1" --> "0..*" SolanaToken
```

---

### Holding and Allocation Domain

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
    
    AccountHolding --> AllocationItem : enrich
    AllocationItem --> SnapshotHolding : persist
    AllocationData --> PortfolioAllocation : store
```

---

### Chain and Token Domain

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
    
    EvmChain "1" --> "0..*" ChainContract
```