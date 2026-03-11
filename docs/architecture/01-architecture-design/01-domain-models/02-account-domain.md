# Account Domain

> **Bounded Context:** Account management, credentials, and holdings storage.
> **Aggregate Root:** Account

---

## Domain Model

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

## Aggregate Boundaries

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
    A -.->|belongs to| U
    P -.->|references| A
```

---

## Account Type Hierarchy

```mermaid
classDiagram
    class Account {
        <<abstract>>
        +Uuid id
        +Uuid user_id
        +String name
        +AccountType account_type
        +bool is_active
    }
    
    class ExchangeAccount {
        +String exchange_name
        +AccountCredentials credentials
        +connect() Connection
        +fetch_balance() List~Holding~
    }
    
    class WalletAccount {
        +String address
        +List~String~ enabled_chains
        +fetch_balance(chain) List~Holding~
    }
    
    Account <|-- ExchangeAccount
    Account <|-- WalletAccount
    
    note for ExchangeAccount "OKX, Binance, etc.\nRequires API credentials"
    note for WalletAccount "EVM wallets, Solana\nRequires address only"
```

---

## Business Rules

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

## Invariants

| Invariant | Description |
|-----------|-------------|
| Valid account type | Must be EXCHANGE or WALLET |
| Exchange requirements | exchange_name and credentials required for EXCHANGE type |
| Wallet requirements | wallet_address required for WALLET type |
| Credential encryption | API credentials must be encrypted before storage |
| Holdings normalization | All quantities stored as normalized decimal strings |
| User ownership | Account must belong to a valid user |

---

## Repository Interface

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

## Events

| Event | Trigger | Description |
|-------|---------|-------------|
| AccountCreated | User creates account | New account initialized |
| AccountActivated | Account activated | Ready for sync |
| AccountDeactivated | Account deactivated | Paused from sync |
| HoldingsSynced | Sync completed | Holdings updated |
| CredentialsUpdated | Credentials changed | Re-encrypted and stored |
| SyncFailed | Sync error | Error logged, alert if needed |

---

## Security Considerations

### Credential Storage

```mermaid
sequenceDiagram
    participant User
    participant API
    participant AccountService
    participant EncryptionService
    participant Database
    
    User->>API: Create account with credentials
    API->>AccountService: CreateAccount(credentials)
    AccountService->>EncryptionService: Encrypt(api_key, api_secret)
    EncryptionService-->>AccountService: Encrypted credentials
    AccountService->>Database: Save account
    Note over AccountService,Database: api_key_encrypted, api_secret_encrypted stored
```

### API Response Security

- Credentials are **never** returned in API responses
- `api_key_encrypted`, `api_secret_encrypted`, `passphrase_encrypted` use `#[serde(skip_serializing)]`
- Only metadata (account type, name, last_synced_at) is exposed