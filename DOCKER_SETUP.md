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

Required configuration in `.env`:
- `KEYCLOAK_REALM`: Your Keycloak realm name (default: `myrealm`)
- `KEYCLOAK_CLIENT_ID`: Your Keycloak client ID (default: `crypto-pocket-butler`)
- `KEYCLOAK_CLIENT_SECRET`: Your Keycloak client secret (leave empty for public client)
- `NEXTAUTH_SECRET`: Generate a secure secret with `openssl rand -base64 32`

**Note**: The default configuration uses internal Docker network names for service-to-service communication (e.g., `http://keycloak:8080`), while external access uses `localhost`. This is the recommended setup for Docker Compose.

### 2. Start All Services

Start the entire stack (database, Keycloak, backend, frontend):

```bash
docker-compose up -d
```

This will:
1. Start PostgreSQL database on port 5432
2. Start Keycloak authentication server on port 8080
3. Build and start the Rust backend API on port 3000
4. Build and start the Next.js frontend on port 3001

### 3. Configure Keycloak

After the services are up, configure Keycloak:

1. Access Keycloak admin console at http://localhost:8080
2. Login with admin/admin
3. Create a realm named `myrealm` (or use the name you set in `.env`)
4. Create a client named `crypto-pocket-butler`
5. Configure the client:
   - Client authentication: OFF (for public client) or ON (for confidential client)
   - Valid redirect URIs: `http://localhost:3001/*`
   - Web origins: `http://localhost:3001`
6. Create a test user

See [docs/KEYCLOAK_SETUP.md](docs/KEYCLOAK_SETUP.md) for detailed instructions.

### 4. Access the Application

- **Frontend**: http://localhost:3001
- **Backend API**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger-ui
- **Keycloak**: http://localhost:8080

## Service Management

### View Logs

```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f backend
docker-compose logs -f frontend
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
docker-compose restart backend
docker-compose restart frontend
```

### Rebuild Services

After code changes:

```bash
# Rebuild specific service
docker-compose up -d --build backend

# Rebuild all services
docker-compose up -d --build
```

## Architecture

The Docker Compose setup includes:

1. **PostgreSQL** (postgres:16-alpine)
   - Database for the application
   - Data persisted in Docker volume `postgres_data`
   - Port: 5432

2. **Keycloak** (quay.io/keycloak/keycloak:26.0)
   - Authentication and authorization server
   - Admin console: http://localhost:8080
   - Port: 8080

3. **Backend** (Rust/Axum)
   - RESTful API
   - Automatic database migrations on startup
   - Port: 3000

4. **Frontend** (Next.js)
   - React-based web interface
   - Server-side rendering
   - Port: 3001

All services are connected via a bridge network `crypto-pocket-butler-network`.

## Development Workflow

### Local Development vs Docker

For active development, you might prefer to run services locally:

```bash
# Run only infrastructure services (database + Keycloak)
docker-compose up -d postgres keycloak

# Then run backend and frontend locally
cd api && cargo run
cd web && npm run dev
```

### Backend Only

To run only the backend with database:

```bash
cd api
docker-compose up -d
```

## Troubleshooting

### Backend fails to start

1. **Database connection error**: Ensure PostgreSQL is healthy
   ```bash
   docker-compose ps
   docker-compose logs postgres
   ```

2. **Migration errors**: Check backend logs
   ```bash
   docker-compose logs backend
   ```

3. **Keycloak connection error**: Ensure Keycloak is running and healthy
   ```bash
   docker-compose logs keycloak
   ```

### Frontend fails to start

1. **Build errors**: Check if standalone output is enabled
   ```bash
   docker-compose logs frontend
   ```

2. **Authentication errors**: Verify Keycloak configuration in `.env`

### Port conflicts

If ports are already in use, you can modify them in `docker-compose.yml`:

```yaml
ports:
  - "5433:5432"  # Use 5433 instead of 5432 for PostgreSQL
  - "8081:8080"  # Use 8081 instead of 8080 for Keycloak
  - "3001:3000"  # Use 3001 instead of 3000 for backend
  - "3002:3001"  # Use 3002 instead of 3001 for frontend
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
| `KEYCLOAK_CLIENT_ID` | Keycloak client ID for frontend | `crypto-pocket-butler` |
| `KEYCLOAK_CLIENT_SECRET` | Keycloak client secret (if confidential client) | (empty) |
| `NEXTAUTH_SECRET` | NextAuth.js secret for session encryption | `change-me-in-production-use-openssl-rand-base64-32` |
| `RUST_LOG` | Backend logging level | `crypto_pocket_butler_backend=info` |

## Additional Resources

- [Frontend Setup Guide](docs/FRONTEND_SETUP.md)
- [Keycloak Setup Guide](docs/KEYCLOAK_SETUP.md)
- [Backend Documentation](api/README.md)
