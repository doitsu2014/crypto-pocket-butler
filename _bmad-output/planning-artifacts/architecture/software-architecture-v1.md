# Software Architecture — crypto-pocket-butler

**Version:** 1.0-draft  
**Date:** 2026-04-26  
**Status:** Draft — pending BMAD team review  
**Authors:** BMAD Architecture Team (Sam, Alex, Ortis, Fe, Quinn)

---

## 1. Overview

crypto-pocket-butler is a crypto portfolio management application providing:
- Multi-wallet and exchange portfolio tracking
- Rebalancing suggestions
- Daily briefs written to Notion

### Technology Stack

| Layer | Technology |
|---|---|
| **Frontend** | Next.js 16 (App Router), TypeScript, TailwindCSS 4, NextAuth.js v5 |
| **API** | Rust, Axum 0.8, SeaORM, PostgreSQL |
| **Authentication** | Keycloak (JWT validation via axum-keycloak-auth) |
| **Deployment** | Docker Compose |

---

## 2. High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        CLIENTS                               │
│  [Web Browser] ←→ [Next.js App (SSR + Client Components)]   │
│  [Mobile PWA]                                               │
└─────────────────────────┬───────────────────────────────────┘
                          │ HTTPS
┌─────────────────────────▼───────────────────────────────────┐
│                    EDGE LAYER                                │
│  [CDN / Cloudflare] — Static assets, caching                 │
│  [Keycloak] — Identity Provider (OIDC)                      │
└─────────────────────────┬───────────────────────────────────┘
                          │ JWT Auth
┌─────────────────────────▼───────────────────────────────────┐
│                 APPLICATION LAYER                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Web App    │  │   REST API    │  │  Notion      │     │
│  │  (Next.js)   │  │   (Axum)      │  │  Sync Job    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                  DOMAIN LAYER                               │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │  Portfolio   │  │    Asset     │  │   Chain      │     │
│  │  Context     │  │   Context    │  │   Context    │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│  ┌──────────────┐  ┌──────────────┐                        │
│  │  Account    │  │  Allocation  │                        │
│  │  Context    │  │   Context    │                        │
│  └──────────────┘  └──────────────┘                        │
└─────────────────────────┬───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│               INFRASTRUCTURE LAYER                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐       │
│  │  SeaORM      │  │  External    │  │    Cache     │       │
│  │  PostgreSQL  │  │  Chain RPCs  │  │   (Moka)     │       │
│  └──────────────┘  └──────────────┘  └──────────────┘       │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. Bounded Contexts (Domain Model)

### 3.1 Portfolio Context
- **Responsibility:** Portfolio management, rebalancing calculations
- **Entities:** Portfolio, Holding, RebalanceRecommendation
- **Value Objects:** AllocationTarget, TokenAmount, USDValue
- **Repository Traits:** PortfolioRepository, HoldingRepository

### 3.2 Asset Context
- **Responsibility:** Asset identity and metadata management
- **Entities:** Asset, TokenMetadata, Chain
- **Value Objects:** AssetIdentity (chain + address), Symbol
- **Repository Traits:** AssetRepository, ChainRepository

### 3.3 Chain Context
- **Responsibility:** Blockchain data retrieval and normalization
- **Entities:** ChainAccount, Wallet, ExchangeAccount
- **Value Objects:** ChainIdentifier, GasEstimate
- **Repository Traits:** ChainAccountRepository

### 3.4 Account Context
- **Responsibility:** User account management, preferences
- **Entities:** User, UserPreferences, NotificationSetting
- **Value Objects:** Email, DisplayName

### 3.5 Allocation Context
- **Responsibility:** Target allocation management
- **Entities:** AllocationRule, AllocationSnapshot
- **Value Objects:** Percentage, TokenWeight

---

## 4. API Architecture

### 4.1 Layer Structure

```
transport/http/          # HTTP handlers (Axum extractors)
    └── handlers/        # Route handlers per domain
application/
    ├── usecases/        # Application services (orchestration)
    ├── dto/             # Data Transfer Objects
    └── services/        # Application-level services
domains/
    └── {domain}/        # Per-bounded-context modules
        ├── entities.rs
        ├── value_objects.rs
        ├── repository.rs  # Trait definitions
        └── services.rs    # Domain services
infrastructure/
    ├── persistence/     # SeaORM implementations
    ├── external/        # Chain RPC clients, external APIs
    └── cache/           # Moka cache implementations
```

### 4.2 REST API Design

**Base URL:** `/api/v1`

| Endpoint | Method | Description |
|---|---|---|
| `/api/v1/portfolios` | GET, POST | List/Create portfolios |
| `/api/v1/portfolios/{id}` | GET, PUT, DELETE | Portfolio CRUD |
| `/api/v1/portfolios/{id}/holdings` | GET | Get holdings |
| `/api/v1/portfolios/{id}/rebalance` | POST | Trigger rebalance |
| `/api/v1/assets` | GET | List tracked assets |
| `/api/v1/chains` | GET | Supported chains |
| `/api/v1/accounts` | GET | User accounts |
| `/api/v1/allocations` | GET, POST | Allocation rules |

### 4.3 Authentication Flow

```
User → Next.js App → Keycloak (OIDC) → JWT Token
                                ↓
                         Next.js receives session
                                ↓
                         API requests include JWT
                                ↓
                    Axum validates JWT via Keycloak
```

- **Web:** Authorization Code + PKCE via NextAuth.js v5
- **API:** JWT Bearer token validation via `axum-keycloak-auth`

---

## 5. Frontend Architecture

### 5.1 App Router Structure

```
web/app/
├── (auth)/              # Auth-required routes (login, register)
├── (dashboard)/         # Main app routes
│   ├── portfolios/
│   ├── assets/
│   ├── settings/
│   └── admin/
├── api/                  # Route handlers (not API routes)
├── layout.tsx
└── page.tsx
```

### 5.2 Component Strategy

| Component Type | Usage |
|---|---|
| **Server Components** | Data fetching, layouts, pages |
| **Client Components** | Interactive forms, modals, real-time updates |
| **Shared UI Components** | Buttons, inputs, cards (islands of interactivity) |

### 5.3 Data Fetching

- **Server Components:** Direct API calls (no client-side fetch)
- **Client Components:** React Query or SWR for cached data
- **Streaming:** Use React Suspense for progressive loading

---

## 6. Key Architectural Decisions

### AD-001: DDD Layered Architecture (Rust)
**Decision:** Use DDD with explicit layers: domain → application → infrastructure  
**Rationale:** Clear separation enables testing, bounded contexts align with team ownership  
**Alternatives Considered:** Flat module structure — rejected due to scaling issues

### AD-002: JWT Auth via Keycloak
**Decision:** Keycloak handles identity, API validates JWTs locally  
**Rationale:** Centralized auth, no API gateway needed, supports OIDC flows  
**Trade-off:** API depends on Keycloak being available for token validation

### AD-003: Moka for In-Memory Cache
**Decision:** Use Moka with async cache for chain data and asset prices  
**Rationale:** Simple, thread-safe, integrates well with Tokio  
**Trade-off:** Cache invalidation requires careful TTL management

### AD-004: SeaORM for Persistence
**Decision:** SeaORM with PostgreSQL for all structured data  
**Rationale:** Type-safe queries, async support, migration tooling  
**Trade-off:** Runtime overhead vs raw SQL — acceptable for this scale

### AD-005: Next.js App Router with Server Components
**Decision:** Use App Router with RSC as default, client components only when needed  
**Rationale:** Better performance via SSR, improved SEO, cleaner data fetching  
**Trade-off:** Complexity of client/server boundary — mitigated by established patterns

---

## 7. Data Flow

### 7.1 Portfolio Update Flow

```
1. Cron job triggers (apalis-cron, runs every 15 minutes)
2. Job fetches all tracked wallet addresses from DB
3. For each chain, batch RPC calls to fetch balances
4. Normalize all balances to common format (decimals + USD)
5. Update holdings in PostgreSQL via SeaORM
6. Invalidate related cache entries (Moka)
7. If significant change (>5%), trigger rebalance calculation
8. Log job completion metrics
```

### 7.2 Rebalancing Flow

```
1. User requests rebalance via Web UI
2. Frontend calls POST /api/v1/portfolios/{id}/rebalance
3. Application service validates request
4. Domain service calculates optimal rebalancing trades
5. Generates RebalanceRecommendation entities
6. Stores in DB with status "pending"
7. Returns recommendations to frontend
8. User confirms → status changes to "executed"
```

### 7.3 Notion Brief Generation

```
1. Scheduled job runs daily at 08:00 UTC
2. Fetches overnight portfolio changes
3. Aggregates price movements from cached data
4. Formats brief using Notion API client
5. Creates/updates Notion page for user
6. Logs generation status and duration
```

---

## 8. Security Model

### 8.1 Authentication
- **Web:** Keycloak OIDC (Authorization Code + PKCE)
- **API:** JWT Bearer tokens validated against Keycloak JWKS
- **Token refresh:** Handled by NextAuth.js v5

### 8.2 Authorization
- Role-based access control via Keycloak groups
- **Roles:** `admin`, `user`, `viewer`
- API enforces authorization at application layer

### 8.3 Data Protection
- All traffic over HTTPS (TLS 1.3)
- Database encryption at rest (PostgreSQL)
- Environment variables for all secrets (dotenvy)
- No sensitive data in logs

### 8.4 Smart Contract Security (Future)
- Wallet operations require explicit user confirmation
- Transaction amounts displayed in USD before signing
- Blacklist/whitelist for contract addresses (per Quinn's input)

---

## 9. Scalability Considerations

### 9.1 Horizontal Scaling
- API (Rust): Stateless, can scale behind load balancer
- Next.js: Can scale with ISR and edge caching
- PostgreSQL: Read replicas for heavy read workloads

### 9.2 Caching Strategy
| Data Type | Cache | TTL |
|---|---|---|
| Asset metadata | Moka | 1 hour |
| Chain balances | Moka | 5 minutes |
| Price data | Moka | 1 minute |
| User session | Keycloak | OAuth session |

### 9.3 Background Jobs
- Use `apalis` for job processing (already in Cargo.toml)
- Worker processes scale independently of API
- Job queue backed by PostgreSQL

---

## 10. Deployment Architecture

```yaml
services:
  keycloak:
    image: keycloak/keycloak
    ports:
      - "8080:8080"

  postgres:
    image: postgres:16
    volumes:
      - pgdata:/var/lib/postgresql/data

  api:
    build: ./api
    depends_on:
      - postgres
      - keycloak
    environment:
      - DATABASE_URL=postgres://...
      - KEYCLOAK_URL=http://keycloak:8080

  web:
    build: ./web
    depends_on:
      - api
    environment:
      - NEXTAUTH_URL=http://localhost:3000

  notifier:
    build: ./api
    command: cargo run --bin notifier
    depends_on:
      - postgres
```

---

## 11. Open Questions

1. **Solana support:** Currently disabled due to dependency conflicts. Resolution needed.
2. **Notion integration:** Auth flow not finalized — API key vs OAuth?
3. **Price feed:** Current approach? CoinGecko vs custom aggregator?
4. **Multi-chain batching:** Any preference for parallel vs sequential RPC calls?

---

## 12. Next Steps

- [ ] Review this document with full BMAD team
- [ ] Resolve open questions
- [ ] Create ADR (Architecture Decision Records) for key decisions
- [ ] Produce detailed component specifications per bounded context
- [ ] Update codebase to match approved architecture