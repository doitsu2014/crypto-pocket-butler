# Software Architecture — crypto-pocket-butler

**Version:** 2.0 (system refactor)
**Date:** 2026-04-27
**Status:** Draft — outcome of BMAD party-mode roundtable (Sam, Alex, Ortis, Fe, Quinn, Pat)
**Supersedes:** `software-architecture-v1.md` (v2.0-revised, 2026-04-26)

> **Reading guide:** v1 was a 2022-shaped Rust DDD monolith blueprint with 2026 version numbers painted on top. This document refactors the *shape*, not the stack. The biggest single change is a **scope honesty cut**: v1 ships as **non-custodial read-only analytics + rebalance suggestions** instead of a custodial signing service. The custodial ambitions are not deleted — they are sequenced as v2 (managed-custody) and v3 (full custodial) escalations behind clean architectural seams.

---

## 1. Roadmap & Scope Framing

The original v1 brief said: *async background rebalancing, actual on-chain execution, custodial wallets, horizontal scale, real PWA.* Custodial + on-chain execution + no jurisdiction/license/insurance answer = unlicensed money transmission in most jurisdictions. The team rejects that scope for v1.

| Phase | Custody model | What ships | What unlocks the next phase |
|---|---|---|---|
| **v1 — now** | **Non-custodial.** App reads + analyzes. User signs in their own wallet (or no signing at all in the v1 cut). | Read-only multi-wallet portfolio across **Ethereum L1, Base, Arbitrum**. Drift report. Rebalance suggestions (text only). PWA. Solana/BTC port stubs. | Real users, validated demand. |
| **v2 — conditional** | **Managed custody via Turnkey / Privy / DFNS.** They hold keys under their license + insurance. We become a delegated operator. | EIP-712 signed intents. Simulate-before-broadcast. MEV-protected venues (Flashbots Protect, CoW, Across). On-chain execution path. | Traction + a written legal review of *our* MSB/VASP exposure. |
| **v3 — far** | **Full custodial.** | In-house signer service, KMS/HSM, Safe 2-of-3, hash-chained audit, OFAC screening at policy gate, kill-switches, dual-control. | MTL/MiCA license, crime+custody insurance, ops headcount. |

**v1's job is to prove the product, not the custody.** Architectural seams are placed so the v2 escalation is *additive*, not a rewrite.

> **Open product questions** (block v1 implementation start):
> 1. Is v1 for your own funds, friends-and-family (closed list), or public signup?
> 2. Anyone else using this in the next 90 days? If yes, who and how many?
> 3. Monetization in v1 — none / subscription / fees on AUM? (Fees on AUM moves us to v3 immediately.)

---

## 2. Technology Stack (refactored)

| Layer | Technology | Change vs v1 doc |
|---|---|---|
| **Frontend** | Next.js 16 (App Router, RSC, **Server Actions**, **Partial Prerendering**), TypeScript, **Tailwind v4 + @theme tokens**, shadcn/ui, Ladle, **TanStack Query v5**, **Reown AppKit + wagmi v2 + viem v2** (read-only wallet connect in v1) | New: Server Actions, PPR, design system, wallet libs, type-safe API client |
| **API contract** | OpenAPI generated from Axum via **`utoipa`** → consumed by `openapi-typescript` + `openapi-fetch` | New: replaces hand-written types |
| **API runtime** | Rust + **Axum 0.8**, `tower` middleware composition | Same Axum; tower stack made explicit |
| **Persistence** | PostgreSQL 16 + **`sqlx` with `query!` macros** | **Replaces SeaORM** — compile-time-checked SQL, no entity codegen drift |
| **Migrations** | `sqlx-cli` or `refinery` | New |
| **Job/event runtime** | `sqlx::postgres::PgListener` (LISTEN/NOTIFY) + transactional outbox; **`apalis` + `apalis-sql`** for true wall-clock jobs only | **Replaces apalis-cron-as-heartbeat** with event-driven sync |
| **In-process cache** | `moka` (request-scoped, ≤1s TTL) | Demoted from "architectural layer" to "implementation detail" |
| **External cache (seam)** | Redis-ready via **`fred`** crate. Not deployed in v1; seam exists for v2 horizontal scale. | New |
| **Config** | **`figment`** layered (env → TOML → defaults), validated at startup | **Replaces `dotenvy`** in prod paths |
| **Secrets** | `secrecy::SecretString`, `zeroize::Zeroizing` for any sensitive material | New |
| **Errors** | `thiserror` at every crate boundary; `anyhow` permitted only in `bin/` | New |
| **Observability** | `tracing` + `opentelemetry_otlp` + `tracing-opentelemetry` (OTLP/gRPC), **with PII scrubbing at SDK layer** | New: OTel day-one is non-negotiable |
| **EVM SDK** | **`alloy`** (signers, providers, consensus). `ethers-rs` is archived — do not ship. | Was unspecified |
| **EVM indexer (read-side)** | **Subsquid** squid per chain (Base, Arbitrum, Ethereum) — replaces RPC-poll | Was raw RPC + 15-min cron |
| **Price feed** | **CoinGecko** (UI sparklines, valuation) for v1. Pyth pull-oracle + Chainlink + on-chain DEX TWAPs reserved for v2 execution-relevant pricing. | Same v1; v2 path documented |
| **Auth (web → api)** | API key in **server-side env only** (RSC + Server Actions proxy). NextAuth v5 session cookies (httpOnly, refresh rotation, PKCE) for any user concept. | **Hard fix**: API key must never reach the browser |
| **Auth (v2 future)** | Keycloak OIDC (JWT validated locally via JWKS) | Same as v1 doc |
| **Deployment** | Docker Compose for dev. Single `api` binary + `indexer` worker for v1 prod. | Notifier deploy removed until 2nd consumer exists |

---

## 3. High-Level Architecture (v1, Path A)

```
┌──────────────────────────────────────────────────────────────────┐
│                          CLIENTS                                  │
│   [PWA — Next.js 16 RSC + Service Worker + Reown AppKit]          │
└───────────────┬──────────────────────────────────────────────────┘
                │ HTTPS (Server Actions / SSE / openapi-fetch)
┌───────────────▼──────────────────────────────────────────────────┐
│                  WEB TIER (Next.js)                               │
│   RSC fetches server-side (API key in process.env, never sent).   │
│   Server Actions for mutations. PPR for dashboard.                │
└───────────────┬──────────────────────────────────────────────────┘
                │ openapi-fetch (typed)
┌───────────────▼──────────────────────────────────────────────────┐
│                       API (single Rust binary)                    │
│  ┌───────────┐  ┌─────────────┐  ┌────────────┐  ┌────────────┐  │
│  │ transport │  │ application │  │  domain    │  │  adapters  │  │
│  │ (axum)    │  │ (use cases) │  │  (pure)    │  │ (chain,    │  │
│  │           │  │             │  │            │  │  db, cache)│  │
│  └───────────┘  └─────────────┘  └────────────┘  └────────────┘  │
└──┬──────────────────────────┬─────────────────────┬──────────────┘
   │                          │                     │
   │ outbox + LISTEN/NOTIFY   │ ports               │
┌──▼─────────────┐  ┌─────────▼─────────┐  ┌────────▼─────────────┐
│   indexer      │  │   PostgreSQL 16    │  │  External services    │
│   worker       │  │   (primary + RR)   │  │  - Subsquid squids    │
│ (Subsquid sub, │  │   - write side     │  │  - CoinGecko price    │
│  projections)  │  │   - read models    │  │  - EVM RPCs (read)    │
└────────────────┘  └────────────────────┘  └───────────────────────┘
```

**Two deployables in v1:** `api` (Rust binary) and `indexer` (worker). One Postgres. One Next.js app. That's it.

---

## 4. Architectural Style

### 4.1 Hexagonal layering (replaces classical-DDD `infrastructure/`)

```
api/
  crates/
    domain/              # pure, no tokio/axum/sqlx — fast unit tests
      portfolio/
      asset/
      account/
    application/         # use cases, ports defined here
      portfolio/
      asset/
    adapters/            # inbound + outbound — implements ports
      transport-http/    # axum handlers, openapi schema (utoipa)
      persistence-pg/    # sqlx repos
      cache-moka/        # in-process micro cache
      cache-redis/       # fred-backed (feature-gated, off in v1)
      price-coingecko/
      indexer-subsquid/  # outbound subscriber + projections
    chain-port/          # use-case traits (see §6)
    chain-evm/
    chain-solana/        # stubs in v1
    chain-bitcoin/       # read-only stub in v1
    chain-router/        # (network, account) -> &dyn ChainAdapter
    signer/              # NOT BUILT IN v1. Reserved for v2 (Path B).
  bin/
    api/                 # axum HTTP server, the v1 deployable
    indexer/             # Subsquid consumer + projection writer
```

The application core depends only on `domain/` and `chain-port/`. Adapters depend on the application core. **Chain types never leak into the domain or use cases.**

### 4.2 Modular monolith — single binary in v1

`v1` ships **one Rust binary** (`api`) plus an `indexer` worker. The earlier round's `executor` and `signer` deployables are **deferred to v2** when custodial returns. This is Round 1's "boring is safe" instinct, vindicated by the simplest-scope decision.

### 4.3 Event-driven core (not cron-driven)

- **Outbox pattern**: every domain state change writes a row to `outbox` in the same transaction. The `indexer` worker (and any future `executor`) consumes from outbox via `SELECT … FOR UPDATE SKIP LOCKED`.
- **`PgListener` (Postgres LISTEN/NOTIFY)**: triggers near-real-time fan-out in-process; survives restarts because outbox is the durable source of truth.
- **`apalis` + `apalis-sql`**: only for true wall-clock jobs (daily snapshots, weekly drift reports), not as a 15-min heartbeat.

### 4.4 CQRS-lite for the read side

The portfolio dashboard is 95% reads with cross-chain aggregation. Write side = domain aggregates with strict invariants. Read side = denormalized projections in Postgres, rebuilt from outbox events. No event sourcing of the entire domain — only the (eventual v2) execution state machine warrants that.

### 4.5 Errors

- `thiserror` at every crate boundary; one `ApiError` enum with `IntoResponse`.
- `anyhow` is banned in library crates; allowed only in `bin/`.

---

## 5. Bounded Contexts (refined)

v1's five contexts collapse to a leaner shape:

| Context | Responsibility | Aggregates / Entities |
|---|---|---|
| **Portfolio** | Holdings, target allocations, drift, rebalance suggestions | `Portfolio` (root), `Holding`, `AllocationTarget`, `DriftReport`, `RebalanceSuggestion` |
| **Asset / Pricing** | Asset identity, metadata, USD valuation | `Asset`, `AssetIdentity (network, address)`, `Price`, `Symbol` |
| **Account / Identity** | User accounts, watched addresses, preferences | `Account`, `WatchedAddress`, `UserPreferences` |
| **(removed) Allocation** | — | Folded into `Portfolio` aggregate as `AllocationTarget` value object |
| **(removed) Chain** | — | Pushed entirely below the application boundary into `chain-*` adapters; no domain entities |

Rationale (Sam, Round 1): `Allocation` was a noun on a whiteboard, not a context with its own language. `Chain` is infrastructure, not domain.

---

## 6. Chain Abstraction — Per-VM Ports

Alex's stake (Round 3): there is **no honest single `ChainAdapter` trait** across EVM, Solana, and Bitcoin. Account-based vs UTXO, `secp256k1` vs `ed25519` vs `ecdsa-secp256k1+SegWit/Taproot`, probabilistic vs slot vs 2-epoch finality. Anyone who draws one trait per chain ships a leaky abstraction.

**The seam is per-use-case ports**, with per-VM adapter crates implementing them.

```rust
// crates/chain-port/src/lib.rs
#[async_trait]
pub trait ReadBalances {
    async fn balances(&self, account: &Account) -> Result<Vec<Balance>, ChainError>;
}

#[async_trait]
pub trait ReadHistory { /* ... */ }

#[async_trait]
pub trait EstimateFee { /* ... */ }                  // v2-relevant
#[async_trait]
pub trait BroadcastSigned { /* ... */ }              // v2-relevant
#[async_trait]
pub trait WatchInclusion { /* ... */ }               // v2-relevant
#[async_trait]
pub trait WatchFinality { /* ... */ }                // v2-relevant
```

| Crate | v1 status | v1 stack | v2+ stack |
|---|---|---|---|
| `chain-port` | **real** — port definitions only | `async-trait` | unchanged |
| `chain-evm` | **real** — Base, Arbitrum, Ethereum L1 (read ports) | `alloy`, Subsquid | + Pyth, Chainlink, CoW, Flashbots Protect, Across, MEV Blocker (write ports) |
| `chain-solana` | **stubs** returning `Unsupported` (port-conformance-tested) | scaffolding only | `solana-sdk`, `solana-client`, Helius/Triton RPC, Helius/Shyft indexer, Pyth-Solana, Jupiter, Wormhole/DeBridge, Squads (if multisig) |
| `chain-bitcoin` | **read-only placeholder** — address watching only | `bitcoincore-rpc` + `mempool.space` / Esplora HTTP API | unchanged for v2 (no DeFi semantics on BTC) |
| `chain-router` | **real** — selects adapter from `(network, account)` | enum dispatch | + dynamic provider failover |

**The application core never names a chain.** It calls `router.read_balances(account)` and the router selects the right adapter. When Solana lands in v2, no use case changes — only the adapter does.

> **Why `enum_dispatch` over `Box<dyn Trait>`** (Ortis, Round 1): EVM/Solana/Bitcoin are a closed set. Static dispatch via an enum is exhaustive-match-checked and avoids vtable-per-call cost. Use `dyn` only at the routing boundary.

### 6.1 Bitcoin — be honest with the user

Bitcoin is **not DeFi**. UTXO model, no smart contracts (BitVM/Babylon are research). v1 + v2 = "we track and display your BTC." Swaps require a bridge (tBTC, Across, ThorChain) and live in v3+ scope.

### 6.2 EVM finality gates (constants)

| Chain | "Settled" depth | "Finalized" depth |
|---|---|---|
| Ethereum L1 | 12 blocks | 2 epochs (~13 min, Casper FFG) |
| Base | n/a (immediate at sequencer) | 64 L2 blocks (post batch-poster) |
| Arbitrum | n/a | 64 L2 blocks (post batch-poster) |

Read projections key off **Finalized**, not Settled. (v2 execution will use Settled-then-finality-gated for UX.)

---

## 7. API Surface (v1)

Base URL: `/api/v1`. OpenAPI spec served at `/api/v1/openapi.json` via `utoipa`.

| Endpoint | Method | Description |
|---|---|---|
| `/api/v1/accounts/me` | GET | Current account profile |
| `/api/v1/watched-addresses` | GET, POST, DELETE | Add/remove watched wallet addresses |
| `/api/v1/portfolios` | GET, POST | List/create portfolios (a portfolio = bag of watched addresses + target allocations) |
| `/api/v1/portfolios/{id}` | GET, PUT, DELETE | Portfolio CRUD |
| `/api/v1/portfolios/{id}/holdings` | GET | Aggregated holdings, USD-valued |
| `/api/v1/portfolios/{id}/drift` | GET | Drift report (current vs target) |
| `/api/v1/portfolios/{id}/suggestions` | GET | Rebalance suggestions (text only — no execution links in v1) |
| `/api/v1/portfolios/{id}/snapshots` | GET | Historical snapshots |
| `/api/v1/portfolios/{id}/export.csv` | GET | CSV export |
| `/api/v1/assets` | GET | Tracked assets (read-through to indexer + CoinGecko) |
| `/api/v1/sse/portfolios/{id}` | GET (SSE) | Real-time push for balance/price updates |

**Mutations from the PWA go through Server Actions**, not direct fetches. Server Actions add the API key from `process.env` server-side and forward to Axum over loopback or private network. The browser never sees the key.

---

## 8. Frontend Architecture

### 8.1 App Router structure

```
web/app/
├── (marketing)/           # public, static
├── (auth)/                # NextAuth v5 (when user accounts land)
├── (dashboard)/           # PPR-enabled
│   ├── layout.tsx         # static shell
│   ├── page.tsx           # PortfolioOverview (Suspense boundaries)
│   ├── portfolios/[id]/
│   │   ├── page.tsx
│   │   ├── holdings/
│   │   ├── drift/
│   │   └── suggestions/
│   ├── wallets/
│   └── settings/
├── api/                   # route handlers (used by PWA service worker only)
└── layout.tsx
```

### 8.2 Component & data strategy

| Concern | Choice |
|---|---|
| Data fetching (initial) | **RSC** with server-side `openapi-fetch` (API key in `process.env`) |
| Data fetching (client) | **TanStack Query v5** with hydration from RSC |
| Mutations | **Server Actions** with `useActionState` + `useOptimistic` |
| Real-time | **SSE** from API → `queryClient.setQueryData` to update cache |
| Wallet connect (v1) | **Reown AppKit + wagmi v2 + viem v2** — read-only, address paste also supported |
| Type safety | **`openapi-typescript` + `openapi-fetch`** generated from Axum's `utoipa` schema in CI |
| Styling | **Tailwind v4** with `@theme` CSS variables → design tokens |
| UI primitives | **shadcn/ui** as the base library |
| Component dev | **Ladle** (Vite-native, faster than Storybook) |
| A11y | **Zero axe violations in CI**, WCAG 2.2 AA budget |
| Performance budget | LCP < 1.2s, INP < 200ms, CLS < 0.05 (PPR-shell helps) |

### 8.3 PWA (real, not a diagram lie)

- **Service Worker**: caches static shell + last-known portfolio snapshot; serves stale on offline.
- **Hydration on reconnect**: client calls `GET /api/v1/portfolios/{id}/snapshots?since=<cursor>` to replay missed events; cursor = last-seen `outbox.sequence_id`.
- **Push (v2+)**: Web Push API for drift alerts; out of v1 scope.
- **Install prompt**: standard `beforeinstallprompt`.

### 8.4 Auth & secret hygiene

- v1: API key lives only in `process.env` on the Next.js server. RSC fetches and Server Actions attach it. **Client Components never receive it.**
- v2 future: NextAuth v5 with Keycloak provider, httpOnly session cookies, refresh-token rotation, PKCE.

---

## 9. Data Flow

### 9.1 Portfolio read path (the hot path)

```
1. PWA opens /(dashboard)/portfolios/[id]
2. RSC calls GET /api/v1/portfolios/{id}/holdings (server-side, with API key)
3. Axum handler → application/usecases/get_holdings
4. Use case reads denormalized read model from Postgres (built by indexer)
5. Use case calls price-coingecko adapter for USD valuation (Moka-cached, ≤60s TTL)
6. Returns aggregated, valued holdings
7. RSC streams via Suspense boundaries; PPR keeps shell static
8. Client hydrates TanStack Query cache; SSE keeps it live
```

### 9.2 Portfolio sync path (event-driven, not cron-driven)

```
1. Subsquid squid emits ERC-20 Transfer / NFT events for watched addresses
2. indexer worker consumes from squid HTTP/GraphQL endpoint
3. For each event:
   3a. Insert raw event row (idempotency key = (network, txhash, log_index))
   3b. Update read-side `holdings_projection` table
   3c. Insert outbox row: { event: BalanceChanged, portfolio_id, ... }
   3d. NOTIFY portfolio_changed (LISTEN/NOTIFY)
4. api process listens; pushes SSE message to subscribed clients
5. Drift recomputed lazily on next /drift read, or eagerly in v2
```

**No 15-min cron heartbeat in v1.** A daily snapshot job (apalis-sql wall-clock) writes `holdings_snapshot` rows for history/CSV export.

### 9.3 Drift detection

```
1. User PUTs target allocations on /api/v1/portfolios/{id}
2. Use case validates, persists, writes outbox event AllocationsChanged
3. GET /api/v1/portfolios/{id}/drift recomputes:
   drift_pct[asset] = (current_pct[asset] - target_pct[asset])
4. Returns DriftReport; flagged entries where |drift| > threshold (default 5%)
```

### 9.4 Rebalance suggestions (v1 = text only)

Pure analytical output. No links auto-fire. No EIP-712. No signing.

```
"To return to target allocation:
 - Sell ~0.32 ETH on Base ($1,250 USD)
 - Buy USDC on Base
 - No cross-chain action required."
```

v2 turns these into intent payloads the user (or Turnkey) can sign.

### 9.5 Cache invalidation

| Data | Cache | TTL | Invalidation |
|---|---|---|---|
| Asset metadata | Moka | 1 hour | TTL only |
| Holdings projection | Postgres (source) | n/a | Indexer event-driven |
| USD price | Moka | 60 sec | TTL only |
| Drift report | Not cached (cheap to compute) | — | — |

---

## 10. Cross-cutting

### 10.1 Observability (day-one, non-negotiable)

- **Tracing**: `tracing` + `tracing-subscriber` + `tracing-opentelemetry` + `opentelemetry_otlp` (OTLP/gRPC export to a collector).
- **Spans**: HTTP request → use case → adapter → DB query / external call.
- **Scrubbing**: a custom `tracing` layer redacts addresses, amounts, and any user PII from span attributes before export. **An unsanitized OTLP exporter is an exfil channel** (Quinn).
- **Request IDs**: `tower-http::request_id::SetRequestIdLayer` + propagated across SSE.
- **Logs**: structured JSON via `tracing-subscriber::fmt` with the OTel layer attached.
- **Metrics**: minimal in v1 — request rate, latency histograms, indexer lag.

### 10.2 Tower middleware stack (composed once)

```rust
let app = Router::new()
    .merge(api_v1_routes())
    .layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
            .layer(TraceLayer::new_for_http())
            .layer(GovernorLayer { config: governor_conf }) // tower_governor
            .layer(CorsLayer::permissive())
            .layer(api_key_extractor()) // FromRequestParts, not hand-rolled middleware
    );
```

### 10.3 Configuration

`figment` layered: defaults (compile-time) ← `config.toml` ← `config.{ENV}.toml` ← env vars (`APP__SECTION__KEY=…`). Validated at startup with `serde` + `validator`. **No `dotenvy` in prod paths** — `.env` is for `cargo run` only.

### 10.4 Errors (boundary-typed)

```rust
// api/src/error.rs
#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("not found")] NotFound,
    #[error("validation: {0}")] Validation(String),
    #[error("chain: {0}")] Chain(#[from] ChainError),
    #[error("db")] Db(#[from] sqlx::Error),
    #[error("internal")] Internal,
}

impl IntoResponse for ApiError { /* ... */ }
```

`anyhow` permitted only in `bin/main.rs` for startup orchestration.

---

## 11. Security Model (Path A, v1)

### 11.1 Threat model in scope

We are **not custodial in v1**. The threat model is: a portfolio analytics service holding (a) user account data, (b) watched public addresses, (c) target allocations. Not keys, not signed transactions, not user funds.

### 11.2 Controls

| Concern | Control |
|---|---|
| Transport | TLS 1.3 everywhere; HSTS |
| API key (web → api) | `process.env` only, never serialized to the browser; rotated via env redeploy; one key per deployment, not per user |
| User auth (when introduced) | NextAuth v5, httpOnly session cookies, PKCE; future Keycloak migration ready |
| Database | Encryption at rest; per-service Postgres role with least-privilege grants |
| Secrets in code | `secrecy::SecretString` for any credential touched at runtime; never in `Debug` output |
| Logging | OTel + structured logs, **with PII scrubbing on span attributes** |
| Audit log | `portfolio_audit_log` Postgres table (append-only via constraint + role grants) — **hash-chaining deferred to v2** when funds move |
| Dependency hygiene | `cargo audit` in CI; pinned `Cargo.lock`; quarterly third-party review |
| Public exposure | Only the Next.js web tier is internet-facing in v1; api can be private-network-only behind the web tier |

### 11.3 Minimum security floor that survives any future path (Quinn)

These are non-negotiable now and into v2/v3:

- EIP-712 typed intents with chain-id binding (when execution lands)
- Simulate-before-broadcast (Tenderly or local Anvil fork) before any signed tx (when execution lands)
- MEV-protected RPCs only (Flashbots Protect / MEV Blocker) — never public mempool
- OFAC screening at intent creation via Chainalysis Address Screener or TRM Labs SaaS — **not in-house** (v2)
- Hash-chained audit log when funds move (v2)
- OTel exporter PII scrubbing (now)
- Secrets via `figment` (now), never `dotenvy` in prod (now)

### 11.4 What's red-carded at PR review

- Private keys in process memory
- Intents without EIP-712 or chain-id binding
- Any broadcast path without simulation
- Unsanitized OTLP exporter
- Public signup without KYC under any custodial variant
- Fee collection on others' funds without a documented licensing memo

---

## 12. Scalability — Architected For, Not Deployed

The user said yes to horizontal scale (Round 2), and "keep simplest" (Round 3). v1 ships a single fat node; v2 escalates only when traction earns it. Architectural seams that make scale-out a config change, not a rewrite:

| Seam | v1 default | v2 escalation |
|---|---|---|
| Cache | `moka` in-process (≤1s TTL) | `fred` Redis client (already a feature flag) |
| WS / SSE fan-out | In-process `tokio::sync::broadcast` | Redis pub/sub channel `user:{id}` |
| Job leadership | Single executor (no leader election needed) | `pg_advisory_lock(0xC0DE)` for executor leadership |
| Stickiness | Single node, no LB | Sticky-session LB or stateless re-subscribe via cursor |
| DB reads | Primary | Read replicas for projections |

> **The "API is stateless" claim from v1 doc** (Ortis flagged this) is honest *only* once the cache is external, the executor is leader-elected, the signer (when introduced) is single-active, and WS fans out via Redis. v2 will earn the claim. v1 doesn't need it.

---

## 13. Deployment

### 13.1 v1 Docker Compose (dev + small prod)

```yaml
services:
  postgres:
    image: postgres:16
    volumes: [pgdata:/var/lib/postgresql/data]

  api:
    build: ./api
    depends_on: [postgres]
    environment:
      - APP__DATABASE_URL=postgres://...
      - APP__API_KEY=<secret>
      - APP__OTEL_ENDPOINT=http://otel-collector:4317

  indexer:
    build: ./api
    command: /bin/indexer
    depends_on: [postgres]

  web:
    build: ./web
    depends_on: [api]
    environment:
      - API_BASE_URL=http://api:8080
      - API_KEY=<secret>          # server-side env only
      - NEXTAUTH_SECRET=<secret>

  otel-collector:
    image: otel/opentelemetry-collector-contrib
    # exports to your backend of choice (Tempo, Honeycomb, Datadog, ...)
```

### 13.2 v2 escalation (Path B managed-custody)

Add one binary: `executor` (consumes outbox, calls Turnkey/Privy/DFNS SDK to sign, broadcasts via MEV-protected RPC, watches inclusion + finality, emits settlement events back to outbox). No HSM. No key ceremony. The custody license stays with the SaaS.

```
+ executor:
+   build: ./api
+   command: /bin/executor
+   depends_on: [postgres]
+   environment:
+     - APP__TURNKEY_ORG_ID=...
+     - APP__TURNKEY_API_KEY=...
```

### 13.3 v3 escalation (full custodial)

Out of scope. Requires legal counsel, MTL/MiCA license, crime+custody insurance, ops headcount, separate VPC subnet, KMS/HSM, dedicated `signer` binary, Safe 2-of-3 multisig, key ceremony, dual-control, OFAC at policy gate, hash-chained audit, kill-switch.

---

## 14. Architectural Decisions (ADRs)

| # | Decision | Rationale | Trade-off |
|---|---|---|---|
| **AD-001** | Hexagonal layering (`domain` / `application` / `adapters`); domain has zero infra deps | Pure-domain unit tests in milliseconds; adapter swap doesn't touch use cases | Slightly more crates than classical DDD |
| **AD-002** | Modular monolith — single `api` binary in v1 | Lowest ops cost; matches "simplest" scope | Must externalize state seams to scale out (done) |
| **AD-003** | `sqlx` + `query!` macros over SeaORM | Compile-time-checked SQL against real Postgres in CI; no entity codegen drift | Manual migrations; less ergonomic for dynamic queries |
| **AD-004** | Subsquid indexer per chain, no RPC-poll heartbeat | Event-driven sync; no 15-min cron debt; per-chain cost control | New external dependency (squid hosting) |
| **AD-005** | Per-VM adapter crates behind use-case ports; app core never names a chain | Honest about EVM/Solana/Bitcoin differences; v2 Solana = adapter swap | Boilerplate per VM; stub-but-conformance-tested for v1 |
| **AD-006** | Outbox + Postgres LISTEN/NOTIFY for events; `apalis-sql` only for wall-clock jobs | Single source of truth (DB row), no Kafka, no dual-write | Throughput ceiling around mid-thousands events/sec — ample for 18 months |
| **AD-007** | OpenTelemetry from day one with PII scrubbing | Debugging multi-chain flows requires distributed traces; scrubbing prevents OTLP-as-exfil | One more layer in the tracing stack |
| **AD-008** | `figment` + `secrecy` + `zeroize`; ban `dotenvy` in prod | Layered config, secrets never in `Debug`, zeroize-on-drop | Dev workflow needs `cargo run --features dev-dotenv` shim |
| **AD-009** | Server Actions + RSC; API key in `process.env` server-side only | Browser never receives the key; mutations are typed end-to-end | Server Actions are App Router-only — already our Next.js choice |
| **AD-010** | `openapi-typescript` + `openapi-fetch` generated from `utoipa` schema in CI | End-to-end type safety; no hand-written TS types | Adds a CI step; schema must stay in sync (CI gate) |
| **AD-011** | Reown AppKit + wagmi v2 + viem v2 for read-only wallet connect in v1 | Proven, accessible, supports the 50+ wallets users actually have | Lock-in to Reown's WC infrastructure |
| **AD-012** | **v1 is non-custodial.** Custodial deferred to v2 (Turnkey/Privy SDK) and v3 (in-house signer + license) | "Custodial without a license" is unlicensed money transmission; "simplest" forces non-custodial | v1 cannot rebalance for the user — it can only suggest |
| **AD-013** | Bitcoin = read-only forever in this app; "rebalance" verb does not apply | UTXO model + no smart contracts means BTC swaps require a bridge — that's a different product | Users with mostly-BTC portfolios get partial value |
| **AD-014** | Single `api` binary v1; `executor` added in v2 only when custodial returns | YAGNI; deploys earn complexity | Round 2 over-designed for the wrong scope; explicitly amended here |

---

## 15. What Was Removed (and Why)

| Removed from v1 | Status | Why |
|---|---|---|
| Custodial signer service | **Deferred to v2** (Path B Turnkey) and v3 (in-house) | No license, no insurance, no answer to "what jurisdiction" |
| KMS / HSM / Safe 2-of-3 / key ceremony | **Deferred to v3** | Out of scope without licensing path |
| On-chain execution (broadcast/sign path) | **Deferred to v2** | Requires custody, even if managed |
| OFAC screening in-house | **Deferred to v2** as SaaS (Chainalysis / TRM) | Not in-house at any phase |
| Hash-chained audit ledger | **Deferred to v2** | Plain append-only Postgres table is sufficient when funds aren't moving |
| Horizontal scale rollout | **Architected for, not deployed** | Single node + replicas suffices for v1; seams exist |
| MEV-aware multi-venue routing | **Deferred to v2** | No execution in v1 |
| 15-min apalis-cron heartbeat | **Removed** | Replaced by event-driven indexer + outbox |
| SeaORM | **Removed** | Replaced by sqlx + compile-time SQL |
| `dotenvy` in prod paths | **Removed** | Replaced by `figment` |
| `Box<dyn ChainRpc>` HashMap router | **Removed** | Replaced by `enum_dispatch` chain-router |
| Notion sync | Already removed in v1 doc | Out of scope |
| Keycloak in v1 | Already deferred in v1 doc | API key (server-side only) for v1 |
| Notifier separate deploy | **Removed for v1** | No second consumer; collapses into `api` |

---

## 16. Open Questions

These block v1 implementation start; flagged in §1 for product alignment:

| # | Question | Asked by | Why it matters |
|---|---|---|---|
| 1 | Own funds, friends-and-family, or public signup for v1? | Quinn, Pat | Picks Path A confidence vs immediate Path B prep |
| 2 | Anyone else using this in next 90 days? Who? How many? | Pat | Sizes whether v1 needs auth/multitenancy |
| 3 | v1 monetization — none / subscription / fees on AUM? | Pat | Fees on AUM jumps straight to v3 (license) |
| 4 | Is the rebalance suggestion text-only acceptable, or does the user want intent-relay-but-user-signs (Path A with light teeth)? | Sam | Determines whether `chain-evm` ships intent-relay ports in v1 |
| 5 | Does the team have any existing Turnkey / Privy / DFNS evaluation? | Pat | Pre-positions v2 escalation |

---

## 17. Roadmap Sequencing — What Survives Across Phases

| Architectural decision | v1 | v2 | v3 |
|---|---|---|---|
| Hexagonal layering | ✅ | ✅ | ✅ |
| Modular monolith — `api` binary | ✅ | ✅ + `executor` | ✅ + `executor` + `signer` |
| Outbox + LISTEN/NOTIFY | ✅ | ✅ | ✅ |
| Per-VM chain ports | ✅ | ✅ + Solana real | ✅ + Bitcoin (if applicable) |
| Subsquid indexer | ✅ EVM | ✅ + Helius (Solana) | unchanged |
| `sqlx` + compile-time SQL | ✅ | ✅ | ✅ |
| OTel + scrubbing | ✅ | ✅ | ✅ |
| `figment` + `secrecy` | ✅ | ✅ | ✅ |
| Server Actions + RSC + PPR | ✅ | ✅ | ✅ |
| API key in env-only | ✅ | ✅ → Keycloak | Keycloak + dual-control admin |
| EIP-712 intents | — | ✅ | ✅ |
| Simulate-before-broadcast | — | ✅ | ✅ |
| MEV-protected RPCs | — | ✅ | ✅ |
| OFAC screening (SaaS) | — | ✅ | ✅ + in-house policy gate |
| Hash-chained audit | — | ✅ | ✅ + signed by signer service |
| Custodial signer | — | SDK (Turnkey/Privy) | In-house, KMS/HSM, Safe 2-of-3 |
| Horizontal scale (deployed) | — | ✅ if traction | ✅ |
| MTL/MiCA license | — | not required (SaaS holds it) | **required** |
| Crime + custody insurance | — | not required | **required** |

> **The whole point of v2.0 architecture:** every escalation is an *additive* deploy, not a rewrite. The seams are already there.

---

## 18. Next Steps (implementation backlog seed)

- [ ] Confirm Open Questions §16 with the user
- [ ] Initialize Cargo workspace per §4.1 layout
- [ ] Migrate from SeaORM to `sqlx` (per AD-003); set up `sqlx-cli` migrations
- [ ] Wire `figment` + `secrecy` config layer (per AD-008)
- [ ] Stand up `tracing` + `opentelemetry_otlp` with PII scrubber (per AD-007)
- [ ] Define `chain-port` traits; scaffold `chain-evm`, `chain-solana` (stubs), `chain-bitcoin` (read stub)
- [ ] Choose Subsquid hosting (self-hosted squid vs Subsquid Cloud) for Base, Arbitrum, Ethereum
- [ ] Implement `holdings_projection` table + indexer worker
- [ ] Expose OpenAPI via `utoipa`; add CI step to generate `openapi-typescript` for web
- [ ] Implement Server Action wrappers for portfolio mutations; verify API key never reaches client bundle
- [ ] Wire Reown AppKit + wagmi v2 + viem v2 for read-only wallet connect
- [ ] Stand up Ladle, port shared UI to shadcn/ui, wire Tailwind v4 `@theme` tokens
- [ ] Service worker + offline shell + cursor-based replay endpoint
- [ ] CI gates: `cargo audit`, `cargo deny`, `cargo nextest`, axe-core a11y, openapi-diff

---

## Appendix A — Process Notes

This v2 is the output of a four-round BMAD party-mode roundtable on 2026-04-27 with Sam (Software Architect), Alex (Blockchain Architect), Ortis (Backend), Fe (Frontend), Quinn (Security/QA), and Pat (PM). Round 1 surfaced modernization moves; Round 2 over-built a custodial fortress; Round 3 corrected scope to non-custodial v1 once the user said "I have no idea about licensing — keep simplest." This document is the converged result.

Two unresolved technical disagreements (parked until v2 implementation):
- **Signer wire protocol** — Postgres outbox (Ortis) vs gRPC + mTLS (Quinn/Alex). Decide at v2 start.
- **Nonce strategy under horizontal scale** — per-account row-lock in Postgres (Ortis) vs centralized nonce-manager with Redis fencing token (Alex). Decide at v2 start; default to row-lock until contention demands otherwise.
