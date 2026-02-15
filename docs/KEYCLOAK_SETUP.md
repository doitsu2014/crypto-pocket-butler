# Keycloak Setup Guide

This guide will help you set up Keycloak for local development with the Crypto Pocket Butler application.

## Option 1: Local Keycloak with Docker

### 1. Start Keycloak

```bash
docker run -d \
  --name keycloak \
  -p 8080:8080 \
  -e KEYCLOAK_ADMIN=admin \
  -e KEYCLOAK_ADMIN_PASSWORD=admin \
  quay.io/keycloak/keycloak:latest \
  start-dev
```

### 2. Access Keycloak Admin Console

Open http://localhost:8080 and log in with:
- Username: `admin`
- Password: `admin`

### 3. Create a Realm

1. Click on the realm dropdown (top-left, next to "Master")
2. Click "Create Realm"
3. Name: `myrealm`
4. Click "Create"

### 4. Create a Client for Frontend

1. Go to **Clients** → Click "Create client"
2. **General Settings:**
   - Client type: `OpenID Connect`
   - Client ID: `crypto-pocket-butler`
   - Click "Next"

3. **Capability config:**
   - Client authentication: `ON` (for confidential client)
   - Authorization: `OFF`
   - Authentication flow:
     - ✅ Standard flow
     - ✅ Direct access grants (optional)
     - ❌ Implicit flow
     - ❌ Service accounts roles
   - Click "Next"

4. **Login settings:**
   - Root URL: `http://localhost:3001`
   - Home URL: `http://localhost:3001`
   - Valid redirect URIs:
     - `http://localhost:3001/*`
     - `http://localhost:3001/api/auth/callback/keycloak`
   - Valid post logout redirect URIs: `http://localhost:3001/*`
   - Web origins: `http://localhost:3001`
   - Click "Save"

5. **Get Client Secret:**
   - Go to the "Credentials" tab
   - Copy the "Client secret" value

### 5. Create a Client for Backend (Optional)

If you want the backend to validate tokens from a different audience:

1. Create another client with ID matching `KEYCLOAK_AUDIENCE` from backend config
2. Or use the same client ID for both frontend and backend

### 6. Create a Test User

1. Go to **Users** → Click "Add user"
2. Fill in:
   - Username: `testuser`
   - Email: `test@example.com`
   - Email verified: `ON`
   - First name: `Test`
   - Last name: `User`
3. Click "Create"
4. Go to **Credentials** tab
5. Click "Set password"
   - Password: `testpass123`
   - Temporary: `OFF`
6. Click "Save"

### 7. Configure Frontend Environment

Update `web/.env.local`:

```env
NEXTAUTH_URL=http://localhost:3001
NEXTAUTH_SECRET=your-generated-secret

KEYCLOAK_CLIENT_ID=crypto-pocket-butler
KEYCLOAK_CLIENT_SECRET=<paste-client-secret-from-step-4>
KEYCLOAK_ISSUER=http://localhost:8080/realms/myrealm

NEXT_PUBLIC_BACKEND_URL=http://localhost:3000
```

### 8. Configure Backend Environment

Create `api/.env`:

```env
KEYCLOAK_SERVER=http://localhost:8080
KEYCLOAK_REALM=myrealm
KEYCLOAK_AUDIENCE=crypto-pocket-butler
```

## Option 2: Existing Keycloak Server

If you have an existing Keycloak server:

1. Create a realm (or use existing)
2. Create a client following steps 4-5 above
3. Update the URLs to match your Keycloak server URL
4. Configure frontend and backend with your server's URL

## Testing the Setup

### 1. Start Backend

```bash
cd api
cargo run
```

Backend will be available at http://localhost:3000

### 2. Start Frontend

```bash
cd web
npm run dev
```

Frontend will be available at http://localhost:3001

### 3. Test Authentication Flow

1. Open http://localhost:3001
2. Click "Get Started" or "View Dashboard"
3. You'll be redirected to sign in
4. Click "Sign in with Keycloak"
5. Log in with your test user credentials
6. You should be redirected back to the dashboard
7. The dashboard should display user information fetched from the backend

## Troubleshooting

### "Invalid redirect URI"
- Ensure all redirect URIs are added to the client configuration
- Check that the URLs match exactly (including protocol and port)

### "Client authentication failed"
- Verify the client secret is correct
- Ensure client authentication is enabled in Keycloak

### Backend returns 401 Unauthorized
- Check that backend and frontend are using the same Keycloak realm
- Verify the audience claim matches between frontend and backend
- Ensure the token is being sent in the Authorization header

### CORS errors
- Add `http://localhost:3001` to Web origins in Keycloak client settings
- Restart the backend after changing CORS settings

## Security Notes

⚠️ **For Development Only**

The setup above is for local development. For production:

1. Use HTTPS for all URLs
2. Generate a strong `NEXTAUTH_SECRET` with `openssl rand -base64 32`
3. Use proper domain names instead of localhost
4. Enable additional Keycloak security features (rate limiting, brute force detection)
5. Use a proper database for Keycloak (not dev mode)
6. Set up proper certificate validation
7. Consider using a reverse proxy (nginx, Traefik) for TLS termination
