# Keycloak Configuration

This directory contains Keycloak initialization scripts for automatic setup of the OAuth 2.0 authentication system.

## Overview

The Keycloak setup includes:

1. **PostgreSQL Integration**: Keycloak uses PostgreSQL for persistent storage instead of an embedded database
2. **Automatic Realm Creation**: Creates the configured realm (default: `myrealm`)
3. **OAuth 2.0 Client Setup**: Automatically configures a client with Authorization Code flow and PKCE
4. **Test User Creation**: Creates a test user for development purposes

## Files

- `Dockerfile`: Container image for running the initialization script
- `init-keycloak.sh`: Bash script that configures Keycloak via REST API
- `init-db.sh`: PostgreSQL initialization script to create the Keycloak database

## Configuration

The initialization script uses the following environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| `KEYCLOAK_REALM` | Realm name to create | `myrealm` |
| `KEYCLOAK_CLIENT_ID` | OAuth 2.0 client ID | `crypto-pocket-butler` |
| `KEYCLOAK_CLIENT_SECRET` | Client secret (generated if empty) | (empty) |
| `WEB_ROOT_URL` | Web application URL for redirects | `http://localhost:3001` |

## OAuth 2.0 Client Configuration

The automatically created client is configured with:

- **Client Type**: Confidential (requires client secret)
- **Authentication Flow**: Authorization Code with PKCE
- **Standard Flow**: Enabled (OAuth 2.0 Authorization Code Flow)
- **Direct Access Grants**: Enabled (for testing purposes)
- **Implicit Flow**: Disabled (deprecated and insecure)
- **Service Accounts**: Disabled (not needed for web app)

### Redirect URIs

The following redirect URIs are automatically configured:

- `${WEB_ROOT_URL}/*` - All web application URLs
- `${WEB_ROOT_URL}/api/auth/callback/keycloak` - NextAuth.js callback

### PKCE Support

The client is configured with PKCE (Proof Key for Code Exchange) support:
- Code challenge method: S256 (SHA-256)
- This provides additional security for the authorization code flow

## Test User

A test user is automatically created with:

- **Username**: `testuser`
- **Password**: `testpass123`
- **Email**: `test@example.com`
- **First Name**: Test
- **Last Name**: User

## Usage

The initialization script runs automatically when you start the Docker Compose stack:

```bash
docker-compose up -d
```

The script will:

1. Wait for Keycloak to be healthy
2. Obtain an admin access token
3. Check if realm exists, create if not
4. Check if client exists, create if not
5. Check if test user exists, create if not
6. Display configuration summary

## Manual Run

If you need to run the initialization script manually:

```bash
# Build the init image
docker build -t keycloak-init ./keycloak

# Run the initialization
docker run --rm \
  --network crypto-pocket-butler-network \
  -e KEYCLOAK_REALM=myrealm \
  -e KEYCLOAK_CLIENT_ID=crypto-pocket-butler \
  -e WEB_ROOT_URL=http://localhost:3001 \
  keycloak-init
```

## Client Secret

If no `KEYCLOAK_CLIENT_SECRET` is provided, the script will:

1. Generate a secure random secret using `openssl rand -base64 32`
2. Display the secret in the logs
3. Use that secret when creating the client

**Important**: Save the generated client secret in your `.env` file as `KEYCLOAK_CLIENT_SECRET` for the web service to use.

## Troubleshooting

### Script fails with "Keycloak is not ready"

- Check if Keycloak container is running: `docker-compose ps keycloak`
- Check Keycloak logs: `docker-compose logs keycloak`
- Ensure PostgreSQL is healthy: `docker-compose ps postgres`

### Client already exists

The script is idempotent - it checks if resources exist before creating them. If you need to recreate:

1. Delete the realm in Keycloak admin console
2. Run the script again: `docker-compose up keycloak-init --force-recreate`

### "Failed to get admin token"

- Verify Keycloak admin credentials (default: admin/admin)
- Check if Keycloak is accessible on port 8080

## Security Notes

⚠️ **Development Only**

This setup is designed for local development. For production:

1. Use strong admin credentials (not admin/admin)
2. Use HTTPS for all URLs
3. Store secrets securely (not in .env files)
4. Disable test user creation
5. Enable additional security features (brute force protection, etc.)
6. Use proper certificate validation
7. Consider using Keycloak's realm export/import for production setup

## Reference

- [Keycloak Admin REST API](https://www.keycloak.org/docs-api/latest/rest-api/)
- [OAuth 2.0 Authorization Code Flow](https://oauth.net/2/grant-types/authorization-code/)
- [PKCE (RFC 7636)](https://oauth.net/2/pkce/)
