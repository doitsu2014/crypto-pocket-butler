# Tech Stack Decisions — u5-market-data

Inputs: `.claude/rules/development.md`, the project memory, and `functional-design/functional-spec.md`.

| Decision | Choice | Rationale |
|---|---|---|
| Language and framework | Rust with Axum, tower, tokio | Fixed by project rules; no other backend language |
| Persistence | sqlx against one Supabase Postgres schema `market_data`, role `market_data_svc` | One schema and role per service; no cross-schema grants, no RLS |
| Decimals | `rust_decimal` for prices and rates | No binary floating point in financial values |
| HTTP client | `reqwest` with rustls | TLS everywhere; bounded timeouts and retries |
| Serialization | `serde` with explicit DTOs at the `http` boundary | DTOs never leak inward |
| Secrets | Supabase Vault (pgsodium); a `SECURITY DEFINER` function in `market_data` reads provider keys, so no cross-schema grant | Project rules; least privilege |
| C6 auth | Per-caller service tokens with a per-operation allow-list | Internal caller authorization, matching the amended contract |
| C11 auth | The shared internal run token from Vault | Matches C11's contract |
| Caching | None. The enabled source list is read from C4 on each request | BR1.4 forbids a second, cached copy of the mutable registry |
| Scheduling | C11 run endpoint called by Job Orchestration | No message queue exists or is added |
| Observability | `tracing` + OTLP, `metrics` exporter to Prometheus | Matches the self-hosted Loki/Prometheus/Tempo stack |
| Package manager (where JS exists) | Yarn ≥ 6 via corepack | Project rule; this unit has no JS |

## Rejected alternatives

- **Redis or an in-memory price cache**: adds a service and a second source of truth for prices;
  reads are cheap against the observation table.
- **A 30-second source-list cache**: would let priority diverge from C4 for up to 30 seconds, which
  BR1.4 forbids.
- **A message queue for refresh**: forbidden by project rules and unnecessary for one scheduled
  run.
- **A separate time-series database**: Postgres with 90-day retention and an indexed
  `(asset_id, source_name, observed_at)` query meets the launch bound.
