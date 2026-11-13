# Architecture — Crypto Pocket Butler

## System Overview

Crypto Pocket Butler is a **service-oriented, synchronous-HTTP system** deployed on Docker Swarm. A React SPA and a separate React support console call a single API Gateway. The gateway validates the user's Supabase JWT, applies a deny-by-default route table, and forwards the JWT unchanged to the owning service over the private network. Each backend service is a Rust Clean Architecture Cargo workspace (`domain`, `application`, `infrastructure`, `http`) that owns exactly one Supabase Postgres schema and one Postgres role. There are no message queues. Supabase supplies Auth (JWT/JWKS/MFA), Postgres, Storage and Vault; everything else runs on the Swarm cluster, including a self-hosted observability stack.

The architecture model of record is `docs/architecture/workspace.dsl`; this document describes the implemented code.

## Architectural Style

**Microservices with a Backend-for-Frontend gateway, per-service data ownership, and Clean Architecture inside each service.**

Evidence:

- 12 service packages under `services/` plus the gateway, each with four crates (`Cargo.toml` workspace members).
- One router and one `AppState` per service (`services/<svc>/http/src/{routes,state}.rs`), each with its own `/healthz`, `/readyz`, `/metrics`.
- One migration pair per service schema (`services/<svc>/migrations/000N_*.{up,down}.sql`), one schema per service (`infra/sql/*-schema.sql`).
- Dependencies point inward only, enforced mechanically by `scripts/check-boundaries.sh`.
- Synchronous internal HTTP with bounded retries and idempotency keys; no queue, no outbox (`docs/architecture/README.md` §6).

## Component Relationships

Documented target wiring. In the implemented code the gateway's deny-by-default route table currently declares only Asset Registry paths (`services/gateway/domain/src/route.rs:68-90`); the other edges are the intended wiring described in `docs/architecture/README.md` §6 and are reachable today only on each service's published port.

```mermaid
flowchart LR
  subgraph Clients
    SPA["React Web Client (SPA)"]
    SC["Support Console"]
  end
  AUTH["Supabase Auth"]
  WC["WalletConnect Relay"]
  GW["API Gateway (BFF)"]
  subgraph Services["Rust services on Docker Swarm"]
    ID["Identity Service"]
    AC["Account Service"]
    AR["Asset Registry Service"]
    MD["Market Data Service"]
    PR["Portfolio Service"]
    LG["Ledger Service"]
    RP["Reporting Service"]
    NT["Notification Service"]
    AN["Analytics Service"]
    JO["Job Orchestration Service"]
    AU["Audit Service"]
  end
  subgraph Platform["Supabase platform"]
    PG["Postgres (one schema per service)"]
    VA["Vault (pgsodium)"]
    ST["Storage (private buckets)"]
  end
  SPA -->|"HTTPS /v1"| GW
  SC -->|"HTTPS /v1"| GW
  SPA -.->|"sign-in, refresh, MFA"| AUTH
  SPA -.->|"read-only session"| WC
  GW -->|"forwarded JWT"| ID
  GW -->|"forwarded JWT"| AC
  GW -->|"forwarded JWT"| AR
  GW -->|"forwarded JWT"| MD
  GW -->|"forwarded JWT"| PR
  GW -->|"forwarded JWT"| LG
  GW -->|"forwarded JWT"| RP
  GW -->|"forwarded JWT"| NT
  GW -->|"forwarded JWT"| AN
  GW -->|"forwarded JWT"| JO
  GW -->|"forwarded JWT"| AU
  JO -->|"service token"| AC
  JO -->|"service token"| MD
  JO -->|"service token"| PR
  JO -->|"service token"| LG
  JO -->|"service token"| RP
  JO -->|"service token"| NT
  JO -->|"service token"| AN
  JO -->|"service token"| AU
  PR -->|"internal HTTP"| AC
  PR -->|"internal HTTP"| LG
  PR -->|"internal HTTP"| MD
  PR -->|"internal HTTP"| ID
  RP -->|"internal HTTP"| PR
  RP -->|"internal HTTP"| LG
  RP -->|"internal HTTP"| ID
  NT -->|"C3 verified contact"| ID
  AC -->|"internal HTTP"| AR
  AC -->|"internal HTTP"| NT
  ID -->|"audit events"| AU
  AC -->|"audit events"| AU
  PR -->|"audit events"| AU
  LG -->|"audit events"| AU
  RP -->|"audit events"| AU
  AN -->|"privacy erase"| ID
  ID --> PG
  AC --> PG
  AR --> PG
  MD --> PG
  PR --> PG
  LG --> PG
  RP --> PG
  NT --> PG
  AN --> PG
  JO --> PG
  AU --> PG
  ID --> VA
  AC --> VA
  MD --> VA
  ID --> ST
  RP --> ST
  AU --> ST
```

Text fallback: the SPA and support console call only the gateway, plus Supabase Auth and WalletConnect directly. The gateway forwards the JWT unchanged to the owning service. Services read each other only over internal HTTP; the Job Orchestration Service invokes run endpoints with a service token; every service writes audit events and its own Postgres schema; secrets come from Vault; exports and PDFs go to private Storage buckets.

## Data Flow

1. **Authentication** — the SPA signs in with Supabase Auth (PKCE); the access token is held in memory only.
2. **Edge** — every request carries `Authorization: Bearer <jwt>`; the gateway validates signature/issuer/audience/expiry against a cached JWKS, matches a declared route, checks the admin gate, adds `x-correlation-id` and — for non-idempotent methods — an `idempotency-key`, then forwards.
3. **Service** — each service re-validates the JWT (defence in depth), maps the validated claims to an internal `Caller`/principal, enforces ownership, runs the use case behind port traits, and maps errors to RFC 7807 problems.
4. **Persistence** — sqlx repositories read/write only the service's own schema; cross-service data comes from that service's HTTP API.
5. **Audit** — security-relevant events are written to the Audit service; domain events are best-effort with local logging fallback.
6. **Scheduling** — the Job Orchestration Service holds a single lease row, ticks, and calls each service's `/v1/internal/run/<job>` with a per-window idempotency key.

## Key Design Decisions

| Decision | Implementation evidence | Cost accepted |
| --- | --- | --- |
| Four-crate Clean Architecture per service | `Cargo.toml`, `services/<svc>/{domain,application,infrastructure,http}`; `scripts/check-boundaries.sh` | More crates and boilerplate per service |
| Forwarded Supabase JWT, validated in every service | `services/*/infrastructure/src/auth.rs` JWKS validators; `require_jwt` middleware | Every service needs JWKS access and clock-skew tolerance |
| Deny-by-default gateway route table | `services/gateway/domain/src/route.rs`; `AuthorizeAndRoute::execute` | Every new endpoint must be declared centrally |
| One schema and role per service, no cross-schema grants | `infra/sql/*-schema.sql`; migration role blocks | Cross-service reads need HTTP and a contract |
| No message queues; bounded retry + reconciliation | `services/*/infrastructure/src/http_retry.rs`; scheduler run records | Each path re-implements retry; eventual consistency repaired by jobs |
| Append-only audit via insert-only role and triggers | `services/audit/migrations/0001_init.up.sql:399-403`; trigger guards | No cryptographic chaining; tamper evidence is grant-based |
| pgsodium crypto-shredding for erasure | `identity.shredding_keys`, `user_personal`; Vault key references | Per-subject keys must exist before the data they protect |
| Secret files, never env values, for credentials | `services/*/src/config.rs` (`secret(...)` reads `*_FILE`) | Operators must materialise Docker secrets from Vault |
| In-memory gateway rate limits and caches | `services/gateway/http/src/rate_limit.rs`; JWKS `RwLock` cache | Per-replica state; horizontal scaling changes limits |
| Rust decimal arithmetic | `rust_decimal` in `[workspace.dependencies]` | No floats; explicit rounding policy per domain |

## Improvement Opportunities

- Declare the non-registry routes in the gateway route table, or the SPA cannot reach Identity, Portfolio, Account and the rest through the documented single entry point.
- Add the Billing service (or mark BRD-016 out of scope) — the C4 model and gateway narrative currently promise entitlements that no code enforces.
- Add principal resolution and entitlement checks to the gateway if §6 of the architecture README is to remain true.
- Add a CI workflow; the documented gates are currently manual.
- Normalise the service runnable-unit convention (`src/main.rs` vs `src/bin/api.rs` vs `src/bin/{api,worker,migrate}.rs`).
- Represent BRD-002's `disabled` and `pending_deletion` statuses, or record the narrower implemented enum as a deliberate decision.

## Interaction Diagrams

### 1. First authenticated request — principal resolution and provisioning (BRD-002 FR-USR-01, AC-USR-01)

```mermaid
sequenceDiagram
    autonumber
    participant C as SPA
    participant G as API Gateway
    participant J as Supabase JWKS
    participant I as Identity Service
    participant D as identity schema
    participant U as Audit Service
    C->>G: GET /v1/principals/{authUserId}<br/>Authorization: Bearer JWT
    G->>J: fetch JWKS (5-minute cache)
    G->>G: verify signature, issuer, audience, expiry
    G->>I: forward request, JWT unchanged
    I->>J: verify JWT again (defence in depth)
    I->>D: find user by auth_subject_hash
    alt no user record
        I->>D: create shredding key reference
        I->>D: insert user, user_personal, shredding_keys
        I->>U: identity.user.provisioned
    else existing record
        I->>D: refresh role, email, verification, status
        I->>U: identity.role.changed / identity.user.activated (when changed)
    end
    I->>D: upsert device session
    I-->>G: PrincipalView(userId, role, status, nextStep)
    G-->>C: 200 principal
```

Text fallback: the gateway validates the JWT and forwards it unchanged; Identity validates it again, finds or creates exactly one internal user keyed by the hashed Auth subject, records audit events for provisioning/role change/activation, records the device session, and returns the principal with the required next step (`none`, `verify_email`, `enrol_mfa`).

### 2. Deny-by-default request (BRD-002 BR-USR-03, FR-USR-05, AC-USR-03)

```mermaid
sequenceDiagram
    autonumber
    participant C as Client
    participant G as API Gateway
    participant U as Audit Service
    C->>G: request without a Bearer token
    G->>U: gateway.denied (actor anonymous)
    G-->>C: 401 Unauthorized
    C->>G: valid JWT, undeclared path
    G->>U: gateway.denied (actor subject)
    G-->>C: 404 Not Found
    C->>G: valid JWT, admin route, role user
    G->>U: gateway.denied (actor subject)
    G-->>C: 403 Forbidden
```

Text fallback: a missing, invalid or expired token is 401; a path with no declared route is 404 and denied; an admin-declared route with a non-admin role is 403. Every denial writes a `gateway.denied` audit event carrying actor, action, target, outcome and correlation id.

### 3. Support access grant (BRD-002 FR-USR-11, BR-USR-09, AC-USR-07)

```mermaid
sequenceDiagram
    autonumber
    participant S as Support Console
    participant I as Identity Service
    participant A as Admin
    participant D as identity schema
    participant U as Audit Service
    S->>I: POST /v1/support-grants (support role, reason 1-500)
    I->>D: insert grant status=pending_approval
    I->>U: identity.support_grant.requested
    A->>I: POST /v1/support-grants/{id}/approve (admin, not requester)
    I->>D: status=active, approved_by, expires_at <= 60 minutes
    I->>U: identity.support_grant.approved
    I->>I: check grant on metadata path (gateway service token)
    alt grant active and in window
        I-->>S: metadata access allowed
    else expired or revoked
        I-->>S: 403 Forbidden
    end
```

Text fallback: a support agent requests a grant with a reason; a different admin approves it; Identity sets an expiry of at most 60 minutes and writes audit events. The metadata path checks the grant through a gateway-only internal route; an expired or revoked grant is refused.

### 4. Account sync to valuation and snapshot (business transaction across Account, Registry, Market Data, Portfolio, Ledger)

```mermaid
sequenceDiagram
    autonumber
    participant JO as Job Orchestration
    participant AC as Account Service
    participant AR as Asset Registry
    participant MD as Market Data
    participant LG as Ledger
    participant PR as Portfolio
    participant AU as Audit
    JO->>AC: POST /v1/internal/run/account-sync (service token, window key)
    AC->>AR: resolve assets and contracts
    AC->>AC: write holdings to account schema
    AC->>AU: sync audit events
    JO->>PR: POST /v1/internal/run/portfolio-valuation
    PR->>AC: GET /v1/users/{id}/holdings
    PR->>MD: GET /v1/prices and /v1/fx
    PR->>LG: GET /v1/users/{id}/pnl
    PR->>PR: value in base currency, label unpriced assets
    PR->>PR: write immutable snapshot
    PR->>AU: valuation and snapshot audit events
```

Text fallback: the scheduler triggers account sync with a service token; Account resolves asset identity through the registry, stores holdings, and audits. The scheduler then triggers portfolio valuation; Portfolio reads holdings from Account, prices and FX from Market Data, and P&L from Ledger, values in the user's base currency, labels unpriced or unmapped assets as excluded rather than zero, and writes an immutable snapshot with audit events.

### 5. Transaction import to P&L (Ledger + Portfolio)

```mermaid
sequenceDiagram
    autonumber
    participant C as SPA
    participant LG as Ledger Service
    participant AC as Account Service
    participant PR as Portfolio Service
    participant D as ledger schema
    C->>LG: import / exchange transaction feed
    LG->>D: append transaction (append-only)
    LG->>D: dedupe against source ids
    LG->>D: pair transfers, apply FIFO/average cost
    LG->>D: compute realized and unrealized P&L
    PR->>LG: GET /v1/users/{id}/pnl
    LG-->>PR: cost basis and P&L per subject
    PR->>PR: fold into valuation and performance
```

Text fallback: transactions are appended, deduplicated, transfer-paired and cost-based in the Ledger schema; Portfolio reads P&L over internal HTTP and folds it into valuation, performance and snapshots. Ledger is the system of record for cost basis; Portfolio never stores transaction rows.
