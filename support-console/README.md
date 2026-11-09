# Support Console

A separate React 18 + TypeScript 5 SPA, served at `/support` from the existing `web` image. It is
the support/admin operator surface for U13: grant request/approval/revocation, grant-scoped
account search, metadata reads, ticket metadata, the admin incident console, and read-only help.

## Commands

```sh
yarn --cwd support-console test        # Vitest + RTL + jest-axe, 180 tests
yarn --cwd support-console lint        # ESLint
yarn --cwd support-console typecheck   # tsc --build
yarn --cwd support-console build       # writes web/dist/support
yarn --cwd support-console dev         # http://localhost:5174/support/
```

Build from the repository root through the web image:

```sh
docker build -f web/Dockerfile -t web .
```

The customer build runs first (it empties `web/dist`), then the support build writes
`web/dist/support`. The runtime image serves `/` and `/support` from the same nginx.

## Security boundaries

- The Supabase session lives in memory only; `persistSession: false` with an in-memory storage
  adapter. Sign-out, grant expiry/revocation, and any 401/403 clear the TanStack Query cache.
- The gateway is the control; the console's role gate is only what the operator sees.
- Search and metadata reads require a grant id. The client refuses a missing grant id before it
  makes a request. The ticket list is grant-scoped too.
- The free-text read reason travels in the `X-Support-Reason` request header, never in a URL, so
  it never reaches access logs, browser history, or referrers. The support API hashes it for C10;
  nginx and the gateway must not log the header.
- The DTO whitelists in `src/domain/metadata.ts` strip any credential, holding, valuation,
  portfolio, requester-label, or ticket-conversation field the API might over-send. Grants and
  incidents use the same narrow-picker treatment.
- There is no audit write path in this SPA; the support API owns C10.
- CSP is enforced by `web/nginx.conf` on `/support` (and by the gateway in production). Its
  `connect-src 'self'` requires the gateway to be same-origin, or its origin must be added.
- A root-scoped customer service worker would control `/support` and `/v1`. U2's worker must
  bypass both paths; `src/pwa/isolate.ts` also unregisters any registration whose scope covers
  `/support` at console startup.

## Design

Parchment Ledger tokens are defined in `src/styles.css` and mirror `design-system/DESIGN.md`.
Status is always a text label plus a shape; every control has a visible focus ring and the styles
respect `prefers-reduced-motion`.

The incident console is a lazy route chunk; the initial shell stays under the 200 KB gzipped
budget.
