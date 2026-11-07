# Tech Stack Decisions — u4-account

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

The stack is fixed by the project rules; this file records the Account-specific selections.

## Decisions

| Area | Decision | Rationale | Alternatives rejected |
|---|---|---|---|
| Language and framework | Rust with Axum, tower, tokio | Fixed by the project rules | Other languages (forbidden) |
| Persistence | Supabase Postgres through sqlx | Fixed by the project rules | ORM (forbidden) |
| Credentials | Supabase Vault secret references | Fixed by the project rules | Env files (forbidden) |
| Venue clients | Per-venue read-only adapters behind port traits (Binance, OKX) | Isolates provider shapes and rate limits | A generic exchange SDK (rejected: control and testability) |
| Chain clients | Per-chain RPC adapters driven by the C4 supported-chains read | New chains need no Account redeploy | Hardcoded chain list (rejected) |
| WalletConnect | SPA-held session; Account stores the address and an opaque reference | Read-only, no signing key in the service | Server-side session (rejected: more secrets) |
| CSV | Streaming parser with explicit per-format column maps | Per-line errors and bounded memory | Load the whole file (rejected: memory) |
| Numeric values | `rust_decimal` for quantities and prices | Fixed by the project rules | Floats (forbidden) |
| Service communication | Synchronous internal HTTP with bounded retries and idempotency keys | Fixed by the project rules | Message queue (forbidden) |
| Deployment | Docker Swarm, health-checked rolling updates, versioned sqlx migrations | Fixed by the project rules | Kubernetes (not the platform) |
| Package manager | Yarn ≥ 6 through corepack (web only) | Fixed by the project rules | npm/pnpm (forbidden) |

## Open decisions

- The per-venue rate-limit numbers are settled at implementation against each venue's documented
  limits.

NFR references: NFR9, NFR12, NFR20.
