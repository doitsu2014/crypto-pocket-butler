# crypto-pocket-butler

A small (but serious) pet project: **crypto portfolio management** across wallets + exchanges, with an **OpenClaw agent** that produces rebalancing suggestions and writes daily briefs to Notion.

## Architecture

### API (Rust)
- **Framework**: Axum 0.8
- **Authentication**: Keycloak JWT validation with axum-keycloak-auth
- **Database**: PostgreSQL with SeaORM
- **API Documentation**: Swagger UI with utoipa (publicly accessible at `/swagger-ui`)
- **Location**: `api/`

### Web (Next.js)
- **Framework**: Next.js 16 with App Router
- **Authentication**: NextAuth.js v5 with Keycloak OIDC (Authorization Code + PKCE)
- **Styling**: TailwindCSS 4
- **Language**: TypeScript
- **Location**: `web/`

See [docs/setup/WEB_SETUP.md](docs/setup/WEB_SETUP.md) for detailed setup instructions and [docs/web/UI-STYLE-GUIDE.md](docs/web/UI-STYLE-GUIDE.md) for the design system documentation.

## Quick Start

### Option 1: Docker Compose (Recommended)

The easiest way to get started is using Docker Compose, which sets up the entire stack (database, Keycloak, api, web):

```bash
# Copy environment variables
cp .env.example .env
# Edit .env and set NEXTAUTH_SECRET (generate with: openssl rand -base64 32)

# Start all services
docker-compose up -d
```

See [docs/setup/DOCKER_SETUP.md](docs/setup/DOCKER_SETUP.md) for detailed Docker instructions.

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

When using Docker Compose (recommended), Keycloak is **automatically configured** with:
- PostgreSQL database for persistent storage
- Realm: `myrealm`
- OAuth 2.0 client with Authorization Code flow and PKCE
- Test user: `testuser` / `testpass123`

No manual Keycloak configuration needed!

For manual setup or additional configuration, see [docs/setup/KEYCLOAK_SETUP.md](docs/setup/KEYCLOAK_SETUP.md).

### 3. API Setup

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

Start the API server:

```bash
cd api
cargo run
```

The API will be available at:
- **API**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger-ui
- **OpenAPI Spec**: http://localhost:3000/api-docs/openapi.json

### 4. Web Setup

Configure environment variables:

```bash
cd web
npm install
# Create .env.local with your Keycloak settings (see docs/setup/WEB_SETUP.md)
```

Start the web development server:

```bash
cd web
npm run dev
```

The web interface will be available at http://localhost:3001

See [docs/setup/WEB_SETUP.md](docs/setup/WEB_SETUP.md) for detailed web setup and configuration.

## Development Workflow

### Running Tests

**API:**
```bash
cd api
cargo test
```

**Web:**
```bash
cd web
npm test
```

### Building for Production

**API:**
```bash
cd api
cargo build --release
```

**Web:**
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
├── api/                   # Rust API with Axum
│   ├── src/              # Source code
│   ├── migration/        # Database migrations
│   ├── .env.example      # Environment variables template
│   └── Cargo.toml        # Rust dependencies
├── web/                   # Next.js web interface
│   ├── app/              # App Router pages
│   ├── components/       # React components
│   └── lib/              # Utilities and helpers
├── docs/                  # Documentation (see docs/README.md)
│   ├── setup/            # Setup and configuration guides
│   ├── architecture/     # Architecture and design docs
│   ├── api/              # API-specific documentation
│   ├── web/              # Web-specific documentation
│   ├── development/      # Implementation summaries
│   └── planning/         # Project planning documents
└── README.md             # This file
```

## Environment Variables Reference

### API Environment Variables

See `api/.env.example` for the complete list. Key variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:postgres@localhost/crypto_pocket_butler` |
| `KEYCLOAK_SERVER` | Keycloak server URL | `https://keycloak.example.com` |
| `KEYCLOAK_REALM` | Keycloak realm name | `myrealm` |
| `KEYCLOAK_AUDIENCE` | Keycloak client ID | `account` |
| `DB_MAX_CONNECTIONS` | Maximum database connections | `100` |
| `DB_MIN_CONNECTIONS` | Minimum idle connections | `5` |

### Web Environment Variables

See [docs/setup/WEB_SETUP.md](docs/setup/WEB_SETUP.md) for web configuration.

## Planned stack
- **API:** Rust (Axum) + Postgres ✅ **Implemented**
- **Web:** React (Next.js) + TypeScript ✅ **Implemented**
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
- ✅ **Keycloak OIDC authentication** with PKCE flow (web)
- ✅ **JWT validation** on API
- ✅ **Bearer token authentication** for API calls
- Start **read-only** for exchanges.
- Never enable withdrawals.
- If trading is enabled later: strict allowlists + max order sizes + full audit log.

## Documentation

**📚 [Browse all documentation](docs/README.md)**

**Quick Links:**
- [Docker Setup Guide](docs/setup/DOCKER_SETUP.md) **← Start here for quickest setup**
- [Web Setup Guide](docs/setup/WEB_SETUP.md)
- [Keycloak Setup Guide](docs/setup/KEYCLOAK_SETUP.md)
- [API Overview](docs/api/api-overview.md)
- [UI Style Guide](docs/web/UI-STYLE-GUIDE.md)
- [Technical Design](docs/architecture/TECHNICAL_DESIGN.md)
- [Naming Convention](docs/architecture/NAMING_CONVENTION.md)

## Troubleshooting

### API won't start

1. **Database connection error**: Make sure PostgreSQL is running and `DATABASE_URL` is correct
2. **Keycloak connection error**: Verify `KEYCLOAK_SERVER` is accessible and the realm exists
3. **Missing .env file**: Copy `.env.example` to `.env` and configure the variables

### Web won't start

1. **Authentication error**: Check Keycloak configuration in `.env.local`
2. **API connection error**: Make sure the API is running on port 3000

See [docs/setup/WEB_SETUP.md](docs/setup/WEB_SETUP.md) for more troubleshooting tips.

## Contributing

This is a personal pet project, but suggestions and contributions are welcome! Please open an issue or pull request.

## License

MIT License - see LICENSE file for details.
