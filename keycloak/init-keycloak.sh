#!/bin/bash
set -e

echo "Waiting for Keycloak to be ready..."
until curl -sf http://keycloak:8080/realms/master > /dev/null; do
  echo "Keycloak is not ready yet, waiting..."
  sleep 5
done

echo "Keycloak is ready! Starting configuration..."

# Get admin access token
echo "Getting admin access token..."
ADMIN_TOKEN=$(curl -s -X POST "http://keycloak:8080/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin" \
  -d "password=admin" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r '.access_token')

if [ -z "$ADMIN_TOKEN" ] || [ "$ADMIN_TOKEN" == "null" ]; then
  echo "Failed to get admin token"
  exit 1
fi

echo "Admin token obtained successfully"

# Check if realm already exists
REALM_EXISTS=$(curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://keycloak:8080/admin/realms/${KEYCLOAK_REALM}")

if [ "$REALM_EXISTS" -eq 200 ]; then
  echo "Realm '${KEYCLOAK_REALM}' already exists, skipping creation"
else
  echo "Creating realm '${KEYCLOAK_REALM}'..."
  curl -s -X POST "http://keycloak:8080/admin/realms" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"realm\": \"${KEYCLOAK_REALM}\",
      \"enabled\": true,
      \"displayName\": \"Crypto Pocket Butler\",
      \"loginTheme\": \"keycloak\",
      \"registrationAllowed\": false,
      \"resetPasswordAllowed\": true,
      \"rememberMe\": true,
      \"verifyEmail\": false,
      \"loginWithEmailAllowed\": true,
      \"duplicateEmailsAllowed\": false,
      \"sslRequired\": \"none\",
      \"accessTokenLifespan\": 3600,
      \"accessTokenLifespanForImplicitFlow\": 900,
      \"ssoSessionIdleTimeout\": 1800,
      \"ssoSessionMaxLifespan\": 36000,
      \"offlineSessionIdleTimeout\": 2592000,
      \"offlineSessionMaxLifespan\": 5184000
    }"
  echo "Realm '${KEYCLOAK_REALM}' created successfully"
fi

# Get updated admin token for the new realm operations
ADMIN_TOKEN=$(curl -s -X POST "http://keycloak:8080/realms/master/protocol/openid-connect/token" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "username=admin" \
  -d "password=admin" \
  -d "grant_type=password" \
  -d "client_id=admin-cli" | jq -r '.access_token')

# Check if client already exists
CLIENT_EXISTS=$(curl -s \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://keycloak:8080/admin/realms/${KEYCLOAK_REALM}/clients" | jq -r ".[] | select(.clientId==\"${KEYCLOAK_CLIENT_ID}\") | .id")

if [ -n "$CLIENT_EXISTS" ]; then
  echo "Client '${KEYCLOAK_CLIENT_ID}' already exists, skipping creation"
else
  echo "Creating client '${KEYCLOAK_CLIENT_ID}'..."
  
  # Generate client secret if not provided
  if [ -z "$KEYCLOAK_CLIENT_SECRET" ]; then
    KEYCLOAK_CLIENT_SECRET=$(openssl rand -base64 32)
    echo "Generated client secret: $KEYCLOAK_CLIENT_SECRET"
    echo "IMPORTANT: Save this client secret in your .env file as KEYCLOAK_CLIENT_SECRET"
  fi
  
  curl -s -X POST "http://keycloak:8080/admin/realms/${KEYCLOAK_REALM}/clients" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"clientId\": \"${KEYCLOAK_CLIENT_ID}\",
      \"name\": \"Crypto Pocket Butler Web\",
      \"description\": \"Web application client for Crypto Pocket Butler\",
      \"enabled\": true,
      \"protocol\": \"openid-connect\",
      \"publicClient\": false,
      \"directAccessGrantsEnabled\": true,
      \"standardFlowEnabled\": true,
      \"implicitFlowEnabled\": false,
      \"serviceAccountsEnabled\": false,
      \"authorizationServicesEnabled\": false,
      \"bearerOnly\": false,
      \"consentRequired\": false,
      \"fullScopeAllowed\": true,
      \"rootUrl\": \"${WEB_ROOT_URL}\",
      \"baseUrl\": \"${WEB_ROOT_URL}\",
      \"redirectUris\": [
        \"${WEB_ROOT_URL}/*\",
        \"${WEB_ROOT_URL}/api/auth/callback/keycloak\"
      ],
      \"webOrigins\": [
        \"${WEB_ROOT_URL}\"
      ],
      \"attributes\": {
        \"pkce.code.challenge.method\": \"S256\"
      },
      \"secret\": \"${KEYCLOAK_CLIENT_SECRET}\"
    }"
  echo "Client '${KEYCLOAK_CLIENT_ID}' created successfully with OAuth 2.0 Authorization Code Flow"
fi

# Check if test user already exists
USER_EXISTS=$(curl -s \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  "http://keycloak:8080/admin/realms/${KEYCLOAK_REALM}/users?username=testuser" | jq -r '.[0].id')

if [ -n "$USER_EXISTS" ] && [ "$USER_EXISTS" != "null" ]; then
  echo "Test user already exists, skipping creation"
else
  echo "Creating test user..."
  USER_ID=$(curl -s -X POST "http://keycloak:8080/admin/realms/${KEYCLOAK_REALM}/users" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -H "Content-Type: application/json" \
    -d '{
      "username": "testuser",
      "email": "test@example.com",
      "emailVerified": true,
      "firstName": "Test",
      "lastName": "User",
      "enabled": true,
      "credentials": [{
        "type": "password",
        "value": "testpass123",
        "temporary": false
      }]
    }' -i | grep -oP 'location: .*/users/\K[a-f0-9-]+' || echo "")
  
  if [ -n "$USER_ID" ]; then
    echo "Test user 'testuser' created successfully with password 'testpass123'"
  else
    echo "Test user may already exist or there was an error"
  fi
fi

echo ""
echo "=========================================="
echo "Keycloak configuration completed!"
echo "=========================================="
echo "Realm: ${KEYCLOAK_REALM}"
echo "Client ID: ${KEYCLOAK_CLIENT_ID}"
echo "Test User: testuser / testpass123"
echo "Admin Console: http://keycloak:8080"
echo "=========================================="
