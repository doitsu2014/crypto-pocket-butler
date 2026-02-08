# 09 — Technical Design (Rust + React)

# Technical Design — Rust Backend + React Frontend
Goal: a secure, modular portfolio system. Rust handles data ingestion, normalization, storage, and scheduling; React provides dashboards + approvals. OpenClaw agent reads from the backend + writes reports to Notion/Telegram.

## Architecture (high-level)
- Rust API Server (Axum): REST/GraphQL endpoints for accounts, holdings, snapshots, recommendations.
- Rust Workers: scheduled sync jobs for OKX + wallets; price service; alerts.
- DB: Postgres (preferred) for normalized data + snapshots; Redis optional for caching.
- React Web App (Next.js): portfolio dashboard, settings, targets, approval workflow.
- OpenClaw Agent: consumes API + produces Notion briefs; can request approval for trades.
## Backend (Rust)
### Suggested crates
- axum (HTTP), tower (middleware), serde (JSON), reqwest (HTTP clients).
- sqlx or diesel (DB), chrono (time), uuid.
- tracing (logging), thiserror/anyhow (errors).
### Services/modules
1. connectors/okx: fetch balances, positions, orders (read-only first).
1. connectors/wallets: EVM first (token balances), then BTC/SOL.
1. pricing: exchange ticker + fallback (CoinGecko).
1. normalization: symbol/network mapping → canonical asset id.
1. portfolio: allocation, drift, concentration, stablecoin buffer.
1. recommendations: build trade suggestions + constraints.
## Frontend (React)
- Next.js app router, TypeScript, TanStack Query, Tailwind (optional).
### Screens
1. Dashboard: total value, allocation pie, top holdings, drift list.
1. Accounts: manage wallets + exchanges; last sync status.
1. Targets: set target weights + drift bands.
1. Recommendations: view suggested trades; approve/deny; add notes.
## Security
- Secrets: store API keys encrypted (env + KMS/SealedSecrets); never store plaintext in DB.
- Auth: JWT/session; role separation (view vs approve vs trade).
- Trade guardrails (if enabled): allowlist, max size, cooldowns, audit log.
## MVP Build Order
- [ ] Rust: data model + Postgres schema
- [ ] Rust: OKX read-only connector + snapshot writer
- [ ] React: dashboard reads latest snapshot
- [ ] OpenClaw: daily brief from snapshot
