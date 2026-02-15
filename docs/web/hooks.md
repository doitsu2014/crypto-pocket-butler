# TanStack Query Hooks Documentation

This directory contains typed data fetching hooks powered by TanStack Query (React Query) for the Crypto Pocket Butler application.

## Overview

All data fetching has been centralized using TanStack Query, providing:
- **Automatic caching** - Reduces unnecessary network requests
- **Background refetching** - Keeps data fresh automatically
- **Optimistic updates** - Immediate UI feedback for mutations
- **Error handling** - Centralized retry logic based on error types
- **Loading states** - Built-in loading and error states
- **Type safety** - Full TypeScript support

## Architecture

### Query Client Configuration (`lib/query-client.ts`)

The query client is configured with intelligent defaults:
- **Stale time**: 30 seconds - data is fresh for this duration
- **Retry logic**: 
  - Auth errors (401/403): No retry
  - Validation errors (400/422): No retry  
  - Network/server errors: Retry up to 2 times with exponential backoff
- **Refetch on window focus**: Enabled for data freshness
- **Exponential backoff**: 1s, 2s, 4s, ... up to 30s

### Provider Setup (`contexts/QueryClientProvider.tsx`)

The `QueryClientProvider` wraps the entire application in `app/layout.tsx`, making TanStack Query available to all components.

## Available Hooks

### Portfolio Hooks (`hooks/usePortfolios.ts`)

#### Queries
- `usePortfolios()` - List all portfolios
- `usePortfolio(id)` - Get single portfolio
- `usePortfolioHoldings(id)` - Get portfolio with current holdings

#### Mutations
- `useCreatePortfolio()` - Create new portfolio
- `useUpdatePortfolio(id)` - Update portfolio
- `useDeletePortfolio()` - Delete portfolio

**Example:**
```typescript
import { usePortfolios, useCreatePortfolio } from '@/hooks';

function PortfoliosPage() {
  const { data: portfolios, isLoading, error } = usePortfolios();
  const createPortfolio = useCreatePortfolio();

  const handleCreate = async () => {
    await createPortfolio.mutateAsync({
      name: "My Portfolio",
      is_default: false
    });
  };

  if (isLoading) return <LoadingSkeleton />;
  if (error) return <ErrorAlert message={error.message} />;
  
  return <div>{/* render portfolios */}</div>;
}
```

### Account Hooks (`hooks/useAccounts.ts`)

#### Queries
- `useAccounts()` - List all accounts
- `useAccount(id)` - Get single account

#### Mutations
- `useCreateAccount()` - Create wallet or exchange account
- `useUpdateAccount(id)` - Update account
- `useDeleteAccount()` - Delete account
- `useSyncAccount()` - Sync account holdings from blockchain/exchange

**Example:**
```typescript
import { useAccounts, useSyncAccount } from '@/hooks';

function AccountsPage() {
  const { data: accounts } = useAccounts();
  const syncAccount = useSyncAccount();

  const handleSync = async (accountId: string) => {
    const result = await syncAccount.mutateAsync(accountId);
    if (result.success) {
      toast.success(`Synced ${result.holdings_count} holdings`);
    }
  };

  return <div>{/* render accounts */}</div>;
}
```

### Snapshot Hooks (`hooks/useSnapshots.ts`)

#### Queries
- `useSnapshots(portfolioId, params?)` - Get portfolio snapshots with optional filters

**Query Parameters:**
- `snapshot_type` - Filter by type (e.g., "daily", "manual")
- `start_date` - ISO date string
- `end_date` - ISO date string
- `limit` - Max number of snapshots

**Example:**
```typescript
import { useSnapshots } from '@/hooks';

function SnapshotsPage({ portfolioId }: { portfolioId: string }) {
  const { data, isLoading } = useSnapshots(portfolioId, {
    snapshot_type: "daily",
    start_date: "2024-01-01",
    limit: 90
  });

  const snapshots = data?.snapshots || [];
  return <div>{/* render chart */}</div>;
}
```

### Recommendation Hooks (`hooks/useRecommendations.ts`)

#### Queries
- `useRecommendations(portfolioId, params?)` - Get recommendations with filters
- `useRecommendation(portfolioId, recId)` - Get single recommendation

#### Mutations
- `useGenerateRecommendations()` - Generate new recommendations
- `useApproveRecommendation()` - Approve a recommendation
- `useRejectRecommendation()` - Reject a recommendation
- `useExecuteRecommendation()` - Execute an approved recommendation

**Example:**
```typescript
import { useRecommendations, useGenerateRecommendations } from '@/hooks';

function RecommendationsPage({ portfolioId }: { portfolioId: string }) {
  const { data } = useRecommendations(portfolioId, { status: "pending" });
  const generateRecs = useGenerateRecommendations();

  const handleGenerate = async () => {
    await generateRecs.mutateAsync(portfolioId);
  };

  return <div>{/* render recommendations */}</div>;
}
```

## Query Keys

Each hook module exports query keys for advanced usage:

```typescript
import { portfolioKeys, accountKeys, snapshotKeys, recommendationKeys } from '@/hooks';

// Invalidate specific queries
queryClient.invalidateQueries({ queryKey: portfolioKeys.lists() });
queryClient.invalidateQueries({ queryKey: accountKeys.detail(accountId) });
```

## Type Definitions (`types/api.ts`)

All API types are centralized in `types/api.ts`:
- `Portfolio`, `Account`, `Snapshot`, `Recommendation`
- Input types for mutations: `CreatePortfolioInput`, `CreateAccountInput`, etc.
- Response types: `ListSnapshotsResponse`, `PortfolioHoldingsResponse`, etc.

## Error Handling

All hooks use the centralized `ApiError` class from `lib/api-client.ts`:

```typescript
interface ApiError {
  message: string;
  type: 'network' | 'auth' | 'validation' | 'server' | 'unknown';
  statusCode?: number;
  details?: unknown;
}
```

**Error handling in components:**
```typescript
const { error } = usePortfolios();

if (error) {
  const errorMessage = error instanceof ApiError 
    ? error.message 
    : 'An unexpected error occurred';
  
  return <ErrorAlert message={errorMessage} />;
}
```

## Automatic Query Invalidation

Mutations automatically invalidate related queries:

- Creating a portfolio → Invalidates portfolio list
- Syncing an account → Invalidates account data AND portfolio holdings
- Executing a recommendation → Invalidates recommendations AND portfolio data

This ensures the UI stays in sync without manual refetching.

## Best Practices

1. **Use the hooks at the component level** - Not in utility functions
2. **Handle loading and error states** - Always check `isLoading` and `error`
3. **Use optimistic updates** - For better UX (see TanStack Query docs)
4. **Leverage automatic refetching** - Data stays fresh on window focus
5. **Don't mix with manual fetch** - Use hooks consistently throughout

## Migration from Manual Fetching

Before (manual state management):
```typescript
const [data, setData] = useState([]);
const [loading, setLoading] = useState(true);
const [error, setError] = useState(null);

useEffect(() => {
  async function load() {
    try {
      setLoading(true);
      const result = await apiClient('/endpoint');
      setData(result);
    } catch (e) {
      setError(e);
    } finally {
      setLoading(false);
    }
  }
  load();
}, []);
```

After (TanStack Query):
```typescript
const { data, isLoading, error } = useMyCustomHook();
```

## Benefits Achieved

✅ **168 fewer lines of code** across 4 components  
✅ **Automatic caching** - Reduced API calls  
✅ **Smart retry logic** - Better error recovery  
✅ **Type safety** - Centralized types  
✅ **Consistent patterns** - Easier to maintain  
✅ **Better UX** - Automatic background updates  

## Further Reading

- [TanStack Query Documentation](https://tanstack.com/query/latest)
- [React Query Best Practices](https://tkdodo.eu/blog/practical-react-query)
