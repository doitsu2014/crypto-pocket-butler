# Tech Stack Decisions — u13-support-console

Inputs: `.claude/rules/development.md`, the project memory, and `functional-design/`.

| Decision | Choice | Rationale |
|---|---|---|
| UI stack | React 18 + TypeScript 5, Vite, TanStack Query, Tailwind v4 Parchment Ledger tokens | Project rule |
| Auth | `@supabase/supabase-js`, memory-only session | Project rule |
| Packaging | Separate static app `support-console/`, packaged into the existing web image | No new runtime service |
| Serving | `/support` path on the gateway/CDN | Same edge, separate bundle |
| API access | One client module calling C2 only | Boundary |
| Testing | Vitest + React Testing Library | Matches the web app |
| Package manager | Yarn ≥ 6 via corepack | Project rule |

## Rejected alternatives

- A server-rendered support app: adds a runtime for a low-traffic internal tool.
- Embedding the console in the customer SPA: mixes privilege levels in one bundle.
