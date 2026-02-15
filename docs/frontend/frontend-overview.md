# Crypto Pocket Butler - Frontend

For detailed setup and usage instructions, see [../setup/FRONTEND_SETUP.md](../setup/FRONTEND_SETUP.md).

For UI/UX design system documentation, see [UI-STYLE-GUIDE.md](UI-STYLE-GUIDE.md).

**For API integration guidelines, see [API_INTEGRATION.md](API_INTEGRATION.md).**

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

## Key Features

- 🔐 Keycloak OIDC authentication with PKCE
- 🔄 Automatic token refresh
- 🎨 Intense neon cyberpunk theme with TailwindCSS
- 🛡️ Secure token management (server-side)
- 📱 Responsive design
- 📚 Comprehensive design system documentation
- 🔌 **Unified API integration with automatic authentication**

## Documentation

- **Setup Guide**: [../setup/FRONTEND_SETUP.md](../setup/FRONTEND_SETUP.md)
- **Design System**: [UI-STYLE-GUIDE.md](UI-STYLE-GUIDE.md)
- **Keycloak Setup**: [../setup/KEYCLOAK_SETUP.md](../setup/KEYCLOAK_SETUP.md)
- **API Integration**: [API_INTEGRATION.md](API_INTEGRATION.md) ⭐

## API Integration

All backend API calls use a **unified, centralized approach**:

1. **Client-side**: Use custom TanStack Query hooks (e.g., `useAccounts`, `usePortfolios`)
2. **API Client**: All requests go through `lib/api-client.ts`
3. **Proxy Layer**: Single catch-all route at `/api/backend/[...path]` handles authentication
4. **Backend**: Requests are forwarded with proper authorization headers

### Example Usage

```typescript
// ✅ Correct way - Use custom hooks
import { useAccounts } from "@/hooks/useAccounts";

function MyComponent() {
  const { data: accounts, isLoading, error } = useAccounts();
  // Component logic...
}

// ✅ For mutations
import { useCreateAccount } from "@/hooks/useAccounts";

function MyForm() {
  const createAccount = useCreateAccount();
  
  const handleSubmit = async (data) => {
    await createAccount.mutateAsync(data);
  };
}
```

**See [API_INTEGRATION.md](API_INTEGRATION.md) for complete documentation.**
