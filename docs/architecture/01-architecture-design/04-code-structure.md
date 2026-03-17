# Backend Architecture - Code Structure

## Code Structure Design

```mermaid
graph TB
    A[api/src/]
    M[main.rs]
    L[lib.rs]
    D[domains/]
    AP[application/]
    I[infrastructure/]
    T[transport/]
    S[shared/]

    A --> M
    A --> L
    A --> D
    A --> AP
    A --> I
    A --> T
    A --> S

    AP --> APS[services/]
    AP --> APU[usecases/]
    AP --> APJ[jobs/]
    AP --> APD[dto/]
    AP --> APC[concurrency/]
    AP --> APR[repositories/]

    I --> IP[persistence/]
    I --> IE[external/]
    I --> IC[cache/]

    IP --> IPE[entities/]
    IP --> IPR[repository impls]

    T --> TH[http/]
    TH --> THH[handlers/]
    TH --> THR[routes.rs]
    TH --> THE[error.rs]
```

---

## Layer Dependency Flow

```mermaid
graph LR
    T[transport] --> AP[application]
    AP --> D[domains]
    AP --> I[infrastructure]
    I -.implements.-> AP
```

---

## HTTP Handler Structure

```mermaid
graph TD
    H[transport/http/handlers/]
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
    accounts ||--|| account_holdings : contains
    assets ||--o{ asset_contracts : has
    assets ||--o{ asset_prices : has
    evm_chains ||--o{ evm_tokens : supports
    evm_chains ||--o{ asset_contracts : has
```

---

## Layer Descriptions

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Domain Layer** | `api/src/domains/` | Business entities and rules, no external dependencies |
| **Application Layer** | `api/src/application/` | Use cases, services, repository traits, orchestration |
| **Infrastructure Layer** | `api/src/infrastructure/` | Persistence, external APIs, caching — implements application interfaces |
| **Transport Layer** | `api/src/transport/` | HTTP handlers, routes, error mapping |

---

## Clean Architecture Principles

### Dependency Direction Rules

Dependencies always point **inward**: Transport → Application → Domain. Infrastructure implements interfaces defined by the Application layer, so the domain and application layers never depend on infrastructure details.

### Why Handlers Don't Call Infrastructure Directly

HTTP handlers (in `transport/http/handlers/`) receive injected use cases and services via Axum `Extension` state. They never import from `infrastructure/` directly. This keeps the transport layer decoupled from persistence and external-API concerns, making handlers independently testable.

### Repository Pattern

Repository *traits* are declared in `application/repositories/` (e.g. `AccountRepository`, `PortfolioRepository`). Concrete implementations live in `infrastructure/persistence/` (e.g. `AccountRepositoryImpl`). The application layer depends only on the trait, not the implementation.

### Use Case Pattern

Each domain-facing operation is encapsulated in a use-case struct under `application/usecases/` (e.g. `AccountUseCases`, `ChainUseCases`, `PortfolioUseCases`). Use cases are constructed once at startup in `main.rs`, wrapped in `Arc`, and shared across handlers via Axum `Extension` injection.