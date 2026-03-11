# Portfolio Domain

> **Bounded Context:** Portfolio management, target allocations, and snapshots.
> **Aggregate Root:** Portfolio

---

## Domain Model

```mermaid
classDiagram
    class Portfolio {
        <<Aggregate Root>>
        +Uuid id
        +Uuid user_id
        +String name
        +String description
        +bool is_default
        +Json target_allocation
        +Json guardrails
        +add_account(account_id)
        +remove_account(account_id)
        +create_snapshot()
        +get_current_allocation()
    }
    
    class PortfolioAccount {
        <<Entity>>
        +Uuid portfolio_id
        +Uuid account_id
    }
    
    class PortfolioAllocation {
        <<Entity>>
        +Uuid id
        +Uuid portfolio_id
        +AllocationData data
        +DateTime calculated_at
    }
    
    class Snapshot {
        <<Entity>>
        +Uuid id
        +Uuid portfolio_id
        +SnapshotData data
        +DateTime created_at
    }
    
    class TargetAllocation {
        <<Value Object>>
        +Map~String,Decimal~ weights
        +validate()
        +get_weight(asset)
    }
    
    class Guardrails {
        <<Value Object>>
        +Decimal max_deviation
        +List~String~ allowed_assets
        +bool is_violated(allocation)
    }
    
    Portfolio "1" --> "0..*" PortfolioAccount
    Portfolio "1" --> "0..*" PortfolioAllocation
    Portfolio "1" --> "0..*" Snapshot
    Portfolio "1" --> "1" TargetAllocation
    Portfolio "1" --> "0..1" Guardrails
    PortfolioAccount --> Account
    Portfolio --> User
```

---

## Aggregate Boundaries

```mermaid
graph TB
    subgraph "Portfolio Aggregate"
        P[Portfolio<br/>Aggregate Root]
        PA[PortfolioAccount]
        PAL[PortfolioAllocation]
        S[Snapshot]
        TA[TargetAllocation]
        G[Guardrails]
        
        P --> PA
        P --> PAL
        P --> S
        P --> TA
        P --> G
    end
    
    subgraph "External References"
        A[Account]
        U[User]
    end
    
    PA -.->|references| A
    P -.->|belongs to| U
```

---

## Business Rules

```mermaid
flowchart TD
    subgraph "Portfolio Creation"
        PC1[User creates portfolio]
        PC2[Name must be unique per user]
        PC3[First portfolio is default]
        PC4[Portfolio created]
        
        PC1 --> PC2 --> PC3 --> PC4
    end
    
    subgraph "Account Assignment"
        AA1[Add account to portfolio]
        AA2{Account belongs to user?}
        AA3{Already in portfolio?}
        AA4[Add to portfolio]
        AA5[Error: not your account]
        AA6[Error: already added]
        
        AA1 --> AA2
        AA2 -->|No| AA5
        AA2 -->|Yes| AA3
        AA3 -->|Yes| AA6
        AA3 -->|No| AA4
    end
    
    subgraph "Snapshot Creation"
        SC1[Trigger snapshot]
        SC2[Aggregate all account holdings]
        SC3[Apply current prices]
        SC4[Calculate weights]
        SC5[Store snapshot]
        
        SC1 --> SC2 --> SC3 --> SC4 --> SC5
    end
```

---

## Invariants

| Invariant | Description |
|-----------|-------------|
| Unique name | Portfolio name must be unique per user |
| Default portfolio | Each user has exactly one default portfolio |
| Account ownership | Only accounts owned by user can be added |
| Target allocation weights | Must sum to 100% (if defined) |
| Guardrail deviation | Alert if allocation deviates beyond threshold |

---

## Repository Interface

```mermaid
classDiagram
    class PortfolioRepository {
        <<Interface>>
        +find_by_id(id: Uuid) Option~Portfolio~
        +find_by_user_id(user_id: Uuid) List~Portfolio~
        +find_default(user_id: Uuid) Option~Portfolio~
        +save(portfolio: Portfolio) Portfolio
        +delete(id: Uuid) bool
    }
    
    class SnapshotRepository {
        <<Interface>>
        +find_by_portfolio(portfolio_id: Uuid) List~Snapshot~
        +find_latest(portfolio_id: Uuid) Option~Snapshot~
        +save(snapshot: Snapshot) Snapshot
    }
```

---

## Events

| Event | Trigger | Description |
|-------|---------|-------------|
| PortfolioCreated | User creates portfolio | New portfolio initialized |
| AccountAdded | Account added to portfolio | Link created |
| AccountRemoved | Account removed from portfolio | Link removed |
| SnapshotCreated | Snapshot completed | Point-in-time record |
| AllocationChanged | Deviation from target | Guardrail alert |