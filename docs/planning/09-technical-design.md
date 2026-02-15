# 09 — Technical Design (Rust + React)

## Technical Design — Rust API + React Web

Goal: a secure, modular portfolio system. Rust handles data ingestion, normalization, storage, and scheduling; React provides dashboards + approvals. OpenClaw agent reads from the API and writes reports to Notion/Telegram.

Key additions:
- **Identity & Access Management via Keycloak** (authentication + authorization)
- **User-configurable portfolios** (each portfolio chooses which wallets/exchanges are included)
- **End-of-day (EOD) portfolio snapshots**

---

## Architecture (high-level)

- **Keycloak (OIDC)**: central login, token issuance, roles/groups.
- **Web (React / Next.js)**:
  - Uses **OIDC Authorization Code + PKCE** to sign users in via Keycloak.
  - Stores access token in memory (or httpOnly session via BFF pattern if preferred).
- **API (Rust / Axum)**:
  - Validates JWT access tokens using Keycloak **JWKS**.
  - Enforces **RBAC/ABAC** (role + resource ownership checks).
  - Exposes REST endpoints for: accounts, portfolios, snapshots, recommendations.
- **Workers (Rust)**:
  - Scheduled sync jobs for wallets/exchanges.
  - Scheduled **EOD snapshot job** per portfolio.
- **DB (Postgres)**:
  - Normalized accounts + holdings.
  - Portfolio definitions.
  - Timestamped snapshots.
- **OpenClaw Agent**:
  - Reads portfolio/snapshot data from API.
  - Generates rebalancing suggestions.
  - Writes daily briefs to Notion and Telegram.

---

## Identity & Access (Keycloak)

### Auth flow
- **Web → Keycloak**: Authorization Code + PKCE.
- **Keycloak → Web**: returns code; web exchanges for tokens.
- **Web → API**: sends `Authorization: Bearer <access_token>`.

### API responsibilities
- Fetch and cache Keycloak **JWKS**.
- Validate:
  - signature
  - `iss` (issuer)
  - `aud` (audience)
  - expiry (`exp`)
- Extract user identity:
  - `sub` (stable user id)
  - `preferred_username` / email (display)
- Authorization rules:
  - **Resource ownership**: user can only access their own portfolios/accounts.
  - **Roles** (example):
    - `portfolio:read`
    - `portfolio:write`
    - `trade:approve` (future)

### Recommended pattern (simple)
- Treat **Keycloak user `sub`** as `user_id` in DB.
- All portfolio/account rows are scoped by `user_id`.

---

## Portfolio Configuration (user-driven)

### Concept
A **Portfolio** is a logical container defined by the user.
Each portfolio includes one or more **data sources**:
- wallets (EVM/BTC/SOL addresses)
- exchanges (OKX subaccounts / API key profiles)

### UX requirement
In the UI:
- user creates a portfolio
- user selects which wallets/exchanges belong to it
- system shows portfolio allocation + drift for that portfolio

### API requirement
- Portfolio CRUD endpoints.
- A join table linking `portfolio_id` ↔ `account_id`.

---

## Snapshots (End-of-Day)

### Requirement
- System produces a **snapshot at end of day** for each portfolio.

### Design
- Define a configurable “EOD cutover time” per user or system-wide.
  - Example: `23:59:59 UTC` or user timezone (later).
- EOD job:
  1) resolves current holdings for the portfolio
  2) stores a snapshot row (`portfolio_snapshot`)
  3) stores snapshot holdings rows (`portfolio_snapshot_holding`)

### Outputs
- Time series charting (portfolio value over time).
- Daily performance metrics (later: TWR).

---

## API (Rust)

### Suggested crates
- `axum` (HTTP), `tower` (middleware), `serde` (JSON)
- `reqwest` (HTTP clients)
- `sqlx` (DB) + `uuid`, `chrono`
- `tracing` (logging), `anyhow`/`thiserror` (errors)
- JWT validation: either a small JWT crate + JWKS fetching, or a dedicated OIDC/JWT verifier library.

### Services/modules
1. `auth/`:
   - Keycloak config (issuer, audience)
   - JWKS fetch/cache
   - JWT validation middleware
   - role + ownership checks
2. `connectors/okx/` (read-only first)
3. `connectors/wallets/` (EVM first)
4. `pricing/`: exchange tickers + fallback
5. `normalization/`: symbol/network mapping → canonical asset id
6. `portfolio/`: allocation, drift, concentration, stablecoin buffer
7. `snapshots/`: EOD snapshot generation
8. `recommendations/`: build suggestions + constraints

---

## Web (React)

Suggested stack:
- Next.js + TypeScript
- TanStack Query

Screens:
1. **Login** (Keycloak)
2. **Portfolios**: list/create/edit portfolios
3. **Portfolio detail**: allocation + holdings + drift
4. **Accounts**: manage wallets + exchanges (read-only credentials stored server-side)
5. **Snapshots**: EOD time series + daily change
6. **Recommendations**: suggested trades; approve/deny; add notes

---

## Security

- Secrets: store exchange API keys encrypted; never store plaintext in DB.
- Keycloak tokens:
  - validate strictly (`iss`, `aud`, `exp`)
  - short-lived access tokens
- Trade guardrails (if enabled later): allowlist, max size, cooldowns, audit log.

---

## MVP Build Order

- [ ] Keycloak integration (web PKCE + API JWT validation)
- [ ] DB schema: users (by `sub`), accounts, portfolios, portfolio_account links
- [ ] OKX read-only connector + snapshot writer
- [ ] EOD snapshot job (portfolio snapshots)
- [ ] React: portfolio dashboard reads latest snapshot
- [ ] OpenClaw: daily brief from snapshots
