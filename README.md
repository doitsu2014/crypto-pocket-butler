# crypto-pocket-butler

A small (but serious) pet project: **crypto portfolio management** across wallets + exchanges, with an **OpenClaw agent** that produces rebalancing suggestions and writes daily briefs to Notion.

## Architecture

### Backend (Rust)
- **Framework**: Axum 0.8
- **Authentication**: Keycloak JWT validation with axum-keycloak-auth
- **Database**: PostgreSQL with SeaORM
- **API Documentation**: Swagger UI with utoipa (publicly accessible at `/swagger-ui`)
- **Location**: `api/`

### Frontend (Next.js)
- **Framework**: Next.js 16 with App Router
- **Authentication**: NextAuth.js v5 with Keycloak OIDC (Authorization Code + PKCE)
- **Styling**: TailwindCSS 4
- **Language**: TypeScript
- **Location**: `web/`

See [docs/FRONTEND_SETUP.md](docs/FRONTEND_SETUP.md) for detailed setup instructions and [docs/UI-STYLE-GUIDE.md](docs/UI-STYLE-GUIDE.md) for the design system documentation.

## Quick Start

### Option 1: Docker Compose (Recommended)

The easiest way to get started is using Docker Compose, which sets up the entire stack (database, Keycloak, backend, frontend):

```bash
# Copy environment variables
cp .env.example .env
# Edit .env and set NEXTAUTH_SECRET (generate with: openssl rand -base64 32)

# Start all services
docker-compose up -d
```

See [DOCKER_SETUP.md](DOCKER_SETUP.md) for detailed Docker instructions.

### Option 2: Local Development Setup

### Prerequisites

- **Rust** 1.70+ with Cargo
- **Node.js** 18+ with npm
- **Docker & Docker Compose** (for PostgreSQL and Keycloak)
- **PostgreSQL** 16+ (or use the provided docker-compose)

### 1. Database Setup

Start PostgreSQL using Docker Compose:

```bash
cd api
docker-compose up -d
```

This will start PostgreSQL on `localhost:5432` with:
- Database: `crypto_pocket_butler`
- Username: `postgres`
- Password: `postgres`

Run database migrations:

```bash
cd api/migration
cargo run
```

### 2. Keycloak Setup

Follow the [Keycloak Setup Guide](docs/KEYCLOAK_SETUP.md) to set up authentication.

**Quick setup for local development:**

```bash
docker run -d \
  --name keycloak \
  -p 8080:8080 \
  -e KEYCLOAK_ADMIN=admin \
  -e KEYCLOAK_ADMIN_PASSWORD=admin \
  quay.io/keycloak/keycloak:latest \
  start-dev
```

Then configure:
1. Create realm: `myrealm`
2. Create client: `crypto-pocket-butler`
3. Configure redirect URIs and client settings
4. Create a test user

See [docs/KEYCLOAK_SETUP.md](docs/KEYCLOAK_SETUP.md) for detailed instructions.

### 3. Backend Setup

Configure environment variables:

```bash
cd api
cp .env.example .env
# Edit .env with your actual configuration
```

Required environment variables in `.env`:
- `DATABASE_URL`: PostgreSQL connection string
- `KEYCLOAK_SERVER`: Your Keycloak server URL (e.g., `http://localhost:8080`)
- `KEYCLOAK_REALM`: Keycloak realm name (e.g., `myrealm`)
- `KEYCLOAK_AUDIENCE`: Keycloak client ID (e.g., `crypto-pocket-butler`)

Start the backend server:

```bash
cd api
cargo run
```

The backend will be available at:
- **API**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger-ui
- **OpenAPI Spec**: http://localhost:3000/api-docs/openapi.json

### 4. Frontend Setup

Configure environment variables:

```bash
cd web
npm install
# Create .env.local with your Keycloak settings (see docs/FRONTEND_SETUP.md)
```

Start the frontend development server:

```bash
cd web
npm run dev
```

The frontend will be available at http://localhost:3001

See [docs/FRONTEND_SETUP.md](docs/FRONTEND_SETUP.md) for detailed frontend setup and configuration.

## Development Workflow

### Running Tests

**Backend:**
```bash
cd api
cargo test
```

**Frontend:**
```bash
cd web
npm test
```

### Building for Production

**Backend:**
```bash
cd api
cargo build --release
```

**Frontend:**
```bash
cd web
npm run build
```

### Database Management

**Run migrations:**
```bash
cd api/migration
cargo run -- up
```

**Rollback last migration:**
```bash
cd api/migration
cargo run -- down
```

**Reset database:**
```bash
cd api/migration
cargo run -- reset
```

## Project Structure

```
.
├── api/                   # Rust backend with Axum
│   ├── src/              # Source code
│   ├── migration/        # Database migrations
│   ├── .env.example      # Environment variables template
│   └── Cargo.toml        # Rust dependencies
├── web/                   # Next.js frontend
│   ├── app/              # App Router pages
│   ├── components/       # React components
│   └── lib/              # Utilities and helpers
├── docs/                  # Documentation
│   ├── FRONTEND_SETUP.md
│   ├── KEYCLOAK_SETUP.md
│   └── UI-STYLE-GUIDE.md
└── README.md             # This file
```

## Environment Variables Reference

### Backend Environment Variables

See `api/.env.example` for the complete list. Key variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:postgres@localhost/crypto_pocket_butler` |
| `KEYCLOAK_SERVER` | Keycloak server URL | `https://keycloak.example.com` |
| `KEYCLOAK_REALM` | Keycloak realm name | `myrealm` |
| `KEYCLOAK_AUDIENCE` | Keycloak client ID | `account` |
| `DB_MAX_CONNECTIONS` | Maximum database connections | `100` |
| `DB_MIN_CONNECTIONS` | Minimum idle connections | `5` |

### Frontend Environment Variables

See [docs/FRONTEND_SETUP.md](docs/FRONTEND_SETUP.md) for frontend configuration.

## Planned stack
- **Backend:** Rust (Axum) + Postgres ✅ **Implemented**
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

## Documentation

- [Docker Setup Guide](DOCKER_SETUP.md) **← Start here for quickest setup**
- [Frontend Setup Guide](docs/FRONTEND_SETUP.md)
- [Keycloak Setup Guide](docs/KEYCLOAK_SETUP.md)
- [Swagger UI Guide](docs/SWAGGER_UI_GUIDE.md)
- [UI Style Guide](docs/UI-STYLE-GUIDE.md)
- [Backend README](api/README.md)
- [Technical Design](docs/TECHNICAL_DESIGN.md)
- [Naming Convention](NAMING_CONVENTION.md)

## Troubleshooting

### Backend won't start

1. **Database connection error**: Make sure PostgreSQL is running and `DATABASE_URL` is correct
2. **Keycloak connection error**: Verify `KEYCLOAK_SERVER` is accessible and the realm exists
3. **Missing .env file**: Copy `.env.example` to `.env` and configure the variables

### Frontend won't start

1. **Authentication error**: Check Keycloak configuration in `.env.local`
2. **Backend API error**: Make sure the backend is running on port 3000

See [docs/FRONTEND_SETUP.md](docs/FRONTEND_SETUP.md) for more troubleshooting tips.

## Contributing

This is a personal pet project, but suggestions and contributions are welcome! Please open an issue or pull request.

## License

MIT License - see LICENSE file for details.
