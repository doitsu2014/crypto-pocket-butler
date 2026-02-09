# Crypto Pocket Butler - Frontend

Next.js frontend with Keycloak authentication using OAuth 2.0 + PKCE flow.

## Features

- 🔐 **Secure Authentication**: Keycloak OIDC with Authorization Code + PKCE flow
- 🎨 **Modern UI**: Built with Next.js 16, React 19, and TailwindCSS 4
- 🛡️ **Token Management**: Secure token storage and automatic inclusion in API calls
- 📱 **Responsive Design**: Mobile-first design with TailwindCSS
- 🔄 **API Integration**: Seamless integration with Rust backend using Bearer tokens

## Prerequisites

- Node.js 20+ and npm
- Keycloak server (running and configured)
- Backend API (running on port 3000 by default)

## Setup

### 1. Install Dependencies

```bash
npm install
```

### 2. Configure Environment Variables

Copy the example environment file and configure it:

```bash
cp .env.example .env.local
```

Update the following variables in `.env.local`:

```env
# NextAuth Configuration
NEXTAUTH_URL=http://localhost:3001
NEXTAUTH_SECRET=your-secret-here-generate-with-openssl-rand-base64-32

# Keycloak Configuration
KEYCLOAK_CLIENT_ID=your-client-id
KEYCLOAK_CLIENT_SECRET=your-client-secret
KEYCLOAK_ISSUER=https://keycloak.example.com/realms/myrealm

# Backend API Configuration
NEXT_PUBLIC_BACKEND_URL=http://localhost:3000
```

#### Generate NEXTAUTH_SECRET

```bash
openssl rand -base64 32
```

### 3. Keycloak Client Configuration

In your Keycloak admin console, configure the client with:

1. **Client ID**: Use the value from `KEYCLOAK_CLIENT_ID`
2. **Client Protocol**: openid-connect
3. **Access Type**: confidential (or public if not using client secret)
4. **Valid Redirect URIs**: 
   - `http://localhost:3001/*`
   - `http://localhost:3001/api/auth/callback/keycloak`
5. **Web Origins**: `http://localhost:3001`
6. **Standard Flow Enabled**: ON
7. **Direct Access Grants Enabled**: OFF (we're using Authorization Code flow)

### 4. Run Development Server

```bash
npm run dev
```

The frontend will be available at http://localhost:3001

## Project Structure

```
frontend-react/
├── app/
│   ├── api/
│   │   ├── auth/[...nextauth]/   # NextAuth.js API routes
│   │   └── backend/              # Backend API proxy routes
│   ├── auth/
│   │   └── signin/               # Custom sign-in page
│   ├── dashboard/                # Protected dashboard page
│   ├── layout.tsx                # Root layout
│   └── page.tsx                  # Home page
├── components/
│   ├── SignOutButton.tsx         # Sign out component
│   └── UserInfo.tsx              # User information display
├── lib/
│   └── api-client.ts             # API client utilities
├── types/
│   └── next-auth.d.ts            # NextAuth type extensions
├── auth.ts                       # NextAuth configuration
└── middleware.ts                 # Route protection middleware
```

## Authentication Flow

1. **User visits protected route** (e.g., `/dashboard`)
2. **Middleware checks authentication** - redirects to sign-in if not authenticated
3. **User clicks "Sign in with Keycloak"**
4. **Authorization Code + PKCE flow**:
   - App generates code verifier and challenge
   - Redirects to Keycloak with challenge and `offline_access` scope
   - User authenticates in Keycloak
   - Keycloak redirects back with authorization code
   - App exchanges code + verifier for tokens (including refresh token)
5. **Tokens stored securely** in session (JWT strategy)
6. **Automatic token refresh**: When a token has lived more than 50% of its lifetime, it is automatically refreshed using the refresh token
7. **API calls include Bearer token** automatically via proxy routes

## Making API Calls

### Client-Side (via proxy)

```typescript
import { apiClient } from "@/lib/api-client";

// The token is handled automatically by the Next.js API route
const userInfo = await apiClient("/api/me");
```

### Server-Side (direct)

```typescript
import { auth } from "@/auth";
import { directBackendClient } from "@/lib/api-client";

const session = await auth();
const data = await directBackendClient("/api/protected", session.accessToken);
```

## Security Features

- ✅ Authorization Code flow with PKCE (no client secret exposed to browser)
- ✅ Tokens stored server-side in JWT session
- ✅ Access token never exposed to client-side JavaScript
- ✅ **Automatic token refresh**: Tokens are refreshed when they pass 50% of their lifetime
- ✅ **Refresh tokens**: `offline_access` scope enables long-lived refresh tokens
- ✅ CSRF protection via NextAuth.js
- ✅ Secure HTTP-only cookies
- ✅ Route protection via middleware

## Development

### Build for Production

```bash
npm run build
npm start
```

### Linting

```bash
npm run lint
```

## Troubleshooting

### "Invalid client or Invalid client credentials"

- Verify `KEYCLOAK_CLIENT_ID` and `KEYCLOAK_CLIENT_SECRET` are correct
- Check that the client exists in Keycloak
- Ensure client access type is set to "confidential"

### "Redirect URI mismatch"

- Add `http://localhost:3001/api/auth/callback/keycloak` to Valid Redirect URIs in Keycloak client settings
- Verify `NEXTAUTH_URL` matches your application URL

### "NEXTAUTH_SECRET not found"

- Generate a secret: `openssl rand -base64 32`
- Add it to `.env.local` as `NEXTAUTH_SECRET`

### Backend API calls fail with 401

- Check that backend is running and accessible
- Verify `NEXT_PUBLIC_BACKEND_URL` is correct
- Ensure Keycloak configuration matches between frontend and backend

## License

See the root project LICENSE file.
