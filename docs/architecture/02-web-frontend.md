# Crypto Pocket Butler Web (Frontend) Architecture

## High-Level Web Architecture

```mermaid
graph TD
    A[Browser] -->|HTTPS| B[Load Balancer<br/>NGINX]
    B --> C[Next.js 16 App<br/>app router]
    B --> D[Static Assets]
    
    C -->|SSR/SGR| E[API Server<br/>Rust + Axum]
    C -->|Client-side| F[TanStack Query]
    
    F -->|Cache| G[Memory Cache]
    F -->|Persistent| H[localStorage]
    
    C -->| builds | I[dist/ folder]
    D --> I
```

---

## Frontend Technology Stack

```mermaid
mindmap
  root((Crypto Pocket Butler Frontend))
    Framework(Next.js 16)
      Routing(app router)
      Rendering(SSR + Static)
      Data Fetching(TanStack Query)
    Styling(TailwindCSS 4)
      Utilities(utility-first)
      Themes(custom theme)
      Responsive(grid/flex)
    Auth(NextAuth.js v5)
      Providers(Keycloak OIDC)
      Sessions(managed sessions)
      PKCE(flow)
    State(React + Zustand)
      Client State(local)
      Server State(query cache)
    UI Components(Shadcn/UI)
      Primitive(primitives)
      Accessible(wai-aria)
```

---

## Web Deployment Architecture

```mermaid
graph TB
    subgraph "Docker Network: crypto-network"
        LB[nginx:proxy<br/>Port 443]
        
        subgraph "Web Service"
            NEXTCDN[Next.js static assets<br/>CDN origin]
        end
        
        subgraph "API Service"
            API[api:3001]
        end
        
        PG[postgres:5432]
        KC[keycloak:8080]
    end
    
    LB --> NEXTCDN
    LB --> API
    API --> PG
    LB --> KC
    
    style NEXTCDN fill:#e8f5e9,stroke:#4caf50
```

---

## Frontend Data Flow: Authentication

```mermaid
sequenceDiagram
    participant Browser
    participant NextAuth as NextAuth.js
    participant Keycloak
    participant API

    Browser->>NextAuth: Attempt login
    NextAuth->>Keycloak: Redirect to Keycloak
    Keycloak->>Keycloak: Show login form
    Keycloak->>NextAuth: Authorization code (via redirect)
    
    NextAuth->>Keycloak: Exchange code for tokens
    Keycloak-->>NextAuth: id_token, access_token, refresh_token
    
    NextAuth->>Browser: Store tokens (session)
    Browser->>API: Request with Bearer token
    API->>Keycloak: Verify token (Keycloak Auth Layer)
    Keycloak-->>API: Token validated
    
    API-->>Browser: 200 OK + data
```

---

## Frontend Component Structure

```mermaid
graph TD
    App[app/layout.tsx]
    
    App --> Layout[App Layout]
    Layout --> Header[Header Component]
    Layout --> Sidebar[Sidebar Navigation]
    
    App --> Page[app/page/page.tsx]
    Page --> PortfolioList[Portfolio List]
    Page --> PortfolioStats[Portfolio Stats]
    
    App --> Portfolio[app/portfolios/[id]/page.tsx]
    Portfolio --> PortfolioDetail[Portfolio Detail]
    Portfolio --> Holdings[Holdings Table]
    Portfolio --> Allocation[Allocation Chart]
    
    App --> Accounts[app/accounts/page.tsx]
    Accounts --> AccountList[Account List]
    Accounts --> AccountForm[Account Form]
    
    App --> Admin[app/admin/layout.tsx]
    Admin --> Dashboard[apalis-board Dashboard]
```

---

## Frontend State Management

```mermaid
graph LR
    subgraph "Server State"
        RQ[TanStack Query<br/>React Query]
        API[API Endpoints]
        Cache[Query Cache]
        Revalidate[Automatic Revalidation]
    end
    
    subgraph "Client State"
        Z[Zustand<br/>Store]
        L[localStorage]
    end
    
    subgraph "Context"
        Auth[Auth Context]
        Theme[Theme Context]
    end
    
    RQ --> API
    RQ --> Cache
    Cache --> Revalidate
    
    Z --> L
```

---

## Frontend Response Flow

```mermaid
sequenceDiagram
    participant User
    participant Browser
    participant Nextjs as Next.js App
    participant API
    participant Cache as TanStack Query Cache

    User->>Browser: Navigate to page
    Browser->>Nextjs: Render page component
    Nextjs->>Nextjs: UseQuery hook
    Nextjs->>API: Fetch data (GET /api/v1/...)
    
    alt Cache Hit
        API-->>Nextjs: 304 Not Modified
        Nextjs-->>Browser: Render from cache
    else Cache Miss
        API->>API: Process request
        API-->>Nextjs: 200 OK + JSON
        Nextjs->>Cache: Store in query cache
        Nextjs-->>Browser: Render with data
    end
    
    Cache-->>Nextjs: Auto-revalidate on focus
```

---

## Frontend Security Architecture

```mermaid
graph TD
    User[User] -->|Auth Code| KB[Keycloak Browser]
    KB -->|PKCE Flow| KC[Keycloak Server]
    KC -->|Tokens| NB[NextAuth.js]
    NB -->|Cookies| Browser
    
    Browser -->|Bearer Token| API[API Endpoint]
    API -->|Verify| KC
    
    subgraph "Protected Routes"
        Authenticated[Authenticated Layout]
        Admin[Admin Layout]
    end
    
    Authenticated -->|Role Check| RBAC[RoleGuard]
    Admin -->|Admin Role| RBAC
    
    RBAC -->| permet| AdminPages[Admin Pages]
    RBAC -->| Block| AdminPages
    
    style AdminPages fill:#ffccbc
    style RBAC fill:#e1bee7
```

---

## Frontend Deployment Flow (CI/CD)

```mermaid
graph TB
    Dev[Developer Push] --> GitHub[GitHub Repo]
    GitHub --> Build[CI/CD Pipeline]
    
    Build --> Test[Unit Tests]
    Test --> BuildNext[Build Next.js]
    BuildNext --> Lint[ESLint/TypeCheck]
    Lint --> Deploy[Deploy]
    
    Deploy --> CDN[Cloudflare/CDN]
    Deploy --> Docker[Docker Image]
    
    CDN --> LB[Load Balancer]
    Docker --> LB
    
    LB --> User[Browser User]
```

---

## Frontend Routing Strategy

```mermaid
graph TD
    App[app/layout.tsx]
    App --> Root[app/page.tsx<br/>Home/Portfolio]
    
    App --> Portfolios[app/portfolios/layout.tsx]
    Portfolios --> List[app/portfolios/page.tsx<br/>List]
    Portfolios --> Detail[app/portfolios/[id]/page.tsx<br/>Detail]
    
    App --> Accounts[app/accounts/layout.tsx]
    Accounts --> List[app/accounts/page.tsx<br/>List]
    Accounts --> Form[app/accounts/:id/page.tsx<br/>Form/Detail]
    
    App --> Chains[app/evm-chains/layout.tsx]
    Chains --> List[app/evm-chains/page.tsx<br/>List]
    
    App --> Admin[app/admin/layout.tsx]
    Admin --> Board[app/admin/board/page.tsx<br/>apalis-board]
```

---

## Frontend Error Boundary Flow

```mermaid
sequenceDiagram
    participant User
    participant Browser
    participant Nextjs
    participant API
    participant ErrorBoundary

    User->>Browser: Navigate to page
    Browser->>Nextjs: Render
    Nextjs->>API: Fetch data
    API-->>Nextjs: 500 Error
    
    Nextjs->>ErrorBoundary: Catch error
    ErrorBoundary->>ErrorBoundary: Log error
    ErrorBoundary->>Browser: Render fallback UI
    
    Browser->>User: Show error message
    User->>ErrorBoundary: Retry button
    ErrorBoundary->>API: Retry fetch
```