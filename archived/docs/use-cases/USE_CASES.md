# Use Cases & Workflows — Crypto Pocket Butler

> End-to-end workflows for all user roles and system automation.

---

## Table of Contents

1. [User Personas](#1-user-personas)
2. [Authentication Workflows](#2-authentication-workflows)
3. [Account Management Workflows](#3-account-management-workflows)
4. [Portfolio Workflows](#4-portfolio-workflows)
5. [Snapshot Workflows](#5-snapshot-workflows)
6. [Rebalancing Workflows](#6-rebalancing-workflows)
7. [Admin Workflows](#7-admin-workflows)
8. [Background System Workflows](#8-background-system-workflows)
9. [Data Flow Diagrams](#9-data-flow-diagrams)

---

## 1. User Personas

| Persona | Description | Primary Actions |
|---------|-------------|----------------|
| **Portfolio Owner** | End user managing their own crypto portfolio | Add accounts, create portfolios, view allocation, take snapshots, review recommendations |
| **Admin** | User with `admin` Keycloak role | All portfolio owner actions + configure EVM chains & tokens |
| **System (Jobs)** | Automated background process | Fetch market data, collect prices, create EOD snapshots |

---

## 2. Authentication Workflows

### 2.1 First-Time Login

```
1. User visits http://localhost:3001
2. Next.js checks session → no active session
3. Redirect to /auth/signin
4. User clicks "Sign In"
5. NextAuth.js initiates OIDC Authorization Code + PKCE flow
6. Browser redirects to Keycloak login page
7. User enters credentials (username + password)
8. Keycloak validates credentials, issues access_token + id_token
9. Browser returns to web app with auth code
10. NextAuth.js exchanges code for tokens
11. Session stored in httpOnly cookie
12. User redirected to /dashboard
13. API: first API call triggers get-or-create user in `users` table
      (keycloak_user_id mapped to internal UUID)
```

### 2.2 Returning User (Session Active)

```
1. User visits any page
2. Next.js reads session cookie → valid session found
3. Page renders with authenticated context
4. All API calls automatically include Bearer token via proxy
```

### 2.3 Token Expiry & Refresh

```
1. Access token expires (Keycloak default: 5 minutes)
2. NextAuth.js detects expiry via session check
3. NextAuth.js uses refresh_token to obtain new access_token
4. New token transparently attached to next API request
5. If refresh_token also expired → user redirected to /auth/signin
```

### 2.4 Sign Out

```
1. User clicks "Sign Out" button
2. NextAuth.js destroys session cookie
3. NextAuth.js calls Keycloak end-session endpoint
4. User redirected to /auth/signin
```

---

## 3. Account Management Workflows

### 3.1 Add an OKX Exchange Account

**Prerequisite**: OKX API keys with **read-only** permissions.

```
1. User navigates to /accounts → clicks "Add Account"
2. Selects account type: "Exchange" → exchange: "OKX"
3. Enters:
   - Name: "My OKX Main"
   - API Key, API Secret, Passphrase
4. Submits form
5. API: POST /api/accounts
   - Stores account with api_key_encrypted, api_secret_encrypted, passphrase_encrypted
   - account_type = "exchange", exchange_name = "okx"
6. Account appears in account list (holdings not yet synced)
7. User clicks "Sync" on the account
8. API: POST /api/accounts/{id}/sync
   → OkxConnector fetches spot balances (HMAC-SHA256 auth)
   → Holdings normalized and stored in accounts.holdings
   → last_synced_at updated
9. Account list shows updated holdings count and last sync time
```

### 3.2 Add an EVM Wallet

**Prerequisite**: Public wallet address (no private keys needed).

```
1. User navigates to /accounts → clicks "Add Account"
2. Selects account type: "Wallet"
3. Enters:
   - Name: "My ETH Wallet"
   - Wallet Address: 0x1234...
4. Selects enabled chains: [Ethereum, Arbitrum, Base]
5. Submits form
6. API: POST /api/accounts
   - account_type = "wallet"
   - wallet_address stored
   - enabled_chains stored as JSONB
7. User clicks "Sync"
8. API: POST /api/accounts/{id}/sync
   → EvmConnector queries each enabled chain via RPC
   → Fetches native coin balance + all ERC-20 balances from evm_tokens registry
   → Holdings normalized and merged across chains
   → Stored in accounts.holdings
9. Account shows multi-chain holdings
```

### 3.3 Sync All Accounts

```
1. User clicks "Sync All" on the accounts page
2. API: POST /api/accounts/sync-all
3. For each active account owned by user:
   - If exchange: calls appropriate connector (OKX, etc.)
   - If wallet: calls EVM connector for each enabled chain
4. Results returned:
   {
     "total": 3,
     "successful": 2,
     "failed": 1,
     "results": [
       { "account_id": "...", "success": true, "holdings_count": 7 },
       { "account_id": "...", "success": false, "error": "Auth failed" }
     ]
   }
5. Updated holdings immediately visible in UI
```

### 3.4 Edit or Delete an Account

```
Edit:
1. User clicks "Edit" on an account
2. Can update: name, API keys (re-encrypted), enabled chains
3. API: PUT /api/accounts/{id}
4. Account credentials updated, re-encrypted at rest

Delete:
1. User clicks "Delete" → confirms dialog
2. API: DELETE /api/accounts/{id}
3. Account removed; any portfolio_accounts join entries cascade-deleted
```

---

## 4. Portfolio Workflows

### 4.1 Create a Portfolio

```
1. User navigates to /portfolios → clicks "New Portfolio"
2. Enters:
   - Name: "Core Holdings"
   - Description: "BTC, ETH, and stablecoins"
3. Submits form
4. API: POST /api/portfolios
   - is_default = false (unless it's the user's first portfolio)
5. New empty portfolio appears in list
```

### 4.2 Link Accounts to a Portfolio

```
1. User opens a portfolio → Settings tab
2. Clicks "Add Account"
3. Selects from their existing accounts
4. API: POST /api/portfolios/{id}/accounts
   { "account_id": "..." }
5. account added to portfolio_accounts join table
6. Portfolio now includes that account's holdings in allocation calculations
```

### 4.3 Set Target Allocation

```
1. User opens a portfolio → Settings tab → "Target Allocation"
2. Enters target percentages per asset:
   - BTC: 40%
   - ETH: 30%
   - USDC: 20%
   - Other: 10%
3. Total must equal 100%
4. API: PUT /api/portfolios/{id}
   { "target_allocation": [{ "symbol": "BTC", "target_pct": 40 }, ...] }
5. Drift detection activated — portfolio now shows drift vs. target
```

### 4.4 View Portfolio Allocation

```
1. User navigates to /portfolios/{id}
2. API: GET /api/portfolios/{id}/allocation
3. Backend aggregates holdings from all linked accounts:
   a. Loads all accounts in portfolio_accounts for this portfolio
   b. Reads accounts.holdings (JSONB) for each account
   c. Merges holdings across accounts (same asset combined)
   d. Fetches latest prices from asset_prices for each asset
   e. Calculates USD value per asset
   f. Computes actual allocation % per asset
   g. Compares to target_allocation → drift %
4. UI renders:
   - AllocationPie chart (actual allocation)
   - HoldingsTable (asset, quantity, USD value, % of portfolio)
   - DriftBadge per asset (shows drift from target)
```

### 4.5 Construct Portfolio Allocation (Manual Refresh)

```
1. User clicks "Refresh Allocation"
2. API: POST /api/portfolios/{id}/construct-allocation
3. Same aggregation as 4.4 but writes result to portfolio_allocations table
4. last_constructed_at timestamp updated
5. UI shows refreshed allocation data
```

---

## 5. Snapshot Workflows

### 5.1 Create a Manual Snapshot

```
1. User navigates to /portfolios/{id}/snapshots → clicks "Take Snapshot"
2. API: POST /api/portfolios/{id}/snapshots
   {
     "snapshot_type": "manual",
     "snapshot_date": "2026-02-21"
   }
3. Backend:
   a. Aggregates holdings from all linked accounts
   b. Fetches current prices
   c. Calculates total_value_usd
   d. Stores snapshot in snapshots table
      (unique constraint prevents duplicate for same date+type)
4. Snapshot appears in history list with date, total value, holdings breakdown
```

### 5.2 Browse Snapshot History

```
1. User navigates to /portfolios/{id}/snapshots
2. API: GET /api/portfolios/{id}/snapshots
   (returns list ordered by snapshot_date desc)
3. UI renders table:
   - Date
   - Type (EOD / Manual / Hourly)
   - Total Value (USD)
   - Number of holdings
4. User clicks a snapshot to view the breakdown at that point in time
```

### 5.3 Automated EOD Snapshot (System)

```
Trigger: cron job at 11 PM UTC daily

1. Job runner calls create_all_portfolio_snapshots()
2. Queries all active portfolios across all users
3. For each portfolio:
   a. Aggregates current holdings from linked accounts
   b. Calculates total value in USD
   c. Stores snapshot: snapshot_type = "eod", snapshot_date = today
   d. ON CONFLICT (portfolio_id, snapshot_date, "eod") → updates existing
4. All portfolios have EOD snapshot for the day
5. Users can see today's EOD snapshot in history
```

---

## 6. Rebalancing Workflows

### 6.1 Generate Rebalancing Recommendation

```
1. User navigates to /portfolios/{id}/recommendations
2. Clicks "Generate Recommendation"
3. API: POST /api/portfolios/{id}/recommendations
4. Backend:
   a. Computes current allocation vs target_allocation
   b. Calculates drift per asset
   c. Applies guardrails (min trade size, max drift threshold)
   d. Generates suggested trades (buy/sell asset, amount, venue)
   e. Stores recommendation in recommendations table
      (status = "pending")
5. Recommendation displayed with:
   - Risk level
   - Rationale
   - Suggested trades list
   - Accept / Reject buttons
```

### 6.2 Review Recommendation History

```
1. User navigates to /portfolios/{id}/recommendations
2. API: GET /api/portfolios/{id}/recommendations
3. List of past recommendations with status:
   - pending: awaiting action
   - accepted: user accepted suggestion
   - rejected: user rejected suggestion
   - executed: trades were executed (future feature)
4. User can click any recommendation to see full details
```

---

## 7. Admin Workflows

> Admin routes require the `admin` Keycloak role.

### 7.1 Configure EVM Chains

```
1. Admin navigates to /admin/evm-chains
2. API: GET /api/admin/evm-chains
3. Shows list of configured chains with chain_name, chain_id, rpc_url, status

Add new chain:
4. Admin clicks "Add Chain"
5. Enters: chain_name, chain_id (e.g., 1 for Ethereum), rpc_url
6. API: POST /api/admin/evm-chains
7. New chain active and usable by EVM wallet sync

Update RPC URL:
4. Admin clicks "Edit" on a chain
5. Updates rpc_url (e.g., switch to private Alchemy endpoint)
6. API: PUT /api/admin/evm-chains/{id}
7. EVM connector uses new RPC URL on next sync
```

### 7.2 Manage EVM Token Registry

```
1. Admin navigates to /admin/evm-tokens
2. API: GET /api/admin/evm-tokens
3. Shows list of tracked ERC-20 tokens (symbol, contract, chain, status)

Add token:
4. Admin clicks "Add Token"
5. Selects chain, enters: symbol, contract_address, decimals
6. API: POST /api/admin/evm-tokens
7. Token now tracked during EVM wallet syncs

Lookup by symbol:
4. Admin clicks "Lookup Contracts"
5. Enters token symbol (e.g., "USDC")
6. API: POST /api/admin/evm-tokens/lookup-contracts { "symbol": "USDC" }
7. System queries CoinPaprika for contract addresses across chains
8. Admin reviews results and confirms which to import

Sync from contracts:
4. Admin clicks "Sync from Contracts"
5. API: POST /api/admin/evm-tokens/sync-from-contracts
6. System fetches contract data from CoinPaprika for all assets in DB
7. evm_tokens registry updated with any new contracts
```

---

## 8. Background System Workflows

### 8.1 Daily Coin Data Collection

```
Schedule: Daily at midnight UTC (0 0 0 * * *)
Job: fetch_all_coins

1. Fetch top coins list from CoinPaprika API
2. For each coin:
   a. Upsert into assets table (symbol, name, coingecko_id, asset_type)
   b. Fetch detailed coin info (contracts, platform info)
   c. Upsert contract addresses into asset_contracts table
3. Market reference data is up-to-date for asset resolution
4. Powers asset selection UI and balance normalization
```

### 8.2 Price Collection (Every 15 Minutes)

```
Schedule: Every 15 minutes (0 */15 * * * *)
Job: price_collection

1. Fetch top-N assets by market cap rank from assets table
2. For each asset batch:
   a. Fetch spot price from CoinGecko or CoinPaprika
   b. Include: price_usd, volume_24h, market_cap, rank, change_%
3. Upsert into asset_prices table (unique per asset+timestamp+source)
4. Latest prices available for portfolio valuation
```

### 8.3 EOD Portfolio Snapshots

```
Schedule: Daily at 11 PM UTC (0 0 23 * * *)
Job: portfolio_snapshot

1. Query all distinct user portfolios
2. For each portfolio:
   a. Load all linked account holdings
   b. Resolve assets using asset_identity helpers
   c. Fetch latest prices from asset_prices
   d. Calculate total_value_usd
   e. Write snapshot (ON CONFLICT update for same day)
3. All portfolios have end-of-day record
4. Historical performance tracking enabled
```

### 8.4 Manual Job Triggers (Admin/Developer)

Any background job can be triggered manually via API:

```bash
# Trigger coin fetch manually
curl -X POST http://localhost:3000/api/jobs/fetch-all-coins \
  -H "Authorization: Bearer <token>"

# Run pending DB migrations
curl -X POST http://localhost:3000/api/migrations \
  -H "Authorization: Bearer <token>"
```

---

## 9. Data Flow Diagrams

### Portfolio Valuation Flow

```
User requests /portfolios/{id}/allocation
  │
  ├─ Load portfolio from DB (with user_id ownership check)
  ├─ Load portfolio_accounts (linked account IDs)
  │
  ├─ For each account:
  │   ├─ Read accounts.holdings (JSONB — cached balances)
  │   └─ Merge into unified holdings map: { symbol → quantity }
  │
  ├─ For each unique symbol:
  │   ├─ Look up asset in assets table
  │   └─ Fetch latest price from asset_prices
  │       (fallback: use last known price)
  │
  ├─ Calculate:
  │   ├─ value_usd = quantity * price_usd (per asset)
  │   ├─ total_value_usd = sum(value_usd)
  │   ├─ actual_pct = value_usd / total_value_usd * 100
  │   └─ drift = actual_pct - target_pct (from target_allocation)
  │
  └─ Return AllocationResponse {
       total_value_usd,
       holdings: [{ symbol, quantity, value_usd, actual_pct, drift_pct }]
     }
```

### Account Sync Flow (OKX)

```
POST /api/accounts/{id}/sync
  │
  ├─ Load account (ownership check: account.user_id == jwt.sub)
  ├─ Decrypt: api_key, api_secret, passphrase
  │
  ├─ OkxConnector.get_balances()
  │   ├─ Build HMAC-SHA256 signature
  │   ├─ Call OKX REST API: GET /api/v5/account/balance
  │   └─ Parse response → Vec<RawBalance>
  │
  ├─ balance_normalization::normalize(raw_balances)
  │   ├─ Filter zero balances
  │   ├─ Standardize symbol names
  │   └─ → Vec<NormalizedHolding> { symbol, quantity, available, frozen }
  │
  ├─ asset_identity::resolve(normalized_holdings)
  │   ├─ Match each symbol to an asset in assets table
  │   └─ → Vec<Holding> { asset_id, symbol, quantity }
  │
  ├─ Write to accounts:
  │   ├─ holdings = Vec<Holding> (as JSONB)
  │   └─ last_synced_at = now()
  │
  └─ Return SyncResult { success, holdings_count }
```

### EVM Wallet Sync Flow

```
POST /api/accounts/{id}/sync (wallet type)
  │
  ├─ Load account (ownership check)
  ├─ Read: wallet_address, enabled_chains
  │
  ├─ For each enabled chain:
  │   ├─ Load chain config from evm_chains table (rpc_url)
  │   ├─ EvmConnector.get_native_balance(wallet_address, rpc_url)
  │   │   └─ RPC call: eth_getBalance
  │   │
  │   ├─ Load active evm_tokens for this chain
  │   └─ For each token:
  │       └─ EvmConnector.get_token_balance(wallet, contract_address, rpc_url)
  │           └─ RPC call: eth_call → balanceOf(address)
  │
  ├─ Merge holdings across all chains
  │   (same symbol on multiple chains → sum quantities)
  │
  ├─ Normalize quantities (divide by 10^decimals)
  │
  └─ Write to accounts.holdings + last_synced_at
```

---

*For system architecture see [../architecture/ARCHITECTURE.md](../architecture/ARCHITECTURE.md).*
*For coding patterns see [../coding-guidelines/CODING_GUIDELINES.md](../coding-guidelines/CODING_GUIDELINES.md).*
