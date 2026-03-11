# Chain & Token Domain

> **Bounded Context:** Blockchain network configurations and token specifications.
> **Aggregate Root:** EvmChain

---

## Domain Model

```mermaid
classDiagram
    class EvmChain {
        <<Aggregate Root>>
        +Uuid id
        +String name
        +String chain_id
        +String rpc_url
        +String native_symbol
        +String explorer_url
        +add_token(token)
        +get_token(contract_address)
    }
    
    class ChainContract {
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
        +bool is_native
    }
    
    class SolanaChain {
        <<Aggregate Root>>
        +String name
        +String rpc_url
        +add_token(token)
    }
    
    class SolanaToken {
        <<Entity>>
        +Uuid id
        +String mint_address
        +String symbol
        +u8 decimals
    }
    
    EvmChain "1" --> "0..*" ChainContract
    EvmChain "1" --> "0..*" EvmToken
    SolanaChain "1" --> "0..*" SolanaToken
```

---

## Supported Chains

```mermaid
graph TB
    subgraph "EVM Chains"
        ETH[Ethereum<br/>chain_id: 1]
        ARB[Arbitrum<br/>chain_id: 42161]
        BSC[BSC<br/>chain_id: 56]
        POL[Polygon<br/>chain_id: 137]
        OP[Optimism<br/>chain_id: 10]
        BASE[Base<br/>chain_id: 8453]
        AVAX[Avalanche<br/>chain_id: 43114]
    end
    
    subgraph "Non-EVM Chains"
        SOL[Solana]
    end
    
    subgraph "Connectors"
        EVMC[EVM Connector]
        SOLC[Solana Connector]
    end
    
    ETH --> EVMC
    ARB --> EVMC
    BSC --> EVMC
    POL --> EVMC
    OP --> EVMC
    BASE --> EVMC
    AVAX --> EVMC
    
    SOL --> SOLC
```

---

## Token Resolution Flow

```mermaid
flowchart TD
    subgraph "EVM Token Resolution"
        EVM1[Receive contract address]
        EVM2[Look up in EvmToken table]
        EVM3{Found?}
        EVM4[Return symbol + decimals]
        EVM5[Fallback to asset_contracts]
        EVM6[Return asset symbol]
        
        EVM1 --> EVM2 --> EVM3
        EVM3 -->|Yes| EVM4
        EVM3 -->|No| EVM5 --> EVM6
    end
    
    subgraph "Solana Token Resolution"
        SOL1[Receive mint address]
        SOL2[Look up in SolanaToken table]
        SOL3{Found?}
        SOL4[Return symbol + decimals]
        SOL5[Return mint address as symbol]
        
        SOL1 --> SOL2 --> SOL3
        SOL3 -->|Yes| SOL4
        SOL3 -->|No| SOL5
    end
```

---

## Chain Configuration

### EVM Chain Properties

| Property | Type | Description |
|----------|------|-------------|
| id | Uuid | Unique identifier |
| name | String | Display name (e.g., "ethereum") |
| chain_id | String | Chain ID for RPC calls |
| rpc_url | String | JSON-RPC endpoint |
| native_symbol | String | Native token symbol (e.g., "ETH") |
| explorer_url | String | Block explorer URL |

### Example Configuration

```json
{
  "id": "uuid-here",
  "name": "ethereum",
  "chain_id": "1",
  "rpc_url": "https://eth.llamarpc.com",
  "native_symbol": "ETH",
  "explorer_url": "https://etherscan.io"
}
```

---

## Business Rules

| Rule | Description |
|------|-------------|
| Native tokens | Each chain has a native token (ETH, BNB, SOL) |
| Contract uniqueness | One contract address per token per chain |
| Chain validation | Only configured chains can be queried |
| Enabled chains per wallet | Users specify which chains to track |

---

## Repository Interface

```mermaid
classDiagram
    class EvmChainRepository {
        <<Interface>>
        +find_all() List~EvmChain~
        +find_by_name(name: String) Option~EvmChain~
        +find_by_chain_id(chain_id: String) Option~EvmChain~
        +save(chain: EvmChain) EvmChain
    }
    
    class EvmTokenRepository {
        <<Interface>>
        +find_by_chain(chain: String) List~EvmToken~
        +find_by_address(address: String, chain: String) Option~EvmToken~
        +save(token: EvmToken) EvmToken
    }
    
    class SolanaTokenRepository {
        <<Interface>>
        +find_all() List~SolanaToken~
        +find_by_mint(mint_address: String) Option~SolanaToken~
        +save(token: SolanaToken) SolanaToken
    }
```

---

## Token Tables Schema

### evm_tokens

| Column | Type | Description |
|--------|------|-------------|
| id | Uuid | Primary key |
| symbol | String | Token symbol |
| chain | String | Chain name |
| contract_address | String | Token contract |
| decimals | u8 | Token decimals |
| is_native | bool | Is native token? |

### solana_tokens

| Column | Type | Description |
|--------|------|-------------|
| id | Uuid | Primary key |
| mint_address | String | SPL token mint |
| symbol | String | Token symbol |
| decimals | u8 | Token decimals |

---

## Events

| Event | Trigger | Description |
|-------|---------|-------------|
| ChainAdded | New chain configured | Available for wallets |
| TokenAdded | New token discovered | Added to registry |
| TokenUpdated | Token info changed | Symbol or decimals updated |