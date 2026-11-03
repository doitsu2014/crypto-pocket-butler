# Crypto Pocket Butler — Development Standards

How the code is built. Architecture structure lives in
[`docs/architecture`](../architecture/README.md); business rules in the
[BRDs](../brds/README.md). When this document and those disagree about business
rules or structure, they win.

## Stack

| Layer | Technology |
| --- | --- |
| Backend services + API Gateway (BFF) | Rust (Axum, tower, sqlx, rust_decimal, tokio) |
| Frontend | React 18 + TypeScript 5 (Vite, TanStack Query, @supabase/supabase-js, Recharts, Tailwind CSS v4 mapped to the [design system](../design-system/DESIGN.md) tokens) |
| Package manager | Yarn ≥ 6 only, pinned via corepack (`packageManager` field); npm/pnpm and their lockfiles are forbidden |
| Database platform | Supabase Postgres — one schema and one role per service, no cross-schema grants |
| Deployment | Docker Swarm — every service and the observability stack as replicated Swarm services |

## Clean Architecture enforcement

Every Rust service is a Cargo workspace of four crates, one per layer:

```
http (interface)  →  application  →  domain
infrastructure    →  application  →  domain
```

- `domain` — entities, value types (decimals, asset identity), business rules. Depends on
  nothing outward: no Axum, no sqlx, no reqwest, no serde transport types.
- `application` — use cases orchestrating domain types behind ports (traits). Dependencies
  cross boundaries only as ports; infrastructure implements them.
- `infrastructure` — adapters: Postgres repositories (sqlx), Vault, OKX/RPC clients, OTLP.
- `http` — Axum routers, request/response DTOs, middleware. DTOs map to/from domain types at
  the boundary; DTO types never leak into `application` or `domain`.

**The dependency rule:** source dependencies point inward only. `domain` is imported by
everyone and imports no sibling; `application` never imports `infrastructure` or `http`;
nothing imports `http`.

**Enforcement is mechanical, not aspirational.** CI checks crate boundaries by parsing each
crate's `Cargo.toml` and rejecting forbidden edges (`domain` → anything,
`application` → `infrastructure`/`http`, any → `http`); a failing check fails the build. A
port that needs a new adapter is a normal change; an import that skips a layer is not.

Frontend is exempt from the four-crate rule but holds the same line: components render, a
single API-client module owns all gateway calls, and business rules (valuation, drift,
allocation) are computed only in the backend — the SPA displays backend-provided numbers.

## Testing

Requirement-driven tests with a happy-path floor per component (org standard). Business rules
from the BRDs are tested in `application` use cases against port fakes; adapters get one
integration test each against a real Postgres schema.
