# Components & Hooks

> Reusable UI components and TanStack Query hooks for data fetching.

---

## Components

### Toast Notifications

**Location:** `contexts/ToastContext.tsx` + `components/Toast.tsx`

Global toast notification system with React Context.

```tsx
import { useToast } from "@/contexts/ToastContext";

function MyComponent() {
  const toast = useToast();

  const handleSuccess = () => {
    toast.success("Portfolio created successfully!");
  };

  const handleError = () => {
    toast.error("Failed to save changes");
  };

  return (
    <button onClick={handleSuccess}>Save</button>
  );
}
```

| Method | Color | Use Case |
|--------|-------|----------|
| `toast.success()` | Green | Successful operations |
| `toast.error()` | Red | Error messages |
| `toast.info()` | Cyan | Informational updates |
| `toast.warning()` | Yellow | Warning messages |

---

### Loading Components

**Location:** `components/Loading.tsx`

#### LoadingSkeleton

```tsx
<LoadingSkeleton count={3} type="card" />  // Card grid
<LoadingSkeleton count={5} type="list" />  // List view
<LoadingSkeleton count={4} type="table" /> // Table rows
```

#### LoadingSpinner

```tsx
<LoadingSpinner size="sm" />
<LoadingSpinner size="md" message="Loading data..." />
<LoadingSpinner size="lg" />
```

---

### ErrorAlert

**Location:** `components/ErrorAlert.tsx`

```tsx
<ErrorAlert 
  message="Failed to load portfolios" 
  onRetry={loadPortfolios}
  onDismiss={() => setError(null)}
  type="banner"  // or "inline"
/>
```

---

### EmptyState

**Location:** `components/EmptyState.tsx`

```tsx
<EmptyState
  icon="portfolio"
  title="No portfolios yet"
  description="Create your first portfolio to get started"
  actionLabel="Create Portfolio"
  onAction={() => setShowCreate(true)}
/>
```

**Available Icons:** `portfolio`, `account`, `snapshot`, `chart`, `empty`

---

## TanStack Query Hooks

### Configuration

**Location:** `lib/query-client.ts`

| Setting | Value | Description |
|---------|-------|-------------|
| Stale time | 30s | Data is fresh for this duration |
| Retry (auth errors) | 0 | No retry for 401/403 |
| Retry (validation) | 0 | No retry for 400/422 |
| Retry (server errors) | 2 | Retry with exponential backoff |
| Refetch on focus | Yes | Keep data fresh |

---

### Account Hooks

**Location:** `hooks/useAccounts.ts`

```tsx
import { 
  useAccounts, 
  useAccount, 
  useCreateAccount, 
  useUpdateAccount,
  useDeleteAccount,
  useSyncAccount 
} from '@/hooks';

// List all accounts
const { data: accounts, isLoading, error } = useAccounts();

// Get single account
const { data: account } = useAccount(accountId);

// Create account
const createAccount = useCreateAccount();
await createAccount.mutateAsync({ name: "My Wallet", type: "wallet" });

// Sync account
const syncAccount = useSyncAccount();
await syncAccount.mutateAsync(accountId);
```

---

### Portfolio Hooks

**Location:** `hooks/usePortfolios.ts`

```tsx
import { 
  usePortfolios, 
  usePortfolio, 
  usePortfolioHoldings,
  useCreatePortfolio, 
  useUpdatePortfolio,
  useDeletePortfolio,
  useAddAccountToPortfolio,
  useRemoveAccountFromPortfolio
} from '@/hooks';

// List all portfolios
const { data: portfolios } = usePortfolios();

// Get portfolio with holdings
const { data: portfolio } = usePortfolioHoldings(portfolioId);

// Create portfolio
const createPortfolio = useCreatePortfolio();
await createPortfolio.mutateAsync({ 
  name: "Main Portfolio", 
  description: "My main investment portfolio" 
});

// Add account to portfolio
const addAccount = useAddAccountToPortfolio(portfolioId);
await addAccount.mutateAsync(accountId);
```

---

### Snapshot Hooks

**Location:** `hooks/useSnapshots.ts`

```tsx
import { 
  useSnapshots, 
  useLatestSnapshot,
  useCreateSnapshot 
} from '@/hooks';

// Get snapshots with date range
const { data: snapshots } = useSnapshots(portfolioId, {
  startDate: "2024-01-01",
  endDate: "2024-01-31"
});

// Get latest snapshot
const { data: latest } = useLatestSnapshot(portfolioId);

// Create snapshot
const createSnapshot = useCreateSnapshot(portfolioId);
await createSnapshot.mutateAsync({ type: "manual" });
```

---

### Asset Hooks

**Location:** `hooks/useAssets.ts`

```tsx
import { useAssets, useAssetPrices } from '@/hooks';

// List all assets
const { data: assets } = useAssets();

// Get prices for assets
const { data: prices } = useAssetPrices(["BTC", "ETH", "SOL"]);
```

---

## API Client

**Location:** `lib/api-client.ts`

### Basic Usage

```tsx
import { apiClient } from "@/lib/api-client";

// GET request
const accounts = await apiClient<Account[]>("/v1/accounts");

// POST request
const newAccount = await apiClient<Account>("/v1/accounts", {
  method: "POST",
  body: { name: "My Wallet", type: "wallet", address: "0x..." }
});

// PUT request
const updated = await apiClient<Account>(`/v1/accounts/${id}`, {
  method: "PUT",
  body: { name: "Updated Name" }
});

// DELETE request
await apiClient(`/v1/accounts/${id}`, { method: "DELETE" });
```

### Error Handling

```tsx
import { ApiError } from "@/lib/api-client";

try {
  await apiClient("/v1/accounts", { method: "POST", body: data });
} catch (error) {
  if (error instanceof ApiError) {
    console.log(error.status);    // HTTP status code
    console.log(error.message);   // Error message
    console.log(error.details);   // Validation details (if any)
  }
}
```

---

## Best Practices

### 1. Use Hooks, Not Direct API Calls

```tsx
// ❌ Don't do this
const [accounts, setAccounts] = useState([]);
useEffect(() => {
  fetch('/api/v1/accounts').then(r => r.json()).then(setAccounts);
}, []);

// ✅ Do this
const { data: accounts } = useAccounts();
```

### 2. Handle Loading and Error States

```tsx
const { data, isLoading, error, refetch } = usePortfolios();

if (isLoading) return <LoadingSkeleton count={3} type="card" />;
if (error) return <ErrorAlert message={error.message} onRetry={refetch} />;
if (!data.length) return <EmptyState icon="portfolio" title="No portfolios" />;

return <PortfolioList portfolios={data} />;
```

### 3. Use Optimistic Updates for Mutations

```tsx
const updatePortfolio = useUpdatePortfolio(id);

// In the hook, optimistic update is configured:
// - Updates UI immediately
// - Rolls back on error
// - Refetches on success
```

### 4. Provide User Feedback

```tsx
const createAccount = useCreateAccount();
const toast = useToast();

const handleSubmit = async (data) => {
  try {
    await createAccount.mutateAsync(data);
    toast.success("Account created successfully!");
    router.push("/accounts");
  } catch (error) {
    toast.error("Failed to create account");
  }
};
```