# crypto-pocket-butler

A small (but serious) pet project: **crypto portfolio management** across wallets + exchanges, with an **OpenClaw agent** that produces rebalancing suggestions and writes daily briefs to Notion.

## Architecture

### Backend (Rust)
- **Framework**: Axum 0.8
- **Authentication**: Keycloak JWT validation with axum-keycloak-auth
- **API Documentation**: Swagger UI with utoipa
- **Location**: `backend-rust/`

### Frontend (Next.js)
- **Framework**: Next.js 16 with App Router
- **Authentication**: NextAuth.js v5 with Keycloak OIDC (Authorization Code + PKCE)
- **Styling**: TailwindCSS 4
- **Language**: TypeScript
- **Location**: `frontend-react/`

See [docs/FRONTEND_SETUP.md](docs/FRONTEND_SETUP.md) for detailed setup instructions and [docs/UI-STYLE-GUIDE.md](docs/UI-STYLE-GUIDE.md) for the design system documentation.

## Quick Start

### Backend
```bash
cd backend-rust
cargo run
```

### Frontend
```bash
cd frontend-react
npm install
cp .env.example .env.local
# Configure .env.local with your Keycloak settings
npm run dev
```

## Planned stack
- **Backend:** Rust (Axum) + Postgres
- **Frontend:** React (Next.js) + TypeScript ✅ **Implemented**
- **Agent:** OpenClaw (suggestions first, execution later with guardrails)

## MVP (first milestone)
- Connect OKX (read-only) + 1 wallet type
- Normalize holdings into one schema
- Show consolidated portfolio + allocation
- Generate a daily/weekly rebalancing suggestion

## Guardrails (draft)
- Base currency: USD
- Rebalancing: fixed targets + guardrails
- Stablecoin minimum: TBD
- Futures cap: TBD

## Security
- ✅ **Keycloak OIDC authentication** with PKCE flow (frontend)
- ✅ **JWT validation** on backend API
- ✅ **Bearer token authentication** for API calls
- Start **read-only** for exchanges.
- Never enable withdrawals.
- If trading is enabled later: strict allowlists + max order sizes + full audit log.
