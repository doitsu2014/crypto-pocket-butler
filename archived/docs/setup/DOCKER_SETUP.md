# Docker Compose Setup

This document provides instructions for running the entire Crypto Pocket Butler application stack using Docker Compose.

## Quick Start

### Prerequisites

- Docker 24.0+
- Docker Compose 2.20+

### 1. Configure Environment Variables

Copy the example environment file and customize it:

```bash
cp .env.example .env
```

Edit `.env` and set at minimum:
- `NEXTAUTH_SECRET`: Generate a secure secret with `openssl rand -base64 32`

Optional configuration:
- `KEYCLOAK_REALM`: Keycloak realm name (default: `myrealm`)
- `KEYCLOAK_CLIENT_ID`: Keycloak client ID (default: `crypto-pocket-butler`)
- `KEYCLOAK_CLIENT_SECRET`: Client secret (auto-generated if empty)
- `WEB_ROOT_URL`: Web application URL (default: `http://localhost:3001`)

**Note**: The default configuration uses internal Docker network names for service-to-service communication (e.g., `http://keycloak:8080`), while external access uses `localhost`. This is the recommended setup for Docker Compose.

### 2. Start All Services

Start the entire stack (database, Keycloak, api, web):

```bash
docker-compose up -d
```

This will:
1. Start PostgreSQL database on port 5432
2. Create Keycloak database in PostgreSQL
3. Start Keycloak authentication server on port 8080 (using PostgreSQL)
4. **Automatically configure Keycloak** with realm, OAuth 2.0 client, and test user
5. Build and start the Rust API on port 3000
6. Build and start the Next.js web service on port 3001

### 3. Keycloak Configuration (Automatic)

The stack includes an automatic Keycloak initialization service that:

✅ Creates the realm (default: `myrealm`)
✅ Creates an OAuth 2.0 client with Authorization Code flow and PKCE
✅ Configures redirect URIs for the web application
✅ Creates a test user (`testuser` / `testpass123`)

**No manual Keycloak configuration needed!**

To view the initialization logs:

```bash
docker-compose logs keycloak-init
```

If a client secret was auto-generated, it will be displayed in the logs. Copy it to your `.env` file as `KEYCLOAK_CLIENT_SECRET`.

### 4. Access the Application

- **Web**: http://localhost:3001
- **API**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger-ui
- **Keycloak**: http://localhost:8080

## Service Management

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f api
docker-compose logs -f web
docker-compose logs -f postgres
docker-compose logs -f keycloak
```

### Stop Services

```bash
# Stop all services
docker-compose down

# Stop and remove volumes (WARNING: deletes database data)
docker-compose down -v
```

### Restart a Service

```bash
docker-compose restart api
docker-compose restart web
```

### Rebuild Services

After code changes:

```bash
# Rebuild specific service
docker-compose up -d --build api

# Rebuild all services
docker-compose up -d --build
```

## Architecture

The Docker Compose setup includes:

1. **PostgreSQL** (postgres:16-alpine)
   - Database for both the application and Keycloak
   - Two databases: `crypto_pocket_butler` and `keycloak`
   - Data persisted in Docker volume `postgres_data`
   - Port: 5432

2. **Keycloak** (quay.io/keycloak/keycloak:26.0)
   - Authentication and authorization server
   - **Uses PostgreSQL** for persistent storage (not dev mode)
   - Configured with OAuth 2.0 Authorization Code flow with PKCE
   - Admin console: http://localhost:8080
   - Port: 8080

3. **Keycloak Init** (Alpine with curl, bash, jq)
   - One-time initialization service
   - Automatically configures Keycloak realm and OAuth 2.0 client
   - Runs after Keycloak is healthy and exits
   - See [keycloak/README.md](../../keycloak/README.md) for details

4. **API** (Rust/Axum)
   - RESTful API
   - Automatic database migrations on startup
   - JWT validation with Keycloak
   - Port: 3000

5. **Web** (Next.js)
   - React-based web interface
   - Server-side rendering
   - Port: 3001

All services are connected via a bridge network `crypto-pocket-butler-network`.

## OAuth 2.0 Configuration

The Keycloak initialization automatically configures:

### Authorization Code Flow with PKCE

- **Flow Type**: OAuth 2.0 Authorization Code Grant
- **PKCE**: Enabled with S256 code challenge method
- **Client Type**: Confidential (requires client secret)
- **Standard Flow**: Enabled
- **Implicit Flow**: Disabled (deprecated)
- **Direct Access Grants**: Enabled (for testing)

### Client Configuration

- **Client ID**: `crypto-pocket-butler` (configurable via `KEYCLOAK_CLIENT_ID`)
- **Redirect URIs**:
  - `http://localhost:3001/*`
  - `http://localhost:3001/api/auth/callback/keycloak`
- **Web Origins**: `http://localhost:3001`
- **Root URL**: `http://localhost:3001`

### Test Credentials

- **Username**: `testuser`
- **Password**: `testpass123`
- **Email**: `test@example.com`

See [keycloak/README.md](../../keycloak/README.md) for more details on the OAuth 2.0 setup.

## Development Workflow

### Local Development vs Docker

For active development, you might prefer to run services locally:

```bash
# Run only infrastructure services (database + Keycloak)
docker-compose up -d postgres keycloak keycloak-init

# Then run api and web locally
cd api && cargo run
cd web && npm run dev
```

### API Only

To run only the API with database:

```bash
cd api
docker-compose up -d
```

## Troubleshooting

### API fails to start

1. **Database connection error**: Ensure PostgreSQL is healthy
   ```bash
   docker-compose ps
   docker-compose logs postgres
   ```

2. **Migration errors**: Check API logs
   ```bash
   docker-compose logs api
   ```

3. **Keycloak connection error**: Ensure Keycloak is running and healthy
   ```bash
   docker-compose logs keycloak
   ```

### Web fails to start

1. **Build errors**: Check if standalone output is enabled
   ```bash
   docker-compose logs web
   ```

2. **Authentication errors**: Verify Keycloak configuration in `.env`

### Port conflicts

If ports are already in use, you can modify them in `docker-compose.yml`:

```yaml
ports:
  - "5433:5432"  # Use 5433 instead of 5432 for PostgreSQL
  - "8081:8080"  # Use 8081 instead of 8080 for Keycloak
  - "3001:3000"  # Use 3001 instead of 3000 for api
  - "3002:3001"  # Use 3002 instead of 3001 for web
```

### Clean start

To start fresh (removes all data):

```bash
docker-compose down -v
docker-compose up -d
```

## Production Considerations

This Docker Compose setup is designed for development. For production:

1. Use proper secrets management (not `.env` files)
2. Enable HTTPS/TLS
3. Use production-ready Keycloak setup (not dev mode)
4. Configure proper resource limits
5. Use external database with backups
6. Set up proper logging and monitoring
7. Use container orchestration (Kubernetes, Docker Swarm)

## Environment Variables Reference

| Variable | Description | Default |
|----------|-------------|---------|
| `KEYCLOAK_REALM` | Keycloak realm name | `myrealm` |
| `KEYCLOAK_AUDIENCE` | Keycloak client ID for backend JWT validation | `crypto-pocket-butler` |
| `KEYCLOAK_CLIENT_ID` | Keycloak client ID for web | `crypto-pocket-butler` |
| `KEYCLOAK_CLIENT_SECRET` | Keycloak client secret (if confidential client) | (empty) |
| `NEXTAUTH_SECRET` | NextAuth.js secret for session encryption | `change-me-in-production-use-openssl-rand-base64-32` |
| `RUST_LOG` | API logging level | `crypto_pocket_butler_backend=info` |

## Additional Resources

- [Web Setup Guide](WEB_SETUP.md)
- [Keycloak Setup Guide](KEYCLOAK_SETUP.md)
- [API Documentation](../api/api-overview.md)
