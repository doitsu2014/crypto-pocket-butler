# Features

> Feature documentation for Crypto Pocket Butler web application.

---

## Portfolio Snapshots Chart

### Overview

The Portfolio Snapshots page displays a time series chart of portfolio value over time using Recharts.

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        Portfolio Snapshots                              │
│                  Historical portfolio value over time                   │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Filters:  [Last 30 days ▼]  [All types ▼]  [Refresh]                │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │ Portfolio Value Over Time                                         │ │
│  │                                                                   │ │
│  │  $100k ┤                                          ●               │ │
│  │        │                                     ●──●/                │ │
│  │   $90k ┤                             ●──●──/                     │ │
│  │        │                      ●──●──/                            │ │
│  │   $80k ┤               ●──●──/                                   │ │
│  │        │        ●──●──/                                          │ │
│  │   $70k ┤  ●──●──/                                                │ │
│  │        └─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────┬─────   │ │
│  │          Jan 1  Jan 5  Jan 10 Jan 15 Jan 20 Jan 25 Jan 30       │ │
│  │                                                                   │ │
│  │  [HOVER TOOLTIP]                                                 │ │
│  │  ┌──────────────────┐                                            │ │
│  │  │ Date: Jan 25, 2024│                                           │ │
│  │  │ Value: $95,432.21│                                            │ │
│  │  └──────────────────┘                                            │ │
│  └───────────────────────────────────────────────────────────────────┘ │
│                                                                         │
│  ┌───────────────────────────────────────────────────────────────────┐ │
│  │ Snapshot History (30)                                            │ │
│  ├─────────┬──────────┬───────────────┬────────────────────────────┤ │
│  │ Date    │ Type     │ Total Value   │ Created At                 │ │
│  ├─────────┼──────────┼───────────────┼────────────────────────────┤ │
│  │ Jan 30  │ [EOD]    │ $95,432.21    │ Jan 30, 2024 5:00 PM      │ │
│  │ Jan 29  │ [EOD]    │ $94,123.45    │ Jan 29, 2024 5:00 PM      │ │
│  │ Jan 28  │ [MANUAL] │ $93,876.54    │ Jan 28, 2024 2:30 PM      │ │
│  └─────────┴──────────┴───────────────┴────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────┘
```

### Features

| Feature | Description |
|---------|-------------|
| **Line Chart** | Portfolio value over time with smooth transitions |
| **Date Range Selector** | Last 7/30/90 days or All time |
| **Snapshot Type Filter** | EOD, Manual, Hourly, All |
| **Interactive Tooltip** | Shows date and value on hover |
| **History Table** | Detailed list of all snapshots |
| **Refresh Button** | Reload data on demand |

### Implementation

**Location:** `app/(auth)/portfolios/[id]/snapshots/page.tsx`

**Components Used:**
- `LineChart`, `Line`, `XAxis`, `YAxis` from Recharts
- `ResponsiveContainer` for responsive sizing
- `CustomTooltip` for styled hover tooltips

```tsx
import { 
  LineChart, Line, XAxis, YAxis, 
  CartesianGrid, Tooltip, ResponsiveContainer 
} from 'recharts';

<ResponsiveContainer width="100%" height={400}>
  <LineChart data={snapshots}>
    <CartesianGrid strokeDasharray="3 3" stroke="#4c1d95" />
    <XAxis dataKey="date" stroke="#c4b5fd" />
    <YAxis stroke="#c4b5fd" />
    <Tooltip content={<CustomTooltip />} />
    <Line 
      type="monotone" 
      dataKey="value" 
      stroke="#e879f9" 
      strokeWidth={2}
      dot={{ fill: '#e879f9', strokeWidth: 2 }}
    />
  </LineChart>
</ResponsiveContainer>
```

---

## API Integration

### Architecture

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

### Authentication Flow

1. User signs in via Keycloak (OIDC/PKCE flow)
2. NextAuth stores access token in secure HTTP-only cookie
3. Next.js API proxy extracts token from session
4. Proxy forwards request to API with `Authorization: Bearer <token>` header

### API Proxy Route

**Location:** `app/api/backend/[...path]/route.ts`

```tsx
import { getServerSession } from "next-auth";
import { authOptions } from "@/lib/auth";

export async function GET(request: Request, { params }: { params: { path: string[] } }) {
  const session = await getServerSession(authOptions);
  
  const backendUrl = `${process.env.API_URL}/${params.path.join("/")}`;
  
  const response = await fetch(backendUrl, {
    headers: {
      Authorization: `Bearer ${session?.accessToken}`,
      "Content-Type": "application/json",
    },
  });
  
  return response;
}
```

### Request Example

```tsx
// Client-side hook call
const { data: accounts } = useAccounts();

// This calls:
// GET /api/backend/v1/accounts
// Which proxies to:
// GET ${API_URL}/v1/accounts (with auth header)
```

---

## Account Management

### Features

- Create exchange accounts (OKX, etc.) with API credentials
- Create wallet accounts (EVM, Solana) with address
- Sync account holdings from exchange/wallet
- View holdings with current prices
- Enable/disable chains for wallet accounts

### Account Types

| Type | Required Fields | Sync Method |
|------|-----------------|-------------|
| Exchange | `exchange_name`, `api_key`, `api_secret` | API call to exchange |
| Wallet | `wallet_address`, `enabled_chains` | RPC calls to enabled chains |

---

## Portfolio Management

### Features

- Create multiple portfolios per user
- Set default portfolio
- Add/remove accounts to portfolio
- View aggregated holdings
- Track allocation vs target

### Portfolio Components

| Component | Location | Description |
|-----------|----------|-------------|
| PortfolioList | `components/portfolio/PortfolioList.tsx` | Grid of portfolio cards |
| PortfolioCard | `components/portfolio/PortfolioCard.tsx` | Single portfolio card |
| PortfolioDetail | `components/portfolio/PortfolioDetail.tsx` | Detailed view with holdings |
| HoldingsTable | `components/portfolio/HoldingsTable.tsx` | Holdings with weights |

---

## Data Flow Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                        Web Application                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │   Pages     │───▶│   Hooks     │───▶│ API Client  │        │
│  │  (React)    │    │ (TanStack)  │    │  (fetch)    │        │
│  └─────────────┘    └─────────────┘    └──────┬──────┘        │
│                                                │               │
│                                        ┌──────▼──────┐        │
│                                        │  API Proxy  │        │
│                                        │ (Next.js)   │        │
│                                        └──────┬──────┘        │
│                                                │               │
└────────────────────────────────────────────────┼───────────────┘
                                                 │
                                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                          Rust API                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │  Handlers   │───▶│   Domain    │───▶│  Entities   │        │
│  │   (Axum)    │    │  (DDD)      │    │  (SeaORM)   │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │ Connectors  │    │    Jobs     │    │   Cache     │        │
│  │ (External)  │    │  (Apalis)   │    │   (Redis)   │        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
└─────────────────────────────────────────────────────────────────┘
```