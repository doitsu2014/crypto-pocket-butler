# crypto-pocket-butler

Crypto portfolio management across wallets and exchanges, with an agent that produces rebalancing suggestions and writes daily briefs to Notion.

## Tech Stack

### API (Rust)
- **Framework**: Axum 0.8
- **ORM**: SeaORM
- **Database**: PostgreSQL
- **Auth**: Keycloak JWT validation via axum-keycloak-auth
- **API Docs**: utoipa with Swagger UI at `/swagger-ui`
- **Location**: `api/`

### Web (Next.js)
- **Framework**: Next.js 16 with App Router
- **Auth**: NextAuth.js v5 with Keycloak OIDC (Authorization Code + PKCE)
- **Styling**: TailwindCSS 4
- **Language**: TypeScript
- **Location**: `web/`

## Commands

### API
- Run dev server: `cd api && cargo run`
- Run tests: `cd api && cargo test`
- Build release: `cd api && cargo build --release`
- Lint: `cd api && cargo clippy -- -D warnings`

### Web
- Run dev server: `cd web && npm run dev`
- Run tests: `cd web && npm test`
- Build production: `cd web && npm run build`
- Lint: `cd web && npm run lint`
- Type check: `cd web && npx tsc --noEmit`

### Database Migrations
- Apply: `cd api/migration && cargo run -- up`
- Rollback: `cd api/migration && cargo run -- down`
- Reset: `cd api/migration && cargo run -- reset`

### Docker (Full Stack)
- Start: `docker-compose up -d`
- Stop: `docker-compose down`

## Code Conventions

### Rust API
- Follow standard Rust conventions (rustfmt, clippy)
- Use SeaORM entities for database models
- Route handlers in `api/src/handler/` or `api/src/routes/`
- OpenAPI annotations via utoipa derive macros (`#[utoipa::path(...)]`)
- All API endpoints require JWT auth (except Swagger UI)
- Use `axum::extract::State` for shared state
- Use `axum::Json<T>` for JSON request/response bodies
- Error handling via application error types

### Next.js Web
- Use App Router (`app/` directory)
- Server components by default; `"use client"` only when needed
- Components in `components/` directory
- Utilities and helpers in `lib/` directory
- TailwindCSS for styling (follow UI Style Guide at `docs/web/UI-STYLE-GUIDE.md`)
- TypeScript strict mode

### General
- Base currency: USD
- Run lint and typecheck before committing
- Write tests for new features
- No secrets or credentials in code

## Project Structure

```
.
├── api/                   # Rust API with Axum
│   ├── src/              # Source code
│   ├── migration/        # Database migrations
│   └── Cargo.toml
├── web/                   # Next.js web interface
│   ├── app/              # App Router pages
│   ├── components/       # React components
│   └── lib/              # Utilities and helpers
├── docs/                  # Documentation
│   ├── setup/            # Setup guides
│   ├── architecture/     # Architecture and design
│   └── web/              # Web-specific docs
├── keycloak/              # Keycloak configuration
├── helm/                  # Kubernetes Helm charts
├── setups/                # Development setup scripts
└── tools/                 # Utility scripts
```

## Key Documentation

- Architecture: `docs/architecture/01-architecture-design/`
- Web UI Style Guide: `docs/web/UI-STYLE-GUIDE.md`
- Docker Setup: `docs/setup/DOCKER_SETUP.md`
- Web Setup: `docs/setup/WEB_SETUP.md`
- Keycloak Setup: `docs/setup/KEYCLOAK_SETUP.md`

## Environment Variables

See `api/.env.example` for API configuration. Key variables:
- `DATABASE_URL`: PostgreSQL connection string
- `KEYCLOAK_SERVER`: Keycloak server URL
- `KEYCLOAK_REALM`: Keycloak realm name
- `KEYCLOAK_AUDIENCE`: Keycloak client ID

See `docs/setup/WEB_SETUP.md` for web configuration.

## Security

- Keycloak OIDC authentication with PKCE flow (web)
- JWT validation on API
- Bearer token authentication for API calls
- Read-only for exchanges
- Never enable withdrawals
