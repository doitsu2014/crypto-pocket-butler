# Next.js Web Development

## Context

Working with the Next.js 16 web frontend in `web/`.

## Conventions

- App Router (`app/` directory) with server components by default
- Add `"use client"` only when client-side interactivity is needed (event handlers, hooks, browser APIs)
- TailwindCSS 4 for styling (no CSS modules, no styled-components)
- TypeScript strict mode
- Components in `components/` directory
- Utilities in `lib/` directory

## Adding a New Page

1. Create `page.tsx` in `app/<route>/`
2. Default to server component
3. Use existing layout and shared components
4. Follow TailwindCSS patterns from the UI Style Guide

## Adding a New Component

1. Create component file in `components/`
2. Use TypeScript with proper prop types
3. Use TailwindCSS classes for styling
4. Export as default or named export
5. Add `"use client"` only if the component uses hooks or event handlers

## Authentication

- NextAuth.js v5 with Keycloak OIDC provider
- Server components: use `auth()` from `@auth`
- Client components: use `useSession()` from `next-auth/react`
- Protected routes via middleware

## Testing

```bash
cd web && npm test
```

## Linting and Type Checking

```bash
cd web && npm run lint
cd web && npx tsc --noEmit
```

## Development Server

```bash
cd web && npm run dev
```

Available at `http://localhost:3001`.

## UI Style Guide

Follow the design system documented in `docs/web/UI-STYLE-GUIDE.md` for:
- Color palette and theming
- Typography scale
- Component patterns
- Spacing and layout conventions
