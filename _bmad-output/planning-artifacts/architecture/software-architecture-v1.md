# Software Architecture — crypto-pocket-butler

**Version:** 2.0-revised
**Date:** 2026-04-26
**Status:** Revised — post BMAD team review
**Authors:** BMAD Architecture Team (Sam, Alex, Ortis, Fe, Quinn)

---

## 1. Overview

crypto-pocket-butler is a crypto portfolio management application providing:
- Multi-wallet and exchange portfolio tracking
- Rebalancing suggestions

### Technology Stack

| Layer | Technology |
|---|---|
| **Frontend** | Next.js 16 (App Router), TypeScript, TailwindCSS 4, NextAuth.js v5 |
| **API** | Rust, Axum 0.8, SeaORM, PostgreSQL |
| **Authentication** | API key middleware (v1); Keycloak OIDC (future multi-user) |
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
└─────────────────────────┬───────────────────────────────────┘
                          │ API Key / JWT (future)
┌─────────────────────────▼───────────────────────────────────┐
│                 APPLICATION LAYER                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Web App    │  │   REST API    │  │  Portfolio   │     │
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

**v1 (current):** API key middleware
- API key passed via `X-API-Key` header
- Validated in Axum middleware against database-stored keys
- Simple, self-contained, no external dependencies

**Future (multi-user):** Keycloak OIDC
- Migration path: replace API key middleware with JWT validation
- Keycloak handles identity, API validates JWTs locally via JWKS
- Authorization Code + PKCE via NextAuth.js v5 for web

> **Decision (AD-006):** Use API key auth for v1 to eliminate Keycloak dependency. Migrate to Keycloak OIDC when multi-user support is needed.

---

## 5. Frontend Architecture

### 5.1 App Router Structure

```
web/app/
├── (auth)/              # Auth-required routes (login, register)
├── (dashboard)/         # Main app routes
│   ├── portfolios/
│   ├── assets/
│   └── settings/
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
- **ISR:** Use Incremental Static Regeneration for portfolio dashboard pages

---

## 6. Key Architectural Decisions

### AD-001: DDD Layered Architecture (Rust)
**Decision:** Use DDD with explicit layers: domain → application → infrastructure
**Rationale:** Clear separation enables testing, bounded contexts align with team ownership
**Alternatives Considered:** Flat module structure — rejected due to scaling issues

### AD-002: API Key Auth (v1)
**Decision:** API key middleware for self-contained auth, no external IdP dependency
**Rationale:** Eliminates Keycloak operational burden for single-user v1. Simple, testable, no external deps.
**Trade-off:** No OIDC/OAuth2 features. Upgrade path needed when multi-user support is added.

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

### AD-006: RPC Abstraction Layer
**Decision:** Abstract external chain RPC calls behind a trait-based interface with failover support
**Rationale:** Multi-chain DeFi requires hitting Ethereum, Arbitrum, Base, Solana — each with different rate limits, failure modes, and cost profiles. Public RPCs hit rate limits; private RPCs (Alchemy, QuickNode) add cost and dependency.
**Structure:**
```
trait ChainRpc {
    async fn get_balance(&self, address: &str) -> Result<Balance>;
    async fn get_tokens(&self, address: &str) -> Result<Vec<TokenBalance>>;
}
struct MultiChainRouter {
    chains: HashMap<ChainId, Box<dyn ChainRpc>>,
    fallback: Box<dyn ChainRpc>,  // failover RPC
}
```
**Trade-off:** Complexity of managing multiple RPC providers. Monitor rate limits and costs.

---

## 7. Data Flow

### 7.1 Portfolio Update Flow

```
1. Cron job triggers (apalis-cron, configurable interval — default 15 min)
2. Job fetches all tracked wallet addresses from DB
3. MultiChainRouter batches RPC calls per chain
4. Use WebSocket subscriptions where available (Ethereum, Solana)
5. Fall back to polling for chains without WS support
6. Normalize all balances to common format (decimals + USD)
7. Update holdings in PostgreSQL via SeaORM
8. Invalidate related cache entries (Moka)
9. If significant change (>5%), trigger rebalance calculation
10. Log job completion metrics
11. Alert on failures (see §9 Monitoring)
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

### 7.3 Cache Invalidation Policy

| Data Type | Cache | TTL | Invalidation |
|---|---|---|---|
| Asset metadata | Moka | 1 hour | TTL only |
| Chain balances | Moka | 5 minutes | TTL + event-driven on wallet tx |
| Price data | Moka | 1 minute | TTL only |

> **Policy:** Distinguish between "stale data that's wrong" (aggressive invalidation) and "data not yet refreshed" (graceful staleness). Portfolio balances use 5-min TTL with event-driven invalidation when wallet activity is detected.

---

## 8. Security Model

### 8.1 Authentication
- **v1:** API key via `X-API-Key` header
- **Future:** Keycloak OIDC (JWT Bearer tokens)

### 8.2 Authorization
- Role-based access control (admin, user, viewer)
- API enforces authorization at application layer

### 8.3 Data Protection
- All traffic over HTTPS (TLS 1.3)
- Database encryption at rest (PostgreSQL)
- Environment variables for all secrets (dotenvy)
- No sensitive data in logs

### 8.4 Audit Trail
- **PostgreSQL audit table:** `portfolio_audit_log` records all portfolio changes
  - Columns: `id`, `user_id`, `portfolio_id`, `action`, `old_value`, `new_value`, `timestamp`
- Replaces Notion as audit/backup log
- Queryable for compliance and recovery

### 8.5 Smart Contract Security
- Wallet operations require explicit user confirmation
- Transaction amounts displayed in USD before signing
- Blacklist/whitelist for contract addresses

---

## 9. Monitoring & Operations

### 9.1 Cron Job Monitoring
- Failed cron jobs trigger alerts (PagerDuty/email)
- Metrics: job success rate, duration, records processed
- Silent failures are unacceptable — all job executions logged

### 9.2 Secrets Management
- All API keys, RPC credentials, DB passwords in `.env` (not committed to git)
- Notion API keys (if any) revoked and removed — no stale secrets

### 9.3 Dependency Management
- Rust crate versions pinned in `Cargo.lock`
- Regular `cargo audit` runs in CI
- Third-party dependencies reviewed quarterly

---

## 10. Scalability Considerations

### 10.1 Horizontal Scaling
- API (Rust): Stateless, can scale behind load balancer
- Next.js: Can scale with ISR and edge caching
- PostgreSQL: Read replicas for heavy read workloads

### 10.2 Caching Strategy
See §7.3 Cache Invalidation Policy

### 10.3 Background Jobs
- Use `apalis` for job processing (already in Cargo.toml)
- Worker processes scale independently of API
- Job queue backed by PostgreSQL

---

## 11. Deployment Architecture

```yaml
services:
  postgres:
    image: postgres:16
    volumes:
      - pgdata:/var/lib/postgresql/data

  api:
    build: ./api
    depends_on:
      - postgres
    environment:
      - DATABASE_URL=postgres://...
      - API_KEY=<secret>

  web:
    build: ./web
    depends_on:
      - api
    environment:
      - NEXTAUTH_URL=http://localhost:3000
      - API_KEY=<secret>

  notifier:
    build: ./api
    command: cargo run --bin notifier
    depends_on:
      - postgres
```

> **Note:** Keycloak removed from deployment for v1. API key auth is self-contained.

---

## 12. Open Questions (Resolved)

| # | Question | Resolution |
|---|---|---|
| 1 | Solana support | Deferred — dependency conflicts to resolve post-v1 |
| 2 | Notion integration | **Removed** — no daily briefs, no Notion sync |
| 3 | Price feed | Use CoinGecko for v1, keep abstraction for future swap |
| 4 | Multi-chain batching | WebSocket subscriptions where available, polling fallback |

---

## 13. Next Steps

- [ ] Update codebase: remove Notion sync components, Notion API client, sync job
- [ ] Audit UI components: remove `NotionSyncStatus`, `NotionConnectionCard`, sync toggles
- [ ] Strip API responses of Notion-related fields (`notion_synced_at`, `notion_page_id`)
- [ ] Implement API key middleware in Axum
- [ ] Add `portfolio_audit_log` table to SeaORM schema
- [ ] Implement MultiChainRouter trait for RPC abstraction
- [ ] Add cron job alerting/monitoring
- [ ] Create ADR for API key auth decision (AD-006)

---

## Appendix: What Was Removed

| Removed | Reason |
|---|---|
| Notion sync job | Not in scope — no daily briefs, no Notion integration |
| Notion API client | Unused without sync job |
| Notion brief generation flow | Not in scope |
| Keycloak (v1) | Overkill for single-user v1; API key auth sufficient |
| Notion-related UI components | No longer needed |
| Notion-related API response fields | Cleaner API surface |