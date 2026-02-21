# Coding Guidelines — Crypto Pocket Butler

> Project-specific conventions for Rust (backend) and TypeScript/React (frontend).
> Follow these patterns when adding features or fixing bugs.

---

## Table of Contents

1. [General Principles](#1-general-principles)
2. [Project Naming Conventions](#2-project-naming-conventions)
3. [Rust Backend Conventions](#3-rust-backend-conventions)
4. [TypeScript / React Frontend Conventions](#4-typescript--react-frontend-conventions)
5. [UI / Design System Rules](#5-ui--design-system-rules)
6. [Database & Migration Conventions](#6-database--migration-conventions)
7. [API Design Conventions](#7-api-design-conventions)
8. [Testing](#8-testing)

---

## 1. General Principles

- **No over-engineering.** Implement exactly what is needed. Do not add abstractions for hypothetical future requirements.
- **Minimal diff.** Do not reformat, add comments, or refactor surrounding code when fixing a targeted issue.
- **Security first.** Validate all user input at system boundaries. Never log secrets, API keys, or credentials.
- **Ownership at the resource level.** Every handler that operates on a user-owned resource must verify `user_id` ownership before performing any write.
- **Async throughout.** Both the API and frontend are fully async. Never use blocking calls inside async contexts.

---

## 2. Project Naming Conventions

### Service Directories

| Directory | Technology | Docker Service | Container Name |
|-----------|-----------|---------------|---------------|
| `api/` | Rust/Axum | `api` | `crypto-pocket-butler-api` |
| `web/` | Next.js | `web` | `crypto-pocket-butler-web` |

### Database

- **Tables**: `snake_case`, plural (e.g., `portfolio_accounts`, `asset_prices`)
- **Columns**: `snake_case` (e.g., `keycloak_user_id`, `last_synced_at`)
- **Primary keys**: always `id UUID`
- **Foreign keys**: `{entity}_id` (e.g., `portfolio_id`, `user_id`)
- **Timestamps**: `created_at TIMESTAMPTZ`, `updated_at TIMESTAMPTZ`
- **Boolean columns**: `is_{state}` prefix (e.g., `is_active`, `is_default`, `is_verified`)
- **JSON/JSONB columns**: use `JSONB` for structured data queried by the app, `JSON` for opaque blobs

### Rust

- **Modules**: `snake_case` matching file names
- **Types / Structs / Enums**: `PascalCase`
- **Functions / Variables**: `snake_case`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Error types**: named `{Domain}Error` (e.g., `AccountSyncError`)

### TypeScript

- **Files**: `PascalCase` for components (`AllocationPie.tsx`), `camelCase` for utilities (`api-client.ts`, `useAccounts.ts`)
- **Components**: `PascalCase`
- **Hooks**: `use` prefix + `PascalCase` (e.g., `useAccounts`, `useCreatePortfolio`)
- **Types / Interfaces**: `PascalCase` (e.g., `Portfolio`, `AccountSyncResult`)
- **Variables / Functions**: `camelCase`

---

## 3. Rust Backend Conventions

### Error Handling

Use `thiserror` for domain errors. Each domain module defines its own error type:

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AccountSyncError {
    #[error("Account not found: {0}")]
    NotFound(Uuid),
    #[error("Exchange authentication failed")]
    AuthFailed,
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),
}
```

In handlers, convert domain errors to HTTP responses via the `error.rs` module:

```rust
// handlers/error.rs defines AppError which implements IntoResponse
pub enum AppError {
    NotFound(String),
    Unauthorized,
    BadRequest(String),
    InternalError(String),
}
```

**Never** use `.unwrap()` or `.expect()` in handler or production code paths. Reserve them for test setup only.

### Async Handlers

All handlers are async and receive state via Axum extractors:

```rust
pub async fn get_portfolio(
    Extension(token): Extension<KeycloakToken<String>>,
    State(state): State<AppState>,
    Path(portfolio_id): Path<Uuid>,
) -> Result<Json<Portfolio>, AppError> {
    let user_id = get_or_create_user(&state.db, &token).await?;
    // Always verify ownership before proceeding
    let portfolio = portfolios::Entity::find_by_id(portfolio_id)
        .filter(portfolios::Column::UserId.eq(user_id))
        .one(&state.db)
        .await?
        .ok_or(AppError::NotFound("Portfolio not found".into()))?;
    Ok(Json(portfolio.into()))
}
```

**Ownership check pattern**: always filter by both resource ID **and** `user_id` in the same query.

### SeaORM Patterns

```rust
// Query with ownership check (preferred)
let account = accounts::Entity::find_by_id(account_id)
    .filter(accounts::Column::UserId.eq(user_id))
    .one(&db)
    .await?
    .ok_or(AppError::NotFound("Account not found".into()))?;

// Insert with conflict handling
let result = entity::Entity::insert(active_model)
    .on_conflict(
        OnConflict::columns([Col::PortfolioId, Col::SnapshotDate, Col::SnapshotType])
            .update_columns([Col::Holdings, Col::TotalValueUsd])
            .to_owned(),
    )
    .exec(&db)
    .await?;

// Batch inserts (prefer over individual inserts in loops)
entity::Entity::insert_many(active_models)
    .exec(&db)
    .await?;
```

### OpenAPI Documentation

All public handlers must be documented with `utoipa` macros:

```rust
#[utoipa::path(
    get,
    path = "/api/portfolios/{id}",
    params(
        ("id" = Uuid, Path, description = "Portfolio ID")
    ),
    responses(
        (status = 200, description = "Portfolio details", body = PortfolioResponse),
        (status = 401, description = "Unauthorized"),
        (status = 404, description = "Not found"),
    ),
    security(("bearer_auth" = []))
)]
pub async fn get_portfolio(...) { ... }
```

All response/request types used in handlers must derive `#[derive(ToSchema)]`.

### Connectors

External API clients live in `connectors/`. Each connector:
- Is a plain struct with an `impl` block
- Owns its HTTP client (`reqwest::Client`) and configuration
- Returns typed `Result<T, ConnectorError>` — never raw JSON values
- Never panics on API errors — always propagates `Err`

```rust
// Pattern: connector struct + typed result
pub struct OkxConnector {
    client: reqwest::Client,
    api_key: String,
    // ...
}

impl OkxConnector {
    pub async fn get_balances(&self) -> Result<Vec<Balance>, OkxError> {
        // ...
    }
}
```

### Decimal Arithmetic

**Always** use `rust_decimal::Decimal` for monetary amounts and prices. Never use `f64` for financial calculations:

```rust
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

let total: Decimal = holdings.iter()
    .map(|h| h.quantity * h.price_usd)
    .sum();
```

### Logging

Use `tracing` macros. Include relevant context as structured fields:

```rust
tracing::info!(
    account_id = %account.id,
    holdings_count = holdings.len(),
    "Account sync completed"
);

tracing::error!(
    account_id = %account_id,
    error = %e,
    "Account sync failed"
);
```

---

## 4. TypeScript / React Frontend Conventions

### Data Fetching — Always Use Custom Hooks

All API calls must go through custom TanStack Query hooks. **Never** call `fetch` or `axios` directly from components.

```typescript
// ✅ Correct
import { useAccounts, useCreateAccount } from "@/hooks/useAccounts";

function AccountsPage() {
  const { data: accounts, isLoading, error } = useAccounts();
  const createAccount = useCreateAccount();
  // ...
}

// ❌ Wrong — direct fetch in component
async function fetchAccounts() {
  const res = await fetch('/api/backend/accounts');
  // ...
}
```

### Hook Conventions

Custom hooks live in `web/hooks/`. Each domain has its own file:

```typescript
// Pattern: query hook + mutation hooks in same file
export function usePortfolios() {
  return useQuery({
    queryKey: ['portfolios'],
    queryFn: () => apiClient.get<Portfolio[]>('/portfolios'),
  });
}

export function useCreatePortfolio() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: (data: CreatePortfolioRequest) =>
      apiClient.post<Portfolio>('/portfolios', data),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['portfolios'] });
    },
  });
}
```

**Query key convention**: always use `[entity]` for list queries, `[entity, id]` for detail queries.

### API Client

All requests go through `web/lib/api-client.ts`. The proxy route at `/api/backend/[...path]` automatically attaches the Bearer token:

```typescript
import { apiClient } from '@/lib/api-client';

// GET request
const portfolios = await apiClient.get<Portfolio[]>('/portfolios');

// POST request
const portfolio = await apiClient.post<Portfolio>('/portfolios', { name: 'My Portfolio' });

// DELETE request
await apiClient.delete(`/portfolios/${id}`);
```

### Component Conventions

```typescript
// 1. Props interface defined at top of file
interface AllocationPieProps {
  holdings: Holding[];
  totalValueUsd: number;
}

// 2. Named export (not default for components in components/)
export function AllocationPie({ holdings, totalValueUsd }: AllocationPieProps) {
  // 3. Early return for loading/error states
  if (!holdings.length) return <EmptyState message="No holdings" />;

  return (
    // JSX
  );
}
```

### State Management

- **Server state**: TanStack Query (all API data)
- **UI state**: `useState` local to the component — keep it minimal
- **Shared UI state**: React Context (Toast notifications, session)
- **No Redux or Zustand** — not part of this stack

### Error & Loading States

Always handle loading and error in components that fetch data:

```typescript
function PortfolioPage({ id }: { id: string }) {
  const { data, isLoading, error } = usePortfolio(id);

  if (isLoading) return <Loading />;
  if (error) return <ErrorAlert message={error.message} />;
  if (!data) return <EmptyState message="Portfolio not found" />;

  return <PortfolioDetail portfolio={data} />;
}
```

Use the shared components: `<Loading />`, `<ErrorAlert />`, `<EmptyState />`.

### Toast Notifications

Use the `useToast` hook from `ToastContext` for user feedback:

```typescript
const { showToast } = useToast();

// On success
showToast({ type: 'success', message: 'Portfolio created successfully' });

// On error
showToast({ type: 'error', message: 'Failed to sync account' });
```

---

## 5. UI / Design System Rules

The application uses a **dark neon cyberpunk** theme. All new UI must follow these rules.

### Color Palette

| Role | Tailwind Classes |
|------|----------------|
| Primary CTA / important icons | `fuchsia-500`, `fuchsia-600` |
| Secondary elements | `violet-500`, `violet-600` |
| Data / info displays | `cyan-400`, `cyan-500` |
| Danger / delete | `red-500`, `red-300` |
| Success / verified | `green-400` |
| Primary text | `slate-200` |
| Secondary text | `slate-300` |
| Card background | `slate-950` |
| Page background | `black` |

### Required Patterns

#### Cards

```tsx
// Primary card (fuchsia border)
<div className="bg-slate-950/70 backdrop-blur-sm border-2 border-fuchsia-500/40
                shadow-[0_0_40px_rgba(217,70,239,0.4)] rounded-2xl p-6">
```

#### Buttons

```tsx
// Primary CTA
<button className="inline-flex items-center px-8 py-3 border-2 border-fuchsia-500
                   text-base font-bold rounded-lg text-white
                   bg-gradient-to-r from-fuchsia-600 via-purple-600 to-violet-600
                   hover:from-fuchsia-500 hover:via-purple-500 hover:to-violet-500
                   shadow-[0_0_30px_rgba(217,70,239,0.6)]
                   hover:shadow-[0_0_50px_rgba(217,70,239,0.9)]
                   transition-all duration-300 transform hover:scale-110">

// Danger button
<button className="inline-flex items-center px-4 py-2 border-2 border-red-500
                   text-sm font-bold rounded-lg text-red-300 bg-red-950/30
                   hover:bg-red-900/50 hover:border-red-400
                   shadow-[0_0_20px_rgba(239,68,68,0.4)]
                   transition-all duration-300 transform hover:scale-105">
```

#### Headings

```tsx
// Section heading
<h2 className="text-3xl font-extrabold
               bg-gradient-to-r from-fuchsia-300 via-violet-300 to-purple-300
               bg-clip-text text-transparent
               drop-shadow-[0_0_20px_rgba(232,121,249,0.6)]">
```

### Rules

- Always use `border-2` (not `border`) on prominent interactive elements
- Always include neon box-shadow (`shadow-[0_0_Xpx_...]`) on cards and buttons
- Always use `backdrop-blur-sm` on modal/card overlays
- Never use light backgrounds — stick to `black` or `slate-950`
- Never use flat colors for buttons — always gradient + glow
- Use `animate-pulse` on icons and primary CTAs only (not body text)
- Apply `transition-all duration-300` + `transform hover:scale-105` on interactive cards

---

## 6. Database & Migration Conventions

### Migration File Naming

```
m{YYYYMMDD}_{6-digit-sequence}_{description}.rs
```

Example: `m20260220_000001_create_evm_tokens.rs`

### Migration Structure

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(MyTable::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(MyTable::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(MyTable::CreatedAt).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MyTable::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum MyTable {
    Table,
    Id,
    CreatedAt,
}
```

### Rules

- Every migration **must** implement `down()` (rollback support)
- Use `if_not_exists()` on `create_table` for idempotency
- Add `unique_key()` constraints in the migration, not just in entities
- **Never** modify existing migration files — always create a new one
- JSON columns: prefer `JSONB` over `JSON` for queryable structured data
- All monetary amounts: `DECIMAL` (never `FLOAT` or `DOUBLE`)
- Encrypted credential columns: suffix `_encrypted` (e.g., `api_key_encrypted`)

---

## 7. API Design Conventions

### URL Structure

```
/api/{resource}                   # Collection
/api/{resource}/{id}              # Single resource
/api/{resource}/{id}/{sub}        # Nested resource
/api/{resource}/{id}/{action}     # Verb (non-CRUD action)
/api/admin/{resource}             # Admin-only resources
/api/jobs/{job-name}              # Manual job triggers
```

### HTTP Methods

| Action | Method | Notes |
|--------|--------|-------|
| List | GET | Always paginated for large collections |
| Fetch single | GET | Returns 404 if not found or wrong owner |
| Create | POST | Returns 201 with created resource |
| Update | PUT | Full replacement; returns 200 with updated resource |
| Delete | DELETE | Returns 204 No Content |
| Trigger action | POST | e.g., `/sync`, `/construct-allocation` |

### Response Conventions

```json
// Single resource
{ "id": "...", "name": "...", ... }

// List
[{ "id": "...", ... }, ...]

// Action result
{ "success": true, "message": "Account synced", "data": { ... } }

// Error (from AppError)
{ "error": "Not found", "message": "Portfolio not found" }
```

### Status Codes

| Code | When |
|------|------|
| 200 | Successful read or update |
| 201 | Successful create |
| 204 | Successful delete |
| 400 | Bad request / validation error |
| 401 | Missing or invalid JWT |
| 403 | Authenticated but unauthorized (wrong owner / missing role) |
| 404 | Resource not found (or ownership check failed) |
| 409 | Conflict (e.g., duplicate snapshot) |
| 500 | Internal server error |

---

## 8. Testing

### Rust

Run the test suite from the `api/` directory:

```bash
cargo test
```

- Unit tests live in the same file as the code they test, in a `#[cfg(test)]` module
- Integration tests live in `api/tests/`
- Use `.expect("test setup message")` only in test setup, never in production code

### Next.js / TypeScript

```bash
cd web
npm test        # Run unit tests
npm run build   # Verify TypeScript compiles with no errors
```

- Component tests use React Testing Library
- Hook tests use `@testing-library/react` with `renderHook`
- Always test loading/error/empty states, not just happy path

---

*For system architecture see [../architecture/ARCHITECTURE.md](../architecture/ARCHITECTURE.md).*
*For user workflows see [../use-cases/USE_CASES.md](../use-cases/USE_CASES.md).*
