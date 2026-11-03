# Development Rules

Ambient rules for every AI agent working in this repository. Extracted from
[`docs/development/README.md`](../../docs/development/README.md) — that document is
authoritative when more detail is needed. Business rules live in the
[BRDs](../../docs/brds/README.md); structure in the
[C4 model](../../docs/architecture/README.md).

## Stack (fixed — do not substitute)

- **Backend + API Gateway**: Rust — Axum, tower, sqlx, rust_decimal, tokio. No other backend
  language; no ORM other than sqlx.
- **Frontend**: React 18 + TypeScript 5 — Vite, TanStack Query, @supabase/supabase-js,
  Recharts, Tailwind CSS v4 mapped to the Gilded Ledger tokens (no raw values).
- **Package manager**: Yarn ≥ 6 only (`yarn` for every command; `npm`/`npx`/`pnpm` and their
  lockfiles are forbidden) — see the `yarn` skill.
- **Database**: Supabase Postgres — one schema and one Postgres role per service, **no
  cross-schema grants**; a service reads another service's data only via that service's HTTP
  API. Row Level Security is not used by services.
- **Deployment**: Docker Swarm. Secrets via Supabase Vault (pgsodium), never env files or
  source. Credentials and tokens are never logged.

## Clean Architecture (enforced, not aspirational)

Every Rust service is a Cargo workspace of four crates:

```
http (interface)  →  application  →  domain
infrastructure    →  application  →  domain
```

- Dependencies point **inward only**. `domain` imports no sibling crate and no framework
  (no Axum, sqlx, reqwest, serde transport types). `application` never imports
  `infrastructure` or `http`. Nothing imports `http`.
- Cross-boundary dependencies are **ports** (traits) defined inward, implemented outward.
- DTOs map to/from domain types at the `http` boundary; DTO types never leak inward.
- Frontend holds the same line: components render; one API-client module owns all gateway
  calls; business math (valuation, drift, allocation) is computed only in the backend.

Use the `clean-architecture` skill to verify boundaries; the `rust-service` skill to scaffold
a new service; the `frontend-design` skill for any UI work (Gilded Ledger design system in
`design-system/`).

## Communication

- The SPA calls **only** the API Gateway (plus Supabase Auth directly for sign-in flows).
- The gateway forwards the user's Supabase JWT unchanged; every service validates it.
- Service-to-service is synchronous internal HTTP; notifications use bounded retries with
  idempotency keys. **No message queues.**

## Testing

Requirement-driven tests with a happy-path floor per component. Business rules are tested in
`application` use cases against port fakes; each adapter gets one integration test against a
real Postgres schema.
