# Backend Architecture - Code Structure

## Code Structure Design

```mermaid
graph TB
    A[api/src/]
    M[main.rs]
    L[lib.rs]
    D[db.rs]
    C[cache.rs]
    H[handlers/]
    DM[domain/]
    E[entities/]
    CN[connectors/]
    HP[helpers/]
    J[jobs/]
    CO[concurrency/]
    
    A --> M
    A --> L
    A --> D
    A --> C
    A --> H
    A --> DM
    A --> E
    A --> CN
    A --> HP
    A --> J
    A --> CO
```

---

## Module Dependency Flow

```mermaid
graph LR
    L[lib.rs] --> M[main.rs]
    M --> H[handlers/]
    M --> D[db.rs]
    M --> J[jobs/]
    M --> C[cache.rs]
    H --> DM[domain/]
    H --> E[entities/]
    H --> CN[connectors/]
    H --> HP[helpers/]
    DM --> E
    DM --> CN
    E --> D
    CN --> HP
    J --> D
    J --> E
    J --> DM
```

---

## API Handler Structure

```mermaid
graph TD
    H[handlers/]
    H --> PF[portfolios.rs]
    H --> AC[accounts.rs]
    H --> SN[snapshots.rs]
    H --> RC[recommendations.rs]
    H --> EC[evm_chains.rs]
    H --> ET[evm_tokens.rs]
    H --> ST[solana_tokens.rs]
    H --> CH[chains.rs]
    H --> JB[jobs.rs]
    H --> MG[migrations.rs]
```

---

## Database Entity Relationships

```mermaid
erDiagram
    users ||--o{ accounts : has
    users ||--o{ portfolios : has
    accounts ||--o{ portfolio_accounts : joined
    portfolios ||--o{ portfolio_accounts : has
    portfolios ||--o{ portfolio_allocations : has
    portfolios ||--o{ snapshots : has
    accounts ||--|| holdings : contains
    assets ||--o{ asset_contracts : has
    assets ||--o{ asset_prices : has
    evm_chains ||--o{ evm_tokens : supports
    evm_chains ||--o{ asset_contracts : has
```