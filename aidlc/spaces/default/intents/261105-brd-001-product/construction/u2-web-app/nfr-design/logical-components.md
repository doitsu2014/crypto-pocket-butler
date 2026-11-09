# Logical Components — u2-web-app

| Layer | Module | Responsibility |
|---|---|---|
| shell | `src/app/AppShell.tsx`, `router.tsx` | layout, routes, `RequireSession` |
| auth | `src/auth/client.ts`, `session.ts`, `mfa.ts` | supabase client, session truth, assurance routing |
| api | `src/api/client.ts`, `queries/*.ts` | one gateway client, query keys, error mapping |
| i18n | `src/i18n/{index,en.json,vi.json}` | resources, locale selection, formatting |
| design | `src/components/*` | Money, status badges, provenance, tables, panels |
| journeys | `src/features/{auth,onboarding,connect,portfolio,activity,briefs,notifications,settings,privacy,help}/*` | route components and forms |
| pwa | `src/pwa/sw.ts`, `manifest.webmanifest` | root-scoped worker with bypass rules |
| telemetry | `src/telemetry.ts` | client error beacon |
| test | `src/test/*` | Vitest setup, fixtures, axe |
