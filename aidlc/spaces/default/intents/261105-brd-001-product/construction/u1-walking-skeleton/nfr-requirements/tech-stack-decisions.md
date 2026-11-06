# Technology Stack Decisions — u1-walking-skeleton

The stack is fixed by the project rules; this file records the choices this Unit relies on and
why, not a re-selection.

| Layer | Choice | Rationale |
|---|---|---|
| Language (services) | Rust | Fixed by `.claude/rules/development.md`; no alternative permitted |
| HTTP framework | Axum + tower | Fixed; tower gives middleware for auth, correlation, and rate limiting |
| Database access | sqlx | Fixed; compile-time checked queries, one schema per service |
| Decimal arithmetic | rust_decimal | Fixed; quantities and values never binary floating point |
| Async runtime | tokio | Fixed |
| Database | Supabase Postgres, schema `registry` | One schema and one Postgres role per service; no cross-schema grants |
| Secrets | Supabase Vault (pgsodium) | Fixed; no secrets in source or env files |
| Auth | Supabase Auth + JWKS validation | The gateway validates and forwards the JWT unchanged |
| Frontend shell | React 18 + TypeScript 5 + Vite + TanStack Query + Tailwind v4 | Fixed; the shell in this Unit is the walking skeleton's UI slice |
| Package manager | Yarn ≥6 (corepack) | Fixed |
| Deployment | Docker Swarm, rolling updates, health-gated | Fixed; production is the only environment |
| Migrations | sqlx versioned up/down pairs | Fixed |
| Observability | Loki, Prometheus, Tempo, Grafana on the Swarm | Fixed by the architecture |

## Rejected alternatives

- **Go or Node services** — forbidden by the stack rule.
- **An ORM** — forbidden; sqlx only.
- **A separate cache service (Redis)** — not justified for a small, read-heavy registry; an
  in-process cache with explicit invalidation is simpler and enough at launch.
- **A message queue for registry changes** — forbidden; invalidation is a local event within the
  registry and its replicas poll the change version.
