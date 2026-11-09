# Tech Stack Decisions — u3-identity

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

The stack is fixed by the project rules; this file records the Identity-specific selections and the
alternatives rejected for each.

## Decisions

| Area | Decision | Rationale | Alternatives rejected |
|---|---|---|---|
| Language and framework | Rust with Axum, tower, and tokio | Fixed by the project rules; typed, async, small memory footprint | Node/TypeScript backend (rejected: stack fixed) |
| Persistence | Supabase Postgres through sqlx; no ORM | Fixed by the project rules; explicit SQL for the role and schema model | Diesel/SeaORM (rejected: sqlx only) |
| Money and time | `rust_decimal` for any monetary value; time types for expiry and retention windows | Fixed by the project rules; no binary floating point | `f64` (rejected) |
| Identity provider | Supabase Auth for sign-up, sign-in, MFA (AAL2), sessions, and recovery codes | The provider already owns the credentials and the session lifecycle; Identity must not duplicate them | Local password store (rejected: security and duplication) |
| Secrets | Supabase Vault (pgsodium) for the per-subject crypto-shredding key | Fixed by the project rules | Environment files (rejected) |
| Schema and roles | One `identity` schema and one Postgres role for the service; no cross-schema grants; no RLS | Fixed by the project rules | Shared schema with Account (rejected: cross-schema boundary) |
| Service communication | Synchronous internal HTTP with bounded retries and idempotency keys | Fixed by the project rules; the export and deletion workers call the owning services | Message queue (rejected: project forbids it) |
| Deployment | Docker Swarm, replicated, health-checked rolling updates; versioned up/down sqlx migrations | Fixed by the project rules; production is the only environment | Kubernetes (rejected: not the chosen platform) |
| Frontend | React 18 + TypeScript 5 on Vite, TanStack Query, Supabase Auth client, Parchment Ledger tokens | Fixed by the project rules; the SPA calls the gateway and Supabase Auth only | Other SPA frameworks (rejected) |
| Package manager | Yarn ≥ 6 pinned through corepack | Fixed by the project rules | npm/pnpm (forbidden) |

## Open decisions

- The exact Supabase Auth admin capability used for session revoke is verified at Code Generation.
  Listing uses Identity's `DeviceSession` view, so it does not depend on an Auth list endpoint.
- The export archive uses the private `identity-exports` bucket with a 24-hour signed download; the
  contributing-service export and anonymise operations are contract change requests to C5, C7, and
  C8 recorded in the functional specification.
- The support-grant check on the Account metadata read is a C2 change request (`grantId` and an
  approval operation) recorded in the functional specification.
