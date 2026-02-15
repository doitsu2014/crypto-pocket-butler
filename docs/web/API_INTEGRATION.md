# API Integration Guide

This guide explains how to integrate with the API in a consistent and maintainable way.

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Authenticated Requests](#authenticated-requests)
3. [API Client Usage](#api-client-usage)
4. [Custom Hooks Pattern](#custom-hooks-pattern)
5. [Error Handling](#error-handling)
6. [Best Practices](#best-practices)

---

## Architecture Overview

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

### Why This Architecture?

1. **Security**: Access tokens are never exposed to the browser
2. **Consistency**: All requests follow the same pattern
3. **Type Safety**: TypeScript ensures correct types throughout
4. **Error Handling**: Centralized error handling with ApiError
5. **Caching**: TanStack Query provides automatic caching and refetching

---

## Authenticated Requests

### All API Requests Are Authenticated

Every request to the API requires authentication. The architecture automatically handles this:

1. User signs in via Keycloak (OIDC/PKCE flow)
2. NextAuth stores the access token in a secure HTTP-only cookie
3. The Next.js API proxy extracts the token from the session
4. The proxy forwards the request to the API with `Authorization: Bearer <token>` header

### No Manual Token Handling Required

You **never** need to manually:
- Extract tokens from cookies
- Add Authorization headers
- Refresh tokens (handled automatically by NextAuth)

---

## API Client Usage

### Basic Usage

The `apiClient` function is located in `lib/api-client.ts`:

```typescript
import { apiClient } from "@/lib/api-client";

// GET request
const accounts = await apiClient<Account[]>("/v1/accounts");

// POST request
const newAccount = await apiClient<Account>("/v1/accounts", {
  method: "POST",
  body: { name: "My Wallet", type: "wallet", address: "0x..." }
});

// PUT request
const updated = await apiClient<Account>(`/v1/accounts/${accountId}`, {
  method: "PUT",
  body: { name: "Updated Name" }
});

// DELETE request
await apiClient<void>(`/v1/accounts/${accountId}`, {
  method: "DELETE"
});
```

### Type Safety

Always provide a type parameter to `apiClient` for type-safe responses:

```typescript
// ✅ Good - Type-safe
const accounts = await apiClient<Account[]>("/v1/accounts");

// ❌ Bad - Untyped
const accounts = await apiClient("/v1/accounts");
```

---

## Custom Hooks Pattern

### Using TanStack Query Hooks

**DO NOT** call `apiClient` directly in components. Instead, use or create custom hooks:

```typescript
// ✅ Good - Using hooks
import { useAccounts } from "@/hooks/useAccounts";

function AccountsList() {
  const { data: accounts, isLoading, error } = useAccounts();
  
  if (isLoading) return <div>Loading...</div>;
  if (error) return <div>Error: {error.message}</div>;
  
  return <ul>{accounts.map(acc => <li key={acc.id}>{acc.name}</li>)}</ul>;
}

// ❌ Bad - Calling apiClient directly
function AccountsList() {
  const [accounts, setAccounts] = useState([]);
  
  useEffect(() => {
    apiClient<Account[]>("/v1/accounts").then(setAccounts);
  }, []);
  
  return <ul>{accounts.map(acc => <li key={acc.id}>{acc.name}</li>)}</ul>;
}
```

### Creating Custom Hooks

Follow the pattern in existing hooks (e.g., `hooks/useAccounts.ts`):

```typescript
// hooks/useMyResource.ts
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/lib/api-client";
import type { MyResource } from "@/types/api";

// 1. Define query keys for cache management
export const myResourceKeys = {
  all: ["myResource"] as const,
  lists: () => [...myResourceKeys.all, "list"] as const,
  list: () => [...myResourceKeys.lists()] as const,
  details: () => [...myResourceKeys.all, "detail"] as const,
  detail: (id: string) => [...myResourceKeys.details(), id] as const,
};

// 2. Query hook for fetching data
export function useMyResources() {
  return useQuery({
    queryKey: myResourceKeys.list(),
    queryFn: async () => {
      return apiClient<MyResource[]>("/v1/my-resources");
    },
  });
}

// 3. Mutation hook for modifying data
export function useCreateMyResource() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async (input: CreateMyResourceInput) => {
      return apiClient<MyResource>("/v1/my-resources", {
        method: "POST",
        body: input,
      });
    },
    onSuccess: () => {
      // Invalidate cache to trigger refetch
      queryClient.invalidateQueries({ queryKey: myResourceKeys.lists() });
    },
  });
}
```

---

## Error Handling

### ApiError Class

All API errors are instances of `ApiError`:

```typescript
class ApiError extends Error {
  type: "auth" | "validation" | "server" | "network" | "unknown";
  statusCode?: number;
  details?: unknown;
}
```

### Handling Errors in Components

```typescript
import { ApiError } from "@/lib/api-client";

function MyComponent() {
  const { data, error } = useAccounts();

  if (error instanceof ApiError) {
    switch (error.type) {
      case "auth":
        return <div>Please sign in to continue</div>;
      case "validation":
        return <div>Invalid data: {error.message}</div>;
      case "server":
        return <div>Server error. Please try again later.</div>;
      case "network":
        return <div>Network error. Check your connection.</div>;
      default:
        return <div>An error occurred: {error.message}</div>;
    }
  }

  // ... render data
}
```

### Error Categories

| Error Type | Status Codes | Common Causes |
|------------|--------------|---------------|
| `auth` | 401, 403 | Token expired, insufficient permissions |
| `validation` | 400, 422 | Invalid input data |
| `server` | 500-599 | API errors, database issues |
| `network` | - | Connection lost, timeout |
| `unknown` | Other 4xx | Other client errors |

---

## Best Practices

### ✅ DO

1. **Use Custom Hooks**: Always use TanStack Query hooks for data fetching
2. **Type Everything**: Provide type parameters to `apiClient<T>()`
3. **Invalidate Cache**: Use `queryClient.invalidateQueries()` after mutations
4. **Handle Errors**: Check for `ApiError` instances and handle appropriately
5. **Follow Conventions**: Use existing hooks as templates for new ones

### ❌ DON'T

1. **Don't bypass the proxy**: Never call the backend URL directly from client code
2. **Don't handle tokens manually**: The architecture does this automatically
3. **Don't use fetch directly**: Always use `apiClient()` for consistency
4. **Don't call hooks in loops**: React hook rules apply
5. **Don't create route files**: The catch-all route handles everything

### Query Key Conventions

Follow these conventions for TanStack Query cache keys:

```typescript
export const resourceKeys = {
  all: ["resourceName"] as const,
  lists: () => [...resourceKeys.all, "list"] as const,
  list: (filters?: Filters) => [...resourceKeys.lists(), filters] as const,
  details: () => [...resourceKeys.all, "detail"] as const,
  detail: (id: string) => [...resourceKeys.details(), id] as const,
  // Add more specific keys as needed
};
```

### Cache Invalidation Strategy

Invalidate the appropriate level of cache after mutations:

```typescript
// After creating a resource
queryClient.invalidateQueries({ queryKey: resourceKeys.lists() });

// After updating a specific resource
queryClient.invalidateQueries({ queryKey: resourceKeys.detail(id) });
queryClient.invalidateQueries({ queryKey: resourceKeys.lists() });

// After deleting a resource
queryClient.invalidateQueries({ queryKey: resourceKeys.lists() });
```

---

## Examples

### Example 1: Fetching a List

```typescript
import { useAccounts } from "@/hooks/useAccounts";

function AccountsList() {
  const { data: accounts, isLoading, error } = useAccounts();

  if (isLoading) {
    return <div className="animate-pulse">Loading accounts...</div>;
  }

  if (error) {
    return <div className="text-red-500">Error: {error.message}</div>;
  }

  return (
    <ul>
      {accounts?.map(account => (
        <li key={account.id}>{account.name}</li>
      ))}
    </ul>
  );
}
```

### Example 2: Creating a Resource

```typescript
import { useCreateAccount } from "@/hooks/useAccounts";

function CreateAccountForm() {
  const createAccount = useCreateAccount();

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    
    try {
      await createAccount.mutateAsync({
        name: "My Wallet",
        type: "wallet",
        address: "0x...",
      });
      // Success! The hook automatically invalidates the cache
      alert("Account created successfully!");
    } catch (error) {
      if (error instanceof ApiError) {
        alert(`Error: ${error.message}`);
      }
    }
  };

  return (
    <form onSubmit={handleSubmit}>
      {/* form fields */}
      <button 
        type="submit" 
        disabled={createAccount.isPending}
      >
        {createAccount.isPending ? "Creating..." : "Create Account"}
      </button>
    </form>
  );
}
```

### Example 3: Updating a Resource

```typescript
import { useUpdateAccount } from "@/hooks/useAccounts";

function EditAccountForm({ accountId }: { accountId: string }) {
  const updateAccount = useUpdateAccount(accountId);

  const handleUpdate = async (newName: string) => {
    try {
      await updateAccount.mutateAsync({ name: newName });
      alert("Account updated!");
    } catch (error) {
      if (error instanceof ApiError && error.type === "validation") {
        alert("Invalid name!");
      }
    }
  };

  return (
    <button onClick={() => handleUpdate("New Name")}>
      Update Account
    </button>
  );
}
```

---

## Need Help?

- **API Client Source**: `lib/api-client.ts`
- **Hook Examples**: `hooks/useAccounts.ts`, `hooks/usePortfolios.ts`
- **Proxy Implementation**: `app/api/backend/[...path]/route.ts`
- **Type Definitions**: `types/api.ts`

For questions or issues with the API integration, refer to these files for working examples.
