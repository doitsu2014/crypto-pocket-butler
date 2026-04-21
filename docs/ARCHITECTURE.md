# Crypto Pocket Butler - Technical Architecture

**Version:** 1.0
**Last Updated:** 2026-04-21
**Owner:** Alex (Crypto Solution Architect)
**Status:** Draft

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              CLIENT LAYER                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐             │
│  │   Web App       │  │   Mobile App    │  │   Desktop App   │             │
│  │   (React/Next)  │  │   (React Native)│  │   (Electron)    │             │
│  └────────┬────────┘  └────────┬────────┘  └────────┬────────┘             │
│           │                    │                    │                       │
│           └────────────────────┼────────────────────┘                       │
│                                │                                            │
└────────────────────────────────┼────────────────────────────────────────────┘
                                 │ HTTPS / WSS
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                              API GATEWAY                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │                        Kong / NGINX                                  │   │
│  │  • Rate Limiting    • Authentication    • Request Routing           │   │
│  │  • CORS             • SSL Termination   • Load Balancing            │   │
│  └─────────────────────────────────────────────────────────────────────┘   │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                           MICROSERVICES LAYER                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │    Wallet    │  │   Portfolio  │  │   Pricing    │  │   Transaction │   │
│  │   Service    │  │   Service    │  │   Service    │  │    Service    │   │
│  │              │  │              │  │              │  │               │   │
│  │ • Connect    │  │ • Aggregate  │  │ • Real-time  │  │ • History     │   │
│  │ • Validate   │  │ • Valuation  │  │ • Fallback   │  │ • Classification│ │
│  │ • Track      │  │ • P&L        │  │ • Cache      │  │ • Export      │   │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘   │
│         │                 │                 │                 │            │
│  ┌──────┴───────┐  ┌──────┴───────┐  ┌──────┴───────┐  ┌──────┴───────┐   │
│  │     User     │  │  Analytics   │  │  Rebalance   │  │ Compliance   │   │
│  │   Service    │  │   Service    │  │   Service    │  │   Service    │   │
│  │              │  │              │  │              │  │              │   │
│  │ • Auth       │  │ • Performance│  │ • Drift      │  │ • KYC/AML    │   │
│  │ • RBAC       │  │ • Attribution│  │ • Recommend  │  │ • Screening  │   │
│  │ • Audit      │  │ • Risk       │  │ • Execute    │  │ • Reporting  │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                              │
└────────────────────────────────┬────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                            DATA LAYER                                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │   PostgreSQL │  │    Redis     │  │  TimescaleDB │  │ Elasticsearch │   │
│  │              │  │              │  │              │  │               │   │
│  │ • Users      │  │ • Sessions   │  │ • Price      │  │ • Search      │   │
│  │ • Portfolios │  │ • Cache      │  │ • Timeseries │  │ • Logs        │   │
│  │ • RBAC       │  │ • Rate Limit │  │ • Analytics  │  │ • Audit       │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                      │
│  │   Kafka      │  │    S3        │  │  ClickHouse  │                      │
│  │              │  │              │  │              │                      │
│  │ • Events     │  │ • Files      │  │ • OLAP       │                      │
│  │ • Streams    │  │ • Exports    │  │ • Reports    │                      │
│  └──────────────┘  └──────────────┘  └──────────────┘                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         EXTERNAL INTEGRATIONS                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │
│  │  Price APIs  │  │  Blockchain  │  │  Wallet SDKs │  │  DEX APIs    │   │
│  │              │  │   Indexers   │  │              │  │              │   │
│  │ • CoinGecko  │  │ • TheGraph   │  │ • WalletConn │  │ • Uniswap    │   │
│  │ • CoinMarket │  │ • Alchemy    │  │ • MetaMask   │  │ • 1inch      │   │
│  │ • Chainlink  │  │ • Infura     │  │ • Ledger     │  │ • CowSwap    │   │
│  └──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘   │
│                                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐                      │
│  │ Compliance   │  │  Identity    │  │  Analytics   │                      │
│  │              │  │              │  │              │                      │
│  │ • Chainalysis│  │ • Auth0      │  │ • PostHog    │                      │
│  │ • Elliptic   │  │ • Okta      │  │ • Mixpanel   │                      │
│  └──────────────┘  └──────────────┘  └──────────────┘                      │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Core Services

### 1. Wallet Service

**Responsibility:** Multi-wallet connectivity, address tracking, transaction indexing

```yaml
Endpoints:
  POST /api/v1/wallets/connect      # Connect new wallet
  GET  /api/v1/wallets              # List connected wallets
  DELETE /api/v1/wallets/{id}       # Disconnect wallet
  GET  /api/v1/wallets/{id}/txs     # Get wallet transactions

Features:
  - WalletConnect v2 protocol
  - MetaMask SDK integration
  - Hardware wallet support (Ledger, Trezor)
  - Read-only address import
  - Multi-chain address tracking
  - Transaction webhook listeners

Supported Chains:
  - EVM: Ethereum, BSC, Polygon, Arbitrum, Optimism, Base, Avalanche
  - Non-EVM: Solana, Bitcoin (via wrappers), Cosmos
```

### 2. Portfolio Service

**Responsibility:** Portfolio aggregation, valuation, P&L calculation

```yaml
Endpoints:
  GET  /api/v1/portfolios                    # Get portfolio summary
  GET  /api/v1/portfolios/{id}/assets        # List portfolio assets
  GET  /api/v1/portfolios/{id}/performance   # Performance metrics
  POST /api/v1/portfolios/{id}/rebalance     # Trigger rebalance

Features:
  - Multi-wallet aggregation
  - Real-time valuation
  - Cost basis tracking (FIFO, LIFO, HIFO)
  - P&L calculation (realized/unrealized)
  - Asset allocation analysis
  - Historical portfolio snapshots

Valuation Pipeline:
  1. Fetch prices from primary source (CoinGecko)
  2. Fallback to secondary (CoinMarketCap)
  3. On-chain DEX prices (Uniswap TWAP)
  4. Cached prices (max 5min stale)
```

### 3. Pricing Service

**Responsibility:** Real-time price aggregation, caching, fallback logic

```yaml
Endpoints:
  GET  /api/v1/prices/{symbol}               # Get current price
  GET  /api/v1/prices/historical             # Historical prices
  WS /api/v1/prices/stream                   # WebSocket price stream

Price Sources:
  Primary:   CoinGecko API (free tier: 10-50 calls/min)
  Secondary: CoinMarketCap API
  Tertiary:  Chainlink Price Feeds
  Fallback:  Uniswap V3 TWAP (on-chain)

Cache Strategy:
  - Hot tokens (top 100): 30 second TTL
  - Mid tokens (100-1000): 2 minute TTL
  - Long tail: 10 minute TTL
  - Stale-while-revalidate pattern
```

### 4. Transaction Service

**Responsibility:** Transaction history, classification, export

```yaml
Endpoints:
  GET  /api/v1/transactions                  # List transactions
  GET  /api/v1/transactions/{id}             # Transaction details
  POST /api/v1/transactions/classify         # Manual classification
  GET  /api/v1/transactions/export           # Export (CSV, PDF)

Features:
  - Automatic transaction classification
  - DeFi protocol detection (swap, stake, provide liquidity)
  - NFT transaction tracking
  - Gas fee tracking
  - Income detection (staking rewards, airdrops)
  - Multi-format export (CSV, PDF, API)

Classification Rules:
  - Swap: Token transfer + DEX interaction
  - Stake: Contract interaction with staking pool
  - Transfer: Simple token transfer
  - Bridge: Cross-chain bridge contract
  - NFT: ERC-721/1155 transfer
```

### 5. User Service

**Responsibility:** Authentication, RBAC, audit logging

```yaml
Endpoints:
  POST /api/v1/auth/login                    # Login
  POST /api/v1/auth/register                 # Register
  POST /api/v1/auth/refresh                  # Refresh token
  GET  /api/v1/users/me                      # Current user
  PUT  /api/v1/users/me                      # Update profile

Features:
  - JWT-based authentication
  - OAuth2 (Google, GitHub)
  - 2FA (TOTP, WebAuthn)
  - Role-based access control (RBAC)
  - Audit logging (all actions)
  - Session management

Roles:
  - Owner: Full access, billing, user management
  - Admin: Portfolio management, no billing
  - Analyst: Read-only, analytics access
  - Viewer: Limited view (specific portfolios)
```

### 6. Analytics Service

**Responsibility:** Performance metrics, attribution, risk analytics

```yaml
Endpoints:
  GET  /api/v1/analytics/performance         # Performance metrics
  GET  /api/v1/analytics/attribution         # Return attribution
  GET  /api/v1/analytics/risk                # Risk metrics
  GET  /api/v1/analytics/benchmark           # Benchmark comparison

Metrics:
  Performance:
    - Total Return (%)
    - IRR (Internal Rate of Return)
    - CAGR (Compound Annual Growth Rate)
    - Sharpe Ratio
    - Sortino Ratio
    - Max Drawdown

  Attribution:
    - By Asset (contribution to returns)
    - By Chain
    - By Strategy (DeFi, NFT, Trading)
    - Sector Exposure

  Risk:
    - Portfolio Volatility
    - Value at Risk (VaR)
    - Concentration Risk
    - Correlation Matrix
```

### 7. Rebalance Service

**Responsibility:** Rebalancing logic, recommendations, execution

```yaml
Endpoints:
  GET  /api/v1/rebalance/targets             # Get target allocations
  PUT  /api/v1/rebalance/targets             # Set target allocations
  GET  /api/v1/rebalance/opportunities       # Rebalance opportunities
  POST /api/v1/rebalance/execute             # Execute rebalance

Features:
  - Target allocation setup
  - Drift detection (>5% threshold)
  - Tax-aware rebalancing
  - Gas cost optimization
  - DEX aggregator integration (1inch, CowSwap)
  - DCA scheduling

Rebalance Strategies:
  - Threshold-based: Trigger when drift > X%
  - Time-based: Monthly, quarterly
  - Manual: User-initiated
  - Smart: ML-based optimization
```

### 8. Compliance Service

**Responsibility:** KYC/AML, screening, reporting

```yaml
Endpoints:
  POST /api/v1/compliance/kyc                # Submit KYC
  GET  /api/v1/compliance/status             # Compliance status
  POST /api/v1/compliance/screen             # Address screening
  GET  /api/v1/compliance/reports            # Generate reports

Features:
  - Sanctioned address screening (OFAC)
  - KYC verification (Sumsub, Jumio)
  - Transaction monitoring
  - Tax report generation
  - Audit trail export
  - Geographic restrictions

Reports:
  - Capital Gains (Form 8949)
  - Income Summary (Schedule D)
  - Transaction History
  - Portfolio Holdings
  - Audit Trail
```

---

## Database Schema

### Core Tables

```sql
-- Users & Authentication
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE TABLE user_roles (
    user_id UUID REFERENCES users(id),
    role VARCHAR(50) NOT NULL, -- owner, admin, analyst, viewer
    portfolio_id UUID,
    PRIMARY KEY (user_id, role, portfolio_id)
);

-- Portfolios
CREATE TABLE portfolios (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    name VARCHAR(255) NOT NULL,
    base_currency VARCHAR(10) DEFAULT 'USD',
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Wallets
CREATE TABLE wallets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    portfolio_id UUID REFERENCES portfolios(id),
    address VARCHAR(255) NOT NULL,
    chain VARCHAR(50) NOT NULL,
    wallet_type VARCHAR(50), -- metamask, walletconnect, ledger
    label VARCHAR(255),
    created_at TIMESTAMP DEFAULT NOW()
);

-- Holdings
CREATE TABLE holdings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    portfolio_id UUID REFERENCES portfolios(id),
    wallet_id UUID REFERENCES wallets(id),
    token_address VARCHAR(255),
    chain VARCHAR(50),
    balance DECIMAL(78, 18),
    cost_basis DECIMAL(78, 18),
    acquired_at TIMESTAMP,
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Transactions
CREATE TABLE transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_id UUID REFERENCES wallets(id),
    tx_hash VARCHAR(255) NOT NULL,
    block_number BIGINT,
    timestamp TIMESTAMP NOT NULL,
    type VARCHAR(50), -- transfer, swap, stake, bridge
    from_address VARCHAR(255),
    to_address VARCHAR(255),
    token_address VARCHAR(255),
    amount DECIMAL(78, 18),
    gas_used DECIMAL(78, 18),
    gas_price DECIMAL(78, 18),
    status VARCHAR(50), -- pending, confirmed, failed
    created_at TIMESTAMP DEFAULT NOW()
);

-- Prices (TimescaleDB hypertable)
CREATE TABLE prices (
    time TIMESTAMPTZ NOT NULL,
    symbol VARCHAR(50) NOT NULL,
    chain VARCHAR(50),
    price_usd DECIMAL(78, 18),
    source VARCHAR(50),
    PRIMARY KEY (time, symbol, chain)
);
SELECT create_hypertable('prices', 'time');

-- Audit Log
CREATE TABLE audit_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id),
    action VARCHAR(255) NOT NULL,
    resource_type VARCHAR(100),
    resource_id UUID,
    old_value JSONB,
    new_value JSONB,
    ip_address INET,
    user_agent TEXT,
    created_at TIMESTAMP DEFAULT NOW()
);
```

---

## Technology Stack

### Next.js 14 Frontend (Recommended)

```yaml
Next.js 14 Features:
  - App Router (file-based routing)
  - React Server Components (RSC)
  - Server Actions (mutations)
  - Route Handlers (API endpoints)
  - Streaming + Suspense
  - Image Optimization
  - Font Optimization
  - Middleware (auth, redirects)

Project Structure:
  crypto-pocket-butler-web/
  ├── app/                      # App Router
  │   ├── (dashboard)/          # Dashboard layout
  │   │   ├── layout.tsx        # Dashboard layout (sidebar, header)
  │   │   ├── page.tsx          # Dashboard home
  │   │   ├── portfolio/
  │   │   │   ├── page.tsx      # Portfolio view
  │   │   │   └── [id]/page.tsx # Portfolio detail
  │   │   ├── analytics/
  │   │   └── settings/
  │   ├── (auth)/               # Auth layout (centered)
  │   │   ├── login/page.tsx
  │   │   └── register/page.tsx
  │   ├── api/                  # API routes
  │   │   ├── wallets/route.ts
  │   │   ├── portfolio/route.ts
  │   │   └── prices/route.ts
  │   └── layout.tsx            # Root layout
  ├── components/
  │   ├── ui/                   # Base UI components (shadcn)
  │   │   ├── button.tsx
  │   │   ├── card.tsx
  │   │   ├── table.tsx
  │   │   └── ...
  │   ├── dashboard/            # Dashboard-specific components
  │   │   ├── PortfolioCard.tsx
  │   │   ├── AssetTable.tsx
  │   │   └── AllocationChart.tsx
  │   ├── wallet/               # Wallet connection components
  │   │   ├── ConnectModal.tsx
  │   │   └── WalletSelector.tsx
  │   └── charts/               # Chart components
  │       ├── PieChart.tsx
  │       └── LineChart.tsx
  ├── lib/                      # Utilities
  │   ├── api.ts                # API client
  │   ├── utils.ts              # Helper functions
  │   └── validations.ts        # Zod schemas
  ├── hooks/                    # Custom hooks
  │   ├── useWallet.ts
  │   ├── usePortfolio.ts
  │   └── usePrices.ts
  ├── stores/                   # Zustand stores
  │   ├── walletStore.ts
  │   └── portfolioStore.ts
  ├── types/                    # TypeScript types
  │   └── index.ts
  └── public/                   # Static assets

Key Dependencies:
  # Core
  - next@14
  - react@18
  - typescript@5

  # State & Data
  - zustand                 # Global state
  - @tanstack/react-query   # Data fetching

  # UI
  - tailwindcss             # Styling
  - radix-ui                # Primitives
  - @radix-ui/react-*       # Components
  - lucide-react            # Icons
  - class-variance-authority # Component variants
  - clsx + tailwind-merge   # Class utilities

  # Charts
  - recharts                # Charts
  - @visx/visx              # D3 for React

  # Tables
  - @tanstack/react-table   # Tables

  # Forms
  - react-hook-form         # Forms
  - zod                     # Validation
  - @hookform/resolvers     # Zod resolver

  # Wallet
  - @walletconnect/modal    # WalletConnect
  - viem                    # Ethereum client
  - wagmi                   # React hooks for Ethereum

  # Utils
  - date-fns                # Date formatting
  - numeral                 # Number formatting
  - decimal.js              # Precise decimals
```

### Rust Services (Recommended for Critical Paths)

```yaml
Use Rust for:
  - Price ingestion & aggregation service (high-frequency)
  - Portfolio valuation engine (real-time calculations)
  - Wallet indexer (blockchain parsing)
  - Cryptographic operations (key derivation, signing)
  - CLI tools (ops, deployment, debugging)
  - Performance-critical microservices

Key Crates:
  # Web Framework
  - axum / actix-web    # HTTP server
  - tonic / prost       # gRPC
  - tokio               # Async runtime

  # Serialization
  - serde / serde_json  # JSON
  - serde_yaml          # YAML
  - bincode             # Binary

  # Database
  - sqlx                # Async SQL (PostgreSQL)
  - diesel              # ORM
  - redis               # Redis client

  # Blockchain
  - ethers / alloy      # Ethereum
  - solana-sdk          # Solana
  - bip32 / bip39       # HD wallets
  - k256 / p256         # Cryptography

  # HTTP Client
  - reqwest             # HTTP client
  - hyper               # Low-level HTTP

  # Testing
  - proptest            # Property-based testing
  - quickcheck          # QuickCheck
  - criterion           # Benchmarking
```

### Backend (Ortis - Rust Specialist)

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Runtime** | **Rust** | Memory safety, zero-cost abstractions, high performance |
| **Framework** | **Axum / Actix-web** | Type-safe, async, excellent performance |
| **API** | REST + gRPC | REST for external, gRPC for internal services |
| **Database** | PostgreSQL 15 | ACID, JSONB, extensions |
| **Timeseries** | TimescaleDB | PostgreSQL extension, efficient price storage |
| **Cache** | Redis 7 | Sessions, rate limiting, price cache |
| **Message Queue** | Kafka | Event streaming, audit logs |
| **Search** | Elasticsearch | Transaction search, analytics |
| **OLAP** | ClickHouse | Heavy analytics, reporting |

### Frontend (Fe - Next.js Specialist)

| Component | Technology | Rationale |
|-----------|------------|-----------|  
| **Framework** | **Next.js 14** | App Router, SSR, RSC, API routes, optimal performance |
| **Language** | **TypeScript 5** | Type safety, better DX, catch errors early |
| **State** | **Zustand** | Lightweight, simple global state |
| **Data Fetching** | **TanStack Query (React Query)** | Caching, background updates, optimistic updates |
| **Charts** | **Recharts** | Flexible, performant, easy to use |
| **Tables** | **TanStack Table** | Virtualized, sortable, filterable |
| **Forms** | **React Hook Form + Zod** | Performance, type-safe validation |
| **Styling** | **Pure Tailwind CSS** | Full control, no UI library dependencies, custom components |
| **Wallet** | **wagmi + viem** | React hooks for Ethereum, lightweight client |
| **Mobile** | **Responsive Next.js PWA** | Mobile-first, offline support |

### Infrastructure

| Component | Technology | Rationale |
|-----------|------------|-----------|
| **Container** | Docker + Kubernetes | Portability, orchestration |
| **CI/CD** | GitHub Actions | Integrated with repo |
| **Cloud** | AWS / GCP | Managed services, scalability |
| **CDN** | Cloudflare | Edge caching, DDoS protection |
| **Monitoring** | Prometheus + Grafana | Metrics, alerting |
| **Logging** | Loki + ELK | Log aggregation |
| **Tracing** | Jaeger | Distributed tracing |

---

## Security Architecture

### Authentication Flow

```
┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  Client  │     │   API    │     │   Auth   │     │   User   │
│          │     │ Gateway  │     │ Service  │     │   DB     │
└────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
     │                │                │                │
     │  POST /login   │                │                │
     │───────────────>│                │                │
     │                │  Forward       │                │
     │                │───────────────>│                │
     │                │                │  Query user    │
     │                │                │───────────────>│
     │                │                │<───────────────│
     │                │                │                │
     │                │                │ Generate JWT   │
     │                │                │                │
     │                │  JWT + Refresh │                │
     │                │<───────────────│                │
     │  JWT + Refresh │                │                │
     │<───────────────│                │                │
     │                │                │                │
```

### Security Measures

| Layer | Measure | Implementation |
|-------|---------|----------------|
| **Network** | DDoS Protection | Cloudflare |
| **Transport** | TLS 1.3 | Let's Encrypt |
| **API** | Rate Limiting | Redis-based, per-user |
| **Auth** | JWT + Refresh | Short-lived access tokens |
| **Auth** | 2FA | TOTP, WebAuthn |
| **Data** | Encryption at Rest | AES-256 |
| **Data** | Encryption in Transit | TLS |
| **Wallet** | Read-Only Access | No private key storage |
| **Audit** | All Actions Logged | Immutable audit trail |

---

## Scalability Strategy

### Horizontal Scaling

```
Service          | Initial | Scale Trigger    | Max Instances
-----------------|---------|------------------|---------------
API Gateway      | 2       | CPU > 70%        | 10
Wallet Service   | 2       | CPU > 70%        | 10
Portfolio Svc    | 2       | CPU > 70%        | 10
Pricing Service  | 3       | Latency > 100ms  | 20
Transaction Svc  | 2       | Queue depth >100 | 10
Analytics Svc    | 2       | CPU > 70%        | 5
```

### Caching Strategy

| Data Type | Cache TTL | Invalidation |
|-----------|-----------|--------------|
| Token Prices (top 100) | 30s | Time-based |
| Token Prices (mid) | 2min | Time-based |
| Portfolio Value | 1min | On transaction |
| User Sessions | 24h | On logout |
| Transaction History | 5min | On new tx |

---

## Deployment Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         Cloud Provider                           │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐ │
│  │                    Kubernetes Cluster                      │ │
│  │                                                            │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐       │ │
│  │  │   Ingress   │  │   Ingress   │  │   Ingress   │       │ │
│  │  │  Controller │  │  Controller │  │  Controller │       │ │
│  │  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘       │ │
│  │         │                │                │               │ │
│  │  ┌──────┴────────────────┴────────────────┴──────┐       │ │
│  │  │              Service Mesh (Istio)              │       │ │
│  │  └──────┬────────────────┬────────────────┬──────┘       │ │
│  │         │                │                │               │ │
│  │  ┌──────┴───────┐ ┌──────┴───────┐ ┌──────┴───────┐     │ │
│  │  │   Backend    │ │   Backend    │ │   Backend    │     │ │
│  │  │   Services   │ │   Services   │ │   Services   │     │ │
│  │  │   (Pods)     │ │   (Pods)     │ │   (Pods)     │     │ │
│  │  └──────────────┘ └──────────────┘ └──────────────┘     │ │
│  │                                                            │ │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐   │ │
│  │  │  PostgreSQL  │  │    Redis     │  │    Kafka     │   │ │
│  │  │  (Stateful)  │  │  (Stateful)  │  │  (Stateful)  │   │ │
│  │  └──────────────┘  └──────────────┘  └──────────────┘   │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                  │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │
│  │   S3 / GCS   │  │  Cloudflare  │  │  Managed     │         │
│  │  (Files)     │  │    (CDN)     │  │  Monitoring  │         │
│  └──────────────┘  └──────────────┘  └──────────────┘         │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## API Design

### REST Conventions

```yaml
Versioning: /api/v1/{resource}
Format: JSON
Auth: Bearer JWT

Response Envelope:
  success: boolean
  data: object | array
  error: object (optional)
  pagination: object (optional)

Example Response:
{
  "success": true,
  "data": {
    "id": "uuid",
    "name": "Portfolio 1",
    "totalValue": 125430.50
  },
  "pagination": {
    "page": 1,
    "limit": 20,
    "total": 150
  }
}
```

### Error Codes

| Code | Meaning |
|------|---------|
| 400 | Bad Request - Invalid input |
| 401 | Unauthorized - Invalid/missing token |
| 403 | Forbidden - Insufficient permissions |
| 404 | Not Found |
| 409 | Conflict - Resource already exists |
| 429 | Too Many Requests - Rate limited |
| 500 | Internal Server Error |
| 503 | Service Unavailable |

---

## Monitoring & Observability

### Metrics (Prometheus)

```yaml
API Metrics:
  - http_requests_total{method, endpoint, status}
  - http_request_duration_seconds{method, endpoint}
  - active_connections

Business Metrics:
  - portfolios_total
  - wallets_connected_total
  - transactions_indexed_total
  - valuation_errors_total

System Metrics:
  - cpu_usage_percent
  - memory_usage_bytes
  - disk_usage_bytes
  - network_bytes_total
```

### Alerts

| Alert | Condition | Severity |
|-------|-----------|----------|
| API Latency | p99 > 500ms for 5min | Warning |
| API Errors | Error rate > 1% for 5min | Critical |
| Price Stale | No price update > 10min | Warning |
| DB Connections | > 80% pool used | Warning |
| Disk Usage | > 80% used | Warning |

---

## Disaster Recovery

| Scenario | RTO | RPO | Strategy |
|----------|-----|-----|----------|
| Single pod failure | 0 | 0 | Auto-restart |
| Node failure | 30s | 0 | Pod rescheduling |
| Zone failure | 2min | 5min | Multi-zone |
| Region failure | 15min | 1hour | Multi-region |
| Data corruption | 1hour | 24hour | Point-in-time recovery |

### Backup Strategy

```yaml
Database:
  - Continuous WAL archiving
  - Daily full backups
  - Retention: 30 days
  - Tested monthly

Files:
  - S3 versioning enabled
  - Cross-region replication
  - Lifecycle policies

Configuration:
  - Git-based (Infrastructure as Code)
  - Secrets in Vault/Secrets Manager
```
