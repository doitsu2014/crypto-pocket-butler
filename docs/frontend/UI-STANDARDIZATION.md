# UI Standardization Implementation

## Overview
This document describes the standardized UI components implemented for error handling, loading states, empty states, and user feedback across the Crypto Pocket Butler application.

## Components Created

### 1. Toast Notification System

**Location**: `contexts/ToastContext.tsx` + `components/Toast.tsx`

**Features**:
- Global toast notification system with React Context
- 4 types: success, error, info, warning
- Auto-dismiss with configurable duration
- Slide-in animation
- Cyberpunk-themed styling with appropriate colors
- Toast provider wraps entire app in root layout

**Usage**:
```typescript
const toast = useToast();

// Success notification
toast.success("Portfolio created successfully!");

// Error notification
toast.error("Failed to load data");

// Info and warning
toast.info("Processing...");
toast.warning("Action cannot be undone");
```

### 2. Loading Components

**Location**: `components/Loading.tsx`

#### LoadingSkeleton
- Three types: card, list, table
- Animated pulse effect
- Cyberpunk violet/purple gradients
- Configurable count

**Usage**:
```typescript
<LoadingSkeleton count={3} type="card" />
<LoadingSkeleton count={5} type="list" />
<LoadingSkeleton count={4} type="table" />
```

#### LoadingSpinner
- Centered spinner with rotating border
- Three sizes: sm, md, lg
- Optional message text
- Animated with glow effect

**Usage**:
```typescript
<LoadingSpinner size="md" message="Loading data..." />
```

#### LoadingButton
- Wraps any button with loading state
- Shows spinner icon when loading
- Disables button during loading
- Preserves all button styles

**Usage**:
```typescript
<LoadingButton
  loading={creating}
  type="submit"
  className="your-button-classes"
>
  {creating ? "Creating..." : "Create"}
</LoadingButton>
```

### 3. Empty State Component

**Location**: `components/EmptyState.tsx`

**Features**:
- Preset icons for common screens (portfolio, account, recommendation, snapshot, settings)
- Title and description text
- Optional action button
- Consistent styling

**Usage**:
```typescript
<EmptyState
  icon="portfolio"
  title="No portfolios yet"
  description="Create your first portfolio to get started"
  action={{
    label: "Create Portfolio",
    onClick: () => setShowCreateForm(true),
  }}
/>
```

### 4. Error Alert Component

**Location**: `components/ErrorAlert.tsx`

**Features**:
- Two types: inline and banner
- Optional retry button
- Optional dismiss button
- Red-themed error styling
- Icon with error message

**Usage**:
```typescript
<ErrorAlert 
  message={error}
  onRetry={loadData}
  onDismiss={() => setError(null)}
  type="banner"
/>
```

### 5. Enhanced API Client

**Location**: `lib/api-client.ts`

**Features**:
- Custom `ApiError` class with error type categorization
- Error types: network, auth, validation, server, unknown
- Better error messages based on HTTP status codes
- Network error detection
- Type-safe error handling

**Usage**:
```typescript
import { apiClient, ApiError } from "@/lib/api-client";

try {
  const data = await apiClient<Portfolio[]>("/v1/portfolios");
} catch (err) {
  if (err instanceof ApiError) {
    switch (err.type) {
      case "auth":
        // Handle authentication error
        break;
      case "network":
        // Handle network error
        break;
      case "validation":
        // Handle validation error
        break;
    }
  }
}
```

## Applied to Screens

All standardized components have been applied to the following core screens:

1. **Portfolios List** (`app/portfolios/components/PortfoliosClient.tsx`)
   - Toast notifications for create success
   - LoadingSkeleton for loading state
   - EmptyState with action button
   - ErrorAlert for data loading errors
   - LoadingButton for create action

2. **Accounts List** (`app/accounts/components/AccountsClient.tsx`)
   - Toast for create/delete/sync success
   - LoadingSkeleton (card type)
   - EmptyState with create action
   - ErrorAlert for errors
   - LoadingButton for multiple actions (sync, create)

3. **Portfolio Detail** (`app/portfolios/[id]/components/PortfolioDetailClient.tsx`)
   - Toast for composition updates
   - LoadingSkeleton for data loading
   - ErrorAlert for API errors

4. **Portfolio Composition Editor** (`app/portfolios/[id]/components/PortfolioCompositionEditor.tsx`)
   - Toast for save success/errors
   - LoadingSkeleton for accounts loading
   - EmptyState for no accounts
   - LoadingButton for save action
   - ErrorAlert for loading errors

5. **Snapshots** (`app/portfolios/[id]/snapshots/components/SnapshotsClient.tsx`)
   - Toast for errors
   - LoadingSpinner for data loading
   - EmptyState for no snapshots
   - ErrorAlert with retry
   - LoadingButton for refresh

6. **Recommendations List** (`app/portfolios/[id]/recommendations/components/RecommendationsClient.tsx`)
   - Toast for generate/approve success
   - LoadingSkeleton (list type)
   - EmptyState with generate action
   - ErrorAlert for errors
   - LoadingButton for generate

7. **Recommendation Detail** (`app/portfolios/[id]/recommendations/[recId]/components/RecommendationDetailClient.tsx`)
   - Toast for approve/reject success
   - LoadingSpinner for loading
   - ErrorAlert with retry
   - LoadingButton for approve/reject actions

8. **Portfolio Settings** (`app/portfolios/[id]/settings/components/SettingsClient.tsx`)
   - Toast for save success/validation errors
   - LoadingSkeleton (list type)
   - ErrorAlert for errors
   - LoadingButton for save action

## Design Patterns

### Error Handling Pattern
```typescript
const [error, setError] = useState<string | null>(null);
const toast = useToast();

// For data loading errors - use ErrorAlert
try {
  const data = await apiClient<T>("/endpoint");
  setData(data);
  setError(null); // Clear previous errors
} catch (err) {
  const message = err instanceof ApiError ? err.message : "Failed to load data";
  setError(message);
}

// For action errors - use Toast
try {
  await apiClient<T>("/endpoint", { method: "POST", body });
  toast.success("Action completed successfully!");
} catch (err) {
  const message = err instanceof ApiError ? err.message : "Action failed";
  toast.error(message);
}
```

### Loading State Pattern
```typescript
const [loading, setLoading] = useState(true);

useEffect(() => {
  loadData();
}, []);

async function loadData() {
  try {
    setLoading(true);
    const data = await apiClient<T>("/endpoint");
    setData(data);
  } catch (err) {
    setError(...);
  } finally {
    setLoading(false);
  }
}

// In render:
{loading && <LoadingSkeleton count={3} type="card" />}
{!loading && data.length === 0 && <EmptyState ... />}
{!loading && data.length > 0 && <DataList data={data} />}
```

### Action Button Pattern
```typescript
const [submitting, setSubmitting] = useState(false);

async function handleSubmit() {
  try {
    setSubmitting(true);
    await apiClient<T>("/endpoint", { method: "POST", body });
    toast.success("Success!");
  } catch (err) {
    toast.error(...);
  } finally {
    setSubmitting(false);
  }
}

// In render:
<LoadingButton loading={submitting} onClick={handleSubmit}>
  {submitting ? "Saving..." : "Save"}
</LoadingButton>
```

## Styling

All components maintain the application's **cyberpunk theme**:

- **Primary colors**: Fuchsia, Violet, Purple
- **Accent colors**: Cyan, Blue
- **Error colors**: Red
- **Success colors**: Green
- **Warning colors**: Yellow
- **Glow effects**: `shadow-[0_0_20px_rgba(...)]`
- **Backdrop blur**: `backdrop-blur-sm`
- **Border styles**: `border-2` with colored borders
- **Animations**: Pulse, spin, slide-in

## Benefits

1. **Consistency**: All screens use the same components for similar functionality
2. **Maintainability**: Changes to UI patterns only need to be made in one place
3. **Type Safety**: ApiError provides better error categorization
4. **User Experience**: Clear feedback with toasts, loading states, and empty states
5. **Code Reduction**: Less duplicated code across components
6. **Developer Experience**: Easy to use hooks and components

## Future Improvements

1. Add toast queue management for multiple simultaneous toasts
2. Add toast position configuration (top-right, top-center, bottom-right, etc.)
3. Add more loading skeleton variations
4. Add animation options for empty states
5. Add error recovery suggestions based on error type
6. Add accessibility improvements (ARIA labels, keyboard navigation)
7. Add unit tests for all components
8. Add Storybook documentation

## Migration Guide

To update an existing component:

1. Import the necessary components:
   ```typescript
   import { useToast } from "@/contexts/ToastContext";
   import { ApiError } from "@/lib/api-client";
   import { LoadingSkeleton, LoadingButton } from "@/components/Loading";
   import EmptyState from "@/components/EmptyState";
   import ErrorAlert from "@/components/ErrorAlert";
   ```

2. Replace inline error displays:
   ```typescript
   // Before:
   {error && <div className="text-red-300">{error}</div>}
   
   // After:
   {error && <ErrorAlert message={error} onDismiss={() => setError(null)} />}
   ```

3. Replace loading skeletons:
   ```typescript
   // Before:
   {loading && <div className="animate-pulse">...</div>}
   
   // After:
   {loading && <LoadingSkeleton count={3} type="card" />}
   ```

4. Replace empty states:
   ```typescript
   // Before:
   {data.length === 0 && <div>No items</div>}
   
   // After:
   {data.length === 0 && <EmptyState icon="..." title="..." />}
   ```

5. Add toast notifications:
   ```typescript
   const toast = useToast();
   
   // On success:
   toast.success("Action completed!");
   
   // On error:
   toast.error(message);
   ```

6. Update buttons with loading states:
   ```typescript
   // Before:
   <button disabled={loading}>{loading ? "Loading..." : "Submit"}</button>
   
   // After:
   <LoadingButton loading={loading}>
     {loading ? "Loading..." : "Submit"}
   </LoadingButton>
   ```
