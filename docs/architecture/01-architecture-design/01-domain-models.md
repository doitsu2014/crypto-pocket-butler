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
    
    Portfolio "1" --> "0..*" PortfolioAccount
    Portfolio "1" --> "0..*" PortfolioAllocation
    Portfolio "1" --> "0..*" Snapshot
    PortfolioAccount --> Account
    Portfolio --> User
```

---

### Account Domain

> **Bounded Context:** Account management, credentials, and holdings storage.
> **Aggregate Root:** Account

```mermaid
classDiagram
    class Account {
        <<Aggregate Root>>
        +Uuid id
        +Uuid user_id
        +String name
        +AccountType account_type
        +bool is_active
        +DateTime last_synced_at
        +DateTime created_at
        +DateTime updated_at
        +add_holding(holding)
        +remove_holding(asset)
        +sync_holdings(holdings)
        +activate()
        +deactivate()
    }
    
    class AccountType {
        <<Enumeration>>
        EXCHANGE
        WALLET
    }
    
    class ExchangeAccount {
        +String exchange_name
        +AccountCredentials credentials
    }
    
    class WalletAccount {
        +String address
        +List~String~ enabled_chains
    }
    
    class AccountCredentials {
        <<Value Object>>
        +String api_key_encrypted
        +String api_secret_encrypted
        +String passphrase_encrypted
    }
    
    class AccountHolding {
        <<Entity>>
        +String asset
        +Decimal quantity
        +Decimal available
        +Decimal frozen
        +u8 decimals
        +Decimal total_value_usd
    }
    
    class AccountHoldings {
        <<Value Object>>
        +List~AccountHolding~ items
        +Decimal total_value_usd
        +add(holding)
        +remove(asset)
        +find(asset)
    }
    
    Account "1" --> "1" AccountType
    Account <|-- ExchangeAccount
    Account <|-- WalletAccount
    ExchangeAccount "1" --> "1" AccountCredentials
    WalletAccount "1" --> "*" String : enabled_chains
    Account "1" --> "1" AccountHoldings : holdings
    AccountHoldings "1" --> "*" AccountHolding
```

---

### Account Domain - Aggregate Boundaries

```mermaid
graph TB
    subgraph "Account Aggregate"
        A[Account<br/>Aggregate Root]
        H[AccountHoldings]
        AH[AccountHolding]
        
        A --> H
        H --> AH
    end
    
    subgraph "Value Objects"
        C[AccountCredentials]
        WA[WalletAddress]
        AT[AccountType]
    end
    
    subgraph "External References"
        U[User]
        P[Portfolio]
    end
    
    A --> C
    A --> WA
    A --> AT
    A -.-> U : belongs to
    P -.-> A : references
```

---

### Account Domain - Invariants & Business Rules

```mermaid
flowchart TD
    subgraph "Account Creation Rules"
        AC1[Account Type Required]
        AC2{Exchange or Wallet?}
        AC3[Exchange: requires<br/>exchange_name + credentials]
        AC4[Wallet: requires<br/>wallet_address]
        
        AC1 --> AC2
        AC2 -->|Exchange| AC3
        AC2 -->|Wallet| AC4
    end
    
    subgraph "Holdings Sync Rules"
        HS1[Sync triggered]
        HS2[Fetch from connector]
        HS3[Normalize quantities]
        HS4[Replace existing holdings]
        HS5[Update last_synced_at]
        
        HS1 --> HS2 --> HS3 --> HS4 --> HS5
    end
    
    subgraph "Credential Security"
        CS1[API Key must be encrypted]
        CS2[API Secret must be encrypted]
        CS3[Never expose in API responses]
        CS4[Encrypt before persistence]
        
        CS1 --> CS4
        CS2 --> CS4
        CS3 -.-> CS4
    end
```

---

### Account Domain - Repository Interface

```mermaid
classDiagram
    class AccountRepository {
        <<Interface>>
        +find_by_id(id: Uuid) Option~Account~
        +find_by_user_id(user_id: Uuid) List~Account~
        +save(account: Account) Account
        +delete(id: Uuid) bool
        +find_active_by_type(type: AccountType) List~Account~
    }
    
    class AccountRepositoryImpl {
        +DatabaseConnection db
        +find_by_id(id: Uuid) Option~Account~
        +find_by_user_id(user_id: Uuid) List~Account~
        +save(account: Account) Account
        +delete(id: Uuid) bool
        +find_active_by_type(type: AccountType) List~Account~
    }
    
    class AccountCache {
        <<Service>>
        +get_holdings(account_id) Option~AccountHoldings~
        +set_holdings(account_id, holdings)
        +invalidate(account_id)
    }
    
    AccountRepository <|.. AccountRepositoryImpl
    AccountRepositoryImpl --> AccountCache
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

---

## Domain Relationships Overview

```mermaid
graph LR
    subgraph "User Context"
        U[User]
    end
    
    subgraph "Account Context"
        A[Account]
        AH[AccountHoldings]
    end
    
    subgraph "Portfolio Context"
        P[Portfolio]
        PA[PortfolioAccount]
        PS[PortfolioSnapshot]
    end
    
    subgraph "Asset Context"
        AS[Asset]
        AP[AssetPrice]
    end
    
    U --> A
    U --> P
    A --> AH
    P --> PA
    PA --> A
    PS --> AH : aggregates
    AH --> AS : references
    AS --> AP
```

---

## DDD Design Decisions

### Why Separate Account Domain?

1. **Single Responsibility**: Account management is a distinct bounded context with its own invariants
2. **Aggregate Root**: Account controls access to its holdings and credentials
3. **Type Safety**: Different account types (Exchange vs Wallet) have different constraints
4. **Security**: Credentials handling is isolated within the aggregate
5. **Testability**: Account domain logic can be tested independently

### Aggregate Boundaries

| Aggregate Root | Contains | Invariants |
|---------------|----------|------------|
| Account | AccountHoldings, Credentials | Holdings consistency, credential encryption |
| Portfolio | PortfolioAllocations, Snapshots | Allocation weights sum to 100% |
| Asset | AssetPrices, AssetContracts | Price uniqueness per chain |

### Value Objects

| Value Object | Purpose |
|-------------|---------|
| AccountCredentials | Encapsulates encrypted API credentials |
| AccountType | Enumeration of account types |
| WalletAddress | Validates and encapsulates wallet address |
| AccountHolding | Immutable holding snapshot |