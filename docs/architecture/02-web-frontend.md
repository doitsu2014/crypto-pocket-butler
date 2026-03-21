# Crypto Pocket Butler Web (Frontend) Architecture

## High-Level Web Architecture

```mermaid
graph TD
    A[Browser] -->|HTTPS| B[Load Balancer\nNGINX]
    B --> C[Next.js 16 App\nApp Router]
    B --> D[Static Assets]

    C -->|Server Components / SSR| E[API Server\nRust + Axum]
    C -->|Client Components| F[TanStack Query v5]

    F -->|In-memory cache| G[Query Cache\n30s stale time]

    C -->|Unified proxy| P[/api/backend/path\nNext.js Route Handler]
    P -->|Bearer token attached| E

    C --> I[.next/standalone]
    D --> I
```

---

## Frontend Technology Stack

```mermaid
mindmap
  root((Crypto Pocket Butler Frontend))
    Framework(Next.js 16.1)
      Routing(App Router)
      Rendering(SSR + Server Components)
      Proxy(Unified API proxy route)
    Styling(TailwindCSS 4)
      Utilities(utility-first)
      Themes(custom dark theme)
      Responsive(grid/flex)
    Charts(Recharts 3)
      Pie(Allocation pie)
      Bar(Allocation bar)
    Auth(NextAuth.js v5 beta)
      Providers(Keycloak OIDC)
      Strategy(JWT session)
      PKCE(Authorization Code + PKCE)
      Refresh(Proactive at 50% lifetime)
    State(React + TanStack Query v5)
      ServerState(query cache — in-memory)
      ClientState(React Context)
```

---

## Web Deployment Architecture

```mermaid
graph TB
    subgraph "Docker Network: crypto-network"
        LB[nginx:proxy\nPort 443]

        subgraph "Web Service"
            NEXT[Next.js 16\nstandalone output]
        end

        subgraph "API Service"
            API[Rust + Axum\napi:3000]
        end

        PG[postgres:5432]
        KC[keycloak:8080]
    end

    LB --> NEXT
    LB --> API
    API --> PG
    LB --> KC

    style NEXT fill:#e8f5e9,stroke:#4caf50
```

---

## Frontend Authentication Flow

```mermaid
sequenceDiagram
    participant Browser
    participant NextAuth as NextAuth.js v5
    participant Keycloak
    participant API

    Browser->>NextAuth: Attempt login (/auth/signin)
    NextAuth->>Keycloak: Redirect — Authorization Code + PKCE
    Keycloak->>Keycloak: Show login form
    Keycloak->>NextAuth: Authorization code (via redirect)

    NextAuth->>Keycloak: Exchange code for tokens
    Keycloak-->>NextAuth: access_token, id_token, refresh_token

    Note over NextAuth: Extract realm_access.roles from JWT payload
    NextAuth->>Browser: Encrypted httpOnly session cookie (JWT strategy)

    Browser->>Next.js: Request page data
    Next.js->>Next.js: /api/backend/[...path] proxy
    Note over Next.js: Reads session server-side, attaches Bearer token
    Next.js->>API: Request + Authorization: Bearer <access_token>
    API->>Keycloak: Verify token
    Keycloak-->>API: Token validated
    API-->>Next.js: 200 OK + JSON
    Next.js-->>Browser: Response (token never exposed to browser)
```

---

## Unified API Proxy

All frontend-to-backend communication goes through a single Next.js route handler.
The browser never holds or sends the access token directly.

```mermaid
graph LR
    C[Client Component\napiClient] -->|/api/backend/v1/path| P[Next.js Proxy\n/api/backend/path/route.ts]
    S[Server Component] -->|directBackendClient| API[Backend\nRust + Axum]
    P -->|Bearer token attached| API

    subgraph "web/lib/"
        AC[api-client.ts\napiClient / directBackendClient / ApiError]
        QC[query-client.ts\nQueryClient config + retry logic]
    end
```

**Request flow**:
`apiClient("/v1/accounts")` → `fetch("/api/backend/v1/accounts")` → proxy reads
session server-side → `fetch("http://api:3000/api/v1/accounts", { Authorization: Bearer ... })`

**Error types** (`ApiError`): `auth` (401/403) | `validation` (400/422) |
`server` (5xx) | `network` | `unknown`

---

## Token Refresh Strategy

```mermaid
sequenceDiagram
    participant Browser
    participant NextAuth
    participant Keycloak

    Note over NextAuth: On every request, JWT callback runs
    NextAuth->>NextAuth: Calculate token age vs lifetime
    alt Token age < 50% of lifetime
        NextAuth-->>Browser: Return existing token
    else Token age >= 50% of lifetime
        NextAuth->>Keycloak: POST /token (refresh_token grant)
        Keycloak-->>NextAuth: New access_token + refresh_token
        NextAuth-->>Browser: Updated session cookie
    end
```

Tokens are refreshed **proactively** at 50% of their lifetime — not on expiry —
to avoid mid-request failures. The `offline_access` scope ensures a refresh token
is always issued.

---

## App Router Structure

```mermaid
graph TD
    Root[app/layout.tsx\nSessionProvider + QueryClient + Toast]

    Root --> Page[app/page.tsx]
    Root --> Dashboard[app/dashboard/page.tsx]

    Root --> Portfolios[app/portfolios/page.tsx\nPortfolio list]
    Portfolios --> PDetail[app/portfolios/id/page.tsx\nDetail + holdings + allocation]
    PDetail --> PRec[recommendations/]
    PDetail --> PSnap[snapshots/]
    PDetail --> PSet[settings/]

    Root --> Accounts[app/accounts/page.tsx\nAccount list]
    Accounts --> ADetail[app/accounts/id/page.tsx\nDetail + sync]
    ADetail --> AEVMChains[evm-chains/]
    ADetail --> AEVMTokens[evm-tokens/]
    ADetail --> ASolana[solana-tokens/]

    Root --> Admin[app/admin/page.tsx\nrequires administrator role]
    Admin --> AdminEVMC[evm-chains/\nConfigure chains + RPC URLs]
    Admin --> AdminEVMT[evm-tokens/\nManage ERC-20 tokens]
    Admin --> AdminSol[solana-tokens/\nManage SPL tokens]

    Root --> Settings[app/settings/page.tsx]
    Root --> Auth[app/auth/signin/]
    Root --> APIProxy[app/api/backend/path/route.ts\nUnified proxy — all HTTP methods]
    Root --> AuthHandler[app/api/auth/\nNextAuth handlers]
```

---

## Component Structure

```mermaid
graph TD
    subgraph "web/components/"
        AL[AppLayout.tsx\nShared page shell]
        ES[EmptyState.tsx]
        EA[ErrorAlert.tsx]
        LD[Loading.tsx]
        SP[SessionProviderWrapper.tsx]
        SO[SignOutButton.tsx]
        TT[Toast.tsx]
        UI[UserInfo.tsx]

        subgraph "portfolio/"
            AB[AllocationBar.tsx]
            AP[AllocationPie.tsx]
            DB[DriftBadge.tsx]
            HT[HoldingsTable.tsx]
        end
    end

    subgraph "web/hooks/"
        HA[useAccounts.ts]
        HC[useChains.ts]
        HEC[useEvmChains.ts]
        HET[useEvmTokens.ts]
        HP[usePortfolios.ts]
        HR[useRecommendations.ts]
        HS[useSnapshots.ts]
        HST[useSolanaTokens.ts]
    end

    subgraph "web/contexts/"
        QCP[QueryClientProvider.tsx]
        TC[ToastContext.tsx]
    end

    subgraph "web/lib/"
        ACL[api-client.ts]
        QCL[query-client.ts]
    end
```

---

## State Management

```mermaid
graph LR
    subgraph "Server State — TanStack Query v5"
        RQ[useQuery / useMutation hooks]
        API[Backend API]
        Cache[In-memory Query Cache\n30s stale time\nrefetch on window focus]
        Retry[Smart retry\nno retry on auth/validation errors\n2 retries on network/server errors]
    end

    subgraph "Client State — React Context"
        Toast[ToastContext\nnotification queue]
        Session[SessionProviderWrapper\nNextAuth session]
    end

    RQ --> API
    RQ --> Cache
    RQ --> Retry
```

**There is no Zustand**. Client state is managed entirely through React Context.
Server/async state is managed by TanStack Query v5 with in-memory caching only —
no localStorage persistence for query data.

---

## Role-Based Access Control

```mermaid
graph TD
    User -->|Auth Code + PKCE| KC[Keycloak]
    KC -->|id_token + access_token| NA[NextAuth.js]
    NA -->|extracts realm_access.roles| Session

    Session --> MW[middleware.ts\nprotects /dashboard /portfolios /admin /api/backend]
    MW -->|unauthenticated| Redirect[/auth/signin]
    MW -->|authenticated| Routes[Protected Routes]

    Routes --> AdminCheck{roles includes\nadministrator?}
    AdminCheck -->|Yes| AdminPages[/admin/*\nEVM chains, tokens, Solana tokens]
    AdminCheck -->|No| Redirect2[/dashboard]
```

The required admin role claim is **`"administrator"`** (Keycloak realm role, extracted
from `realm_access.roles` in the JWT payload). Standard authenticated users cannot
access `/admin/*` routes — they are redirected to `/dashboard`.

---

## Frontend Error Handling

```mermaid
sequenceDiagram
    participant User
    participant Component
    participant apiClient
    participant Proxy as /api/backend proxy
    participant API

    User->>Component: Interact
    Component->>apiClient: apiClient<T>("/v1/...")
    apiClient->>Proxy: fetch(/api/backend/v1/...)
    Proxy->>API: Proxied request

    alt API error (4xx/5xx)
        API-->>Proxy: Error response
        Proxy-->>apiClient: JSON error body
        apiClient->>apiClient: parseErrorResponse → ApiError
        Note over apiClient: type: auth | validation | server | unknown
        apiClient-->>Component: throw ApiError
        Component->>Component: TanStack Query retries (if applicable)
        Component-->>User: ErrorAlert / Toast notification
    else Network failure
        apiClient-->>Component: throw ApiError(type: network)
        Component-->>User: Network error message
    end
```

---

## Frontend Deployment Flow

```mermaid
graph TB
    Dev[Developer Push] --> GitHub[GitHub Repo]
    GitHub --> Build[CI/CD Pipeline]

    Build --> Lint[ESLint + TypeCheck]
    Lint --> BuildNext[next build\nstandalone output]
    BuildNext --> Docker[Docker Image\nnext standalone]

    Docker --> LB[NGINX Load Balancer]
    LB --> User[Browser User]
```

Next.js is built with `output: 'standalone'` — the Docker image includes only the
production server files without `node_modules`, keeping the image size minimal.

---

## Frontend Development Task Patterns

This section guides agents generating `tasks.md` for frontend features. All task generation
MUST follow the constitution's task-driven workflow (`/speckit.tasks`).

### Path Conventions

| Task type | Location |
|-----------|----------|
| New page route | `web/app/<route>/page.tsx` |
| New route layout | `web/app/<route>/layout.tsx` |
| New route loading state | `web/app/<route>/loading.tsx` |
| New route error boundary | `web/app/<route>/error.tsx` |
| Shared UI component | `web/components/<ComponentName>.tsx` |
| Domain UI component | `web/components/<domain>/<ComponentName>.tsx` |
| Data-fetching hook | `web/hooks/use<Domain>.ts` |
| Context provider | `web/contexts/<Name>Context.tsx` |
| Type definitions | `web/types/<domain>.ts` |
| Low-level utility | `web/lib/<name>.ts` |

### Standard Task Phases for Frontend Features

#### Phase 1: Setup (shared infrastructure — if new feature needs it)

```
- [ ] T001 Create route directory web/app/<route>/
- [ ] T002 [P] Add TypeScript types for <Domain> in web/types/<domain>.ts
- [ ] T003 [P] Add API error handling for new endpoint in web/lib/api-client.ts (if new error type)
```

#### Phase 2: Foundational (blocking prerequisites)

```
- [ ] T004 Add data-fetching hook web/hooks/use<Domain>.ts
         (uses apiClient, wraps useQuery / useMutation)
- [ ] T005 [P] Add context provider if new shared state needed in web/contexts/<Name>Context.tsx
- [ ] T006 [P] Add middleware matcher entry in web/middleware.ts (if new protected route)
```

#### Phase 3+: User Story Implementation

```
# Page
- [ ] TXXX [USn] Create page component web/app/<route>/page.tsx
- [ ] TXXX [USn] Create layout (if needed) web/app/<route>/layout.tsx
- [ ] TXXX [USn] Create loading skeleton web/app/<route>/loading.tsx

# Components (can be parallel if different files)
- [ ] TXXX [P] [USn] Create <ComponentA> in web/components/<domain>/<ComponentA>.tsx
- [ ] TXXX [P] [USn] Create <ComponentB> in web/components/<domain>/<ComponentB>.tsx

# Integration
- [ ] TXXX [USn] Wire hook into page — replace static data with useQuery calls
- [ ] TXXX [USn] Add error state handling using ErrorAlert component
- [ ] TXXX [USn] Add empty state using EmptyState component
```

#### Polish Phase

```
- [ ] TXXX [P] Add loading states (Loading component or skeleton)
- [ ] TXXX [P] Add Toast notifications for mutation success/failure
- [ ] TXXX Update docs/architecture/02-web-frontend.md if structure changed
- [ ] TXXX TypeScript strict-mode check — no implicit `any`
- [ ] TXXX Run ESLint: pnpm lint
```

### Key Constraints for Task Generation

1. **No data fetching in components** — always delegate to a hook in `web/hooks/`.
2. **No direct backend calls** — all API calls via `apiClient()` through the proxy.
3. **No state libraries** — use React Context or TanStack Query only.
4. **Protected routes** — add new route paths to `web/middleware.ts` matcher in the same task
   that creates the route.
5. **Admin routes** — any route under `web/app/admin/` MUST include a role guard checking
   `session.roles?.includes("administrator")`.
6. **Mutation tasks** — use `useMutation` from TanStack Query + Toast notification on
   success/error. Never optimistically update without invalidating the relevant query.
7. **TypeScript** — all new files MUST be `.tsx` or `.ts`. No `.js` or `.jsx`.

### Dependency Order Within a User Story

```
Types → Hook → Component(s) → Page → Integration → Error/Loading states → Docs update
```

Components and types within a story can be created in parallel (mark `[P]`) since they
are in different files. The page integration task depends on the hook and components.
