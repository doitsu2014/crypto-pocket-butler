# Asset Domain

> **Bounded Context:** Asset definitions, pricing, and contract addresses.
> **Aggregate Root:** Asset

---

## Domain Model

```mermaid
classDiagram
    class Asset {
        <<Aggregate Root>>
        +String symbol
        +String name
        +String rank
        +add_price(chain, price)
        +add_contract(chain, address)
        +get_current_price(chain)
    }
    
    class AssetPrice {
        <<Entity>>
        +String asset
        +String chain
        +Decimal price_usd
        +DateTime updated_at
    }
    
    class AssetContract {
        <<Entity>>
        +String asset
        +String chain
        +String contract_address
        +u8 decimals
    }
    
    class EvmToken {
        <<Entity>>
        +Uuid id
        +String symbol
        +String chain
        +String contract_address
        +u8 decimals
    }
    
    class SolanaToken {
        <<Entity>>
        +Uuid id
        +String mint_address
        +String symbol
        +u8 decimals
    }
    
    Asset "1" --> "0..*" AssetPrice
    Asset "1" --> "0..*" AssetContract
    Asset "1" --> "0..*" EvmToken
    Asset "1" --> "0..*" SolanaToken
```

---

## Aggregate Boundaries

```mermaid
graph TB
    subgraph "Asset Aggregate"
        A[Asset<br/>Aggregate Root]
        AP[AssetPrice]
        AC[AssetContract]
        ET[EvmToken]
        ST[SolanaToken]
        
        A --> AP
        A --> AC
        A --> ET
        A --> ST
    end
    
    subgraph "Price Providers"
        CG[CoinGecko]
        CP[CoinPaprika]
    end
    
    subgraph "Chain Networks"
        ETH[Ethereum]
        ARB[Arbitrum]
        SOL[Solana]
    end
    
    CG -.-> AP
    CP -.-> AP
    ET -.-> ETH
    ET -.-> ARB
    ST -.-> SOL
```

---

## Asset Identity Resolution

```mermaid
flowchart TD
    subgraph "Identity Resolution"
        AR1[Receive token from chain]
        AR2{Known chain?}
        AR3[Look up by contract/mint]
        AR4{Found in registry?}
        AR5[Use mapped symbol]
        AR6[Create new mapping]
        AR7[Return asset symbol]
        
        AR1 --> AR2
        AR2 -->|No| AR6
        AR2 -->|Yes| AR3
        AR3 --> AR4
        AR4 -->|Yes| AR5
        AR4 -->|No| AR6
        AR5 --> AR7
        AR6 --> AR7
    end
```

---

## Price Collection Flow

```mermaid
sequenceDiagram
    participant Scheduler
    participant PriceJob
    participant CoinGecko
    participant CoinPaprika
    participant Database
    
    Scheduler->>PriceJob: Trigger price collection
    PriceJob->>CoinGecko: Fetch prices
    CoinGecko-->>PriceJob: Price list
    PriceJob->>CoinPaprika: Fetch prices (fallback)
    CoinPaprika-->>PriceJob: Price list
    
    loop For each asset
        PriceJob->>Database: Upsert AssetPrice
    end
    
    Note over PriceJob,Database: Prices stored with chain context
```

---

## Business Rules

| Rule | Description |
|------|-------------|
| Symbol uniqueness | Asset symbol must be unique across the system |
| Chain-specific prices | Same asset can have different prices per chain |
| Contract uniqueness | One contract address per asset per chain |
| Rank updates | Asset rank updated from price providers |
| Decimals preserved | Token decimals stored for balance normalization |

---

## Repository Interface

```mermaid
classDiagram
    class AssetRepository {
        <<Interface>>
        +find_by_symbol(symbol: String) Option~Asset~
        +find_all() List~Asset~
        +find_by_rank(limit: int) List~Asset~
        +save(asset: Asset) Asset
    }
    
    class AssetPriceRepository {
        <<Interface>>
        +find_price(asset, chain) Option~AssetPrice~
        +find_prices(assets) Map~String,Decimal~
        +save_price(price: AssetPrice) AssetPrice
    }
    
    class AssetContractRepository {
        <<Interface>>
        +find_contract(asset, chain) Option~AssetContract~
        +find_by_address(address, chain) Option~AssetContract~
        +save(contract: AssetContract) AssetContract
    }
```

---

## Events

| Event | Trigger | Description |
|-------|---------|-------------|
| PriceUpdated | New price fetched | Asset price changed |
| AssetAdded | New asset discovered | Added to registry |
| ContractAdded | New contract mapped | Chain-specific address |
| RankUpdated | Rank changed | Asset ranking updated |