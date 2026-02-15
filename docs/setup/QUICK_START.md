# Quick Start with Docker Compose

This guide shows how to quickly get the entire Crypto Pocket Butler stack running with Docker Compose.

## Prerequisites

- Docker 24.0+
- Docker Compose 2.20+

## Steps

### 1. Clone and Configure

```bash
# Clone the repository
git clone https://github.com/doitsu2014/crypto-pocket-butler.git
cd crypto-pocket-butler

# Copy environment file
cp .env.example .env

# Generate a secure NextAuth secret
sed -i "s/NEXTAUTH_SECRET=.*/NEXTAUTH_SECRET=$(openssl rand -base64 32)/" .env
```

### 2. Start All Services

```bash
docker compose up -d
```

This single command will:

1. **Start PostgreSQL** (port 5432)
   - Creates two databases: `crypto_pocket_butler` and `keycloak`
   - Data persisted in Docker volume

2. **Start Keycloak** (port 8080)
   - Uses PostgreSQL for persistent storage (production mode)
   - Admin credentials: `admin` / `admin`

3. **Configure Keycloak Automatically**
   - Creates realm: `myrealm`
   - Creates OAuth 2.0 client: `crypto-pocket-butler`
   - Generates and displays client secret
   - Creates test user: `testuser` / `testpass123`

4. **Start Rust API** (port 3000)
   - Runs database migrations automatically
   - JWT validation with Keycloak
   - Swagger UI available at `/swagger-ui`

5. **Start Next.js Web** (port 3001)
   - NextAuth.js with Keycloak OIDC
   - Authorization Code flow with PKCE

### 3. Save Client Secret

Check the keycloak-init logs for the generated client secret:

```bash
docker compose logs keycloak-init
```

Look for a line like:
```
Generated client secret: ABC123...
```

Add this to your `.env` file:
```bash
KEYCLOAK_CLIENT_SECRET=ABC123...
```

Then restart the web service:
```bash
docker compose restart web
```

### 4. Access the Application

- **Web App**: http://localhost:3001
- **API**: http://localhost:3000
- **Swagger UI**: http://localhost:3000/swagger-ui
- **Keycloak Admin**: http://localhost:8080 (admin/admin)

### 5. Test Authentication

1. Open http://localhost:3001
2. Click "Sign in"
3. Log in with `testuser` / `testpass123`
4. You should be redirected back to the app, authenticated

## What Just Happened?

### PostgreSQL Setup
- Two databases created: `crypto_pocket_butler` for the app, `keycloak` for auth
- Persistent storage in Docker volume `postgres_data`

### Keycloak Setup
- Running in **production mode** (not dev mode) with PostgreSQL
- **OAuth 2.0 Authorization Code Flow** with **PKCE**:
  - More secure than implicit flow
  - Proof Key for Code Exchange prevents authorization code interception
  - Standard flow for web applications

### Client Configuration
The automatically created client has:
- **Client Type**: Confidential (requires client secret)
- **Standard Flow**: Enabled (OAuth 2.0 Authorization Code)
- **PKCE**: Enabled with S256 code challenge
- **Redirect URIs**: Configured for local development
- **Web Origins**: Configured for CORS

## Troubleshooting

### Keycloak init failed
```bash
# Rebuild and re-run the initialization
docker compose build keycloak-init
docker compose run --rm keycloak-init
```

### Port conflicts
Edit `docker-compose.yml` to change ports:
```yaml
ports:
  - "5433:5432"  # PostgreSQL
  - "8081:8080"  # Keycloak
  - "3001:3000"  # API
  - "3002:3001"  # Web
```

### Clean start
```bash
# Stop and remove all containers and volumes
docker compose down -v

# Start fresh
docker compose up -d
```

### Check logs
```bash
# All services
docker compose logs -f

# Specific service
docker compose logs -f keycloak
docker compose logs -f api
docker compose logs -f web
```

## Next Steps

- Read the [Docker Setup Guide](./DOCKER_SETUP.md) for more details
- Check the [Keycloak README](../../keycloak/README.md) for OAuth 2.0 configuration
- Review the [Web Setup Guide](./WEB_SETUP.md) for frontend configuration
- Explore the [API Documentation](../api/api-overview.md)

## Production Deployment

⚠️ This setup is for local development. For production:

1. Use strong passwords (not `admin`/`admin`)
2. Enable HTTPS/TLS
3. Use proper secrets management
4. Configure resource limits
5. Set up monitoring and logging
6. Use external managed databases
7. Configure proper CORS and CSP policies

See [DOCKER_SETUP.md](./DOCKER_SETUP.md) for production considerations.
