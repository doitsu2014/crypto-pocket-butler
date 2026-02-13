# UI Components Documentation

This directory contains reusable UI components for the Crypto Pocket Butler application.

## Toast Notifications

### Overview
Global toast notification system for displaying success, error, warning, and info messages to users.

### Components
- **ToastContext** (`/contexts/ToastContext.tsx`) - Context provider for managing toast state
- **Toast** (`/components/Toast.tsx`) - Visual toast components and container

### Usage

#### Basic Usage
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

#### Available Methods
```tsx
// Success notification (green)
toast.success("Operation completed!");

// Error notification (red)
toast.error("Something went wrong!");

// Warning notification (yellow)
toast.warning("Please review your settings");

// Info notification (cyan)
toast.info("New features available");

// Custom duration (default is 5000ms)
toast.success("Saved!", 3000);
```

#### Integration Pattern
The toast system is automatically available throughout the application via the ToastProvider in the root layout. Simply import and use the `useToast` hook in any client component.

### Features
- ✅ Auto-dismiss with configurable duration (default: 5 seconds)
- ✅ Manual dismissal via close button
- ✅ Multiple toasts stack vertically
- ✅ Smooth slide-in animation
- ✅ Cyberpunk-themed styling with glowing effects
- ✅ Icon indicators for each toast type
- ✅ Responsive design

---

## Loading Components

### Overview
Consistent loading states and skeleton patterns for async operations.

### Components

#### 1. LoadingSkeleton
Animated skeleton loaders for different content types.

**Types:**
- `card` - Grid layout for card-based content (portfolios, accounts)
- `list` - Vertical list layout for list items
- `table` - Table layout with header and rows

**Usage:**
```tsx
import { LoadingSkeleton } from "@/components/Loading";

function PortfoliosList() {
  const [loading, setLoading] = useState(true);
  const [portfolios, setPortfolios] = useState([]);

  if (loading) {
    return <LoadingSkeleton count={3} type="card" />;
  }

  return (
    <div className="grid grid-cols-3 gap-6">
      {portfolios.map(p => <PortfolioCard key={p.id} portfolio={p} />)}
    </div>
  );
}
```

**Props:**
- `count?: number` - Number of skeleton items to render (default: 3)
- `type?: "card" | "list" | "table"` - Layout type (default: "card")

#### 2. LoadingSpinner
Centered spinner with optional message for full-page or section loading.

**Usage:**
```tsx
import { LoadingSpinner } from "@/components/Loading";

function DataView() {
  const [loading, setLoading] = useState(true);

  if (loading) {
    return <LoadingSpinner size="lg" message="Loading your data..." />;
  }

  return <div>Your data here</div>;
}
```

**Props:**
- `size?: "sm" | "md" | "lg"` - Spinner size (default: "md")
- `message?: string` - Optional loading message displayed below spinner

#### 3. LoadingButton
Button component with integrated loading state.

**Usage:**
```tsx
import { LoadingButton } from "@/components/Loading";

function SaveForm() {
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    await saveData();
    setSaving(false);
  };

  return (
    <LoadingButton
      loading={saving}
      onClick={handleSave}
      className="px-4 py-2 bg-violet-600 text-white rounded-lg"
    >
      Save Changes
    </LoadingButton>
  );
}
```

**Props:**
- `loading?: boolean` - Whether the button is in loading state
- `children: React.ReactNode` - Button content
- `className?: string` - Additional CSS classes
- `onClick?: () => void` - Click handler
- `type?: "button" | "submit" | "reset"` - Button type (default: "button")
- `disabled?: boolean` - Whether button is disabled

### Design System

All components follow the cyberpunk theme with:
- **Colors:** Violet (#8b5cf6) and Fuchsia (#d946ef) primary colors
- **Effects:** Backdrop blur, glowing shadows, animated borders
- **Dark Mode:** Slate backgrounds with transparency
- **Animations:** Smooth transitions and pulse effects

---

## Other Components

### ErrorAlert
Display inline error messages with optional retry and dismiss actions.

### EmptyState
Context-aware empty state component with appropriate messaging and call-to-action buttons.

### SignOutButton
Authentication sign-out button with proper session handling.

---

## Best Practices

### 1. Loading States
Always provide feedback for async operations:
```tsx
const [loading, setLoading] = useState(true);
const [error, setError] = useState<string | null>(null);
const toast = useToast();

useEffect(() => {
  const fetchData = async () => {
    try {
      setLoading(true);
      const data = await apiClient("/v1/portfolios");
      setPortfolios(data);
    } catch (err) {
      const message = err instanceof ApiError ? err.message : "Failed to load";
      setError(message);
      toast.error(message);
    } finally {
      setLoading(false);
    }
  };
  fetchData();
}, [toast]);

if (loading) return <LoadingSkeleton type="card" count={3} />;
if (error) return <ErrorAlert message={error} />;
```

### 2. Toast Feedback
Provide clear feedback for user actions:
```tsx
const handleDelete = async (id: string) => {
  try {
    await apiClient(`/v1/portfolios/${id}`, { method: "DELETE" });
    toast.success("Portfolio deleted successfully");
    // Update UI
  } catch (err) {
    toast.error("Failed to delete portfolio");
  }
};
```

### 3. Progressive Loading
Use appropriate skeleton types matching your content layout:
- Card grid → `<LoadingSkeleton type="card" count={6} />`
- Data table → `<LoadingSkeleton type="table" count={10} />`
- Vertical list → `<LoadingSkeleton type="list" count={5} />`

### 4. Button Loading States
Always disable and show loading state for async button actions:
```tsx
<LoadingButton
  loading={isDeleting}
  onClick={handleDelete}
  className="btn-danger"
  disabled={!canDelete}
>
  Delete Portfolio
</LoadingButton>
```

---

## Component Architecture

```
frontend-react/
├── contexts/
│   └── ToastContext.tsx         # Toast state management
├── components/
│   ├── Toast.tsx                # Toast UI components
│   ├── Loading.tsx              # Loading components
│   ├── ErrorAlert.tsx           # Error display
│   ├── EmptyState.tsx           # Empty states
│   └── README.md                # This file
└── app/
    └── layout.tsx               # ToastProvider integration
```

The toast system is provided at the root level, making it available to all components without prop drilling.

---

## Interactive Demo

Visit `/demo/ui-components` in your development environment to see an interactive showcase of all UI components with live examples and code snippets.

---

## Additional Resources

- **Next.js Documentation**: [https://nextjs.org/docs](https://nextjs.org/docs)
- **Tailwind CSS**: [https://tailwindcss.com/docs](https://tailwindcss.com/docs)
- **React Context API**: [https://react.dev/reference/react/createContext](https://react.dev/reference/react/createContext)

For questions or contributions, please refer to the main project README.
