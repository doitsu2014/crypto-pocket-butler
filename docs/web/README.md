# Crypto Pocket Butler - Web Application

> Frontend documentation for the Crypto Pocket Butler web application.

## Quick Start

```bash
# Install dependencies
npm install

# Configure environment
cp .env.example .env.local
# Edit .env.local with your Keycloak settings

# Run development server
npm run dev
```

Open [http://localhost:3001](http://localhost:3001) with your browser.

---

## Tech Stack

| Package | Version | Purpose |
|---------|---------|---------|
| Next.js | 16.1.6 | React framework with App Router |
| React | 19.2.4 | UI library |
| TanStack Query | 5.90.21 | Data fetching and caching |
| TailwindCSS | 4.x | Styling |
| Recharts | 3.7.0 | Charts and visualizations |
| NextAuth | 5.0.0-beta | OIDC authentication |

---

## Architecture

### Request Flow

```
Web Component
    ↓
TanStack Query Hook (useAccounts, usePortfolios, etc.)
    ↓
apiClient() function (lib/api-client.ts)
    ↓
Next.js API Proxy (/api/backend/[...path])
    ↓
Rust API
```

### Key Benefits

1. **Security** - Access tokens never exposed to browser
2. **Consistency** - All requests follow the same pattern
3. **Type Safety** - Full TypeScript support
4. **Caching** - TanStack Query provides automatic caching
5. **Error Handling** - Centralized with ApiError

---

## Documentation

| Document | Description |
|----------|-------------|
| [01-design-system.md](./01-design-system.md) | Style guide, colors, typography, UI patterns |
| [02-components.md](./02-components.md) | Reusable components and hooks |
| [03-features.md](./03-features.md) | Feature documentation (snapshots, API integration) |

---

## Project Structure

```
web/
├── app/                    # Next.js App Router
│   ├── (auth)/            # Authenticated routes
│   │   ├── accounts/      # Account management
│   │   ├── portfolios/    # Portfolio management
│   │   └── snapshots/     # Snapshot charts
│   ├── api/               # API routes (proxy)
│   └── layout.tsx         # Root layout
├── components/            # Reusable UI components
│   ├── Toast.tsx
│   ├── Loading.tsx
│   └── ErrorAlert.tsx
├── contexts/              # React contexts
│   ├── ToastContext.tsx
│   └── QueryClientProvider.tsx
├── hooks/                 # TanStack Query hooks
│   ├── useAccounts.ts
│   ├── usePortfolios.ts
│   └── useSnapshots.ts
└── lib/                   # Utilities
    ├── api-client.ts
    └── query-client.ts
```

---

## Key Features

- 🔐 **Keycloak OIDC** authentication with PKCE
- 🔄 **Automatic token refresh** handled by NextAuth
- 🎨 **Cyberpunk neon theme** with TailwindCSS
- 🛡️ **Secure token management** (server-side only)
- 📱 **Responsive design** for all screen sizes
- 📊 **Interactive charts** with Recharts
- 🔌 **Unified API integration** with automatic auth