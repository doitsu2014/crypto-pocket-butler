# Service map

What exists, who owns what, and where a change belongs. `docs/architecture/README.md` holds the C4
model and is the authority; this file is the shape you need before opening code, and it points at
the authority rather than restating it.

## The deployed pieces

| Piece | Path | Role |
|---|---|---|
| Identity | `services/identity` | users, principals, roles, support access, preferences and consent, account lifecycle, jurisdiction gate |
| Account | `services/account` | accounts and their metadata |
| Portfolio | `services/portfolio` | holdings, valuations, allocation, drift, order intents |
| Asset Registry | `services/asset-registry` | the asset catalogue |
| Market Data | `services/market-data` | prices and market feeds |
| Ledger | `services/ledger` | transactions and cost basis |
| Reporting | `services/reporting` | report composition |
| Analytics | `services/analytics` | product and portfolio analytics events |
| Notification | `services/notification` | user notifications and alerts |
| Audit | `services/audit` | append-only audit records, written by every service |
| Job Orchestration | `services/job-orchestration` | scheduled and manual jobs |
| Gateway | `services/gateway` | the only public entry point; route table, auth, rate limiting |

The two frontends are separate Yarn packages, not part of the Cargo workspace: `web/` (the SPA) and
`support-console/`. Both call **only** the gateway. The only direct external calls a browser makes
are Supabase Auth for sign-in and WalletConnect for read-only sessions.

## The service anatomy

Every service is a Cargo workspace of four crates plus a composition root:

```
services/<name>/domain/          entities, value types, business rules — no framework
services/<name>/application/     use cases behind port traits
services/<name>/infrastructure/  adapters: sqlx repositories, outbound HTTP, Vault, JWKS, OTLP
services/<name>/http/            Axum routers, middleware, DTOs
services/<name>/src/             composition root: config, wiring, bin/* (api, worker, migrate)
services/<name>/migrations/      NNNN_<name>.{up,down}.sql pairs (absent for the gateway)
services/<name>/tests/           service-level suites; some services keep them in the crates' own tests/
```

The gateway is the exception only in that it has no migrations — it owns no schema.

`bash scripts/check-boundaries.sh` enforces the direction mechanically by reading
`cargo metadata --no-deps`: a `*-domain` crate gets no local dependencies at all; `*-application`
may depend only on `*-domain`; `*-infrastructure` and `*-http` only on `*-domain` and
`*-application`; the service package may depend on anything because it is the composition root.

## Data ownership

One schema and one PostgreSQL role per service. No cross-schema grants, no Row Level Security in
services, and a service reads another service's data **only** through that service's HTTP API. If a
change feels like it needs another service's table, it needs a contract instead.

Migrations run with `search_path` set to the service's own schema, which is why object names inside
migration files are unqualified.

## Communication

- Browser → gateway, on `C1` (SPA) and `C2` (support console).
- Service → service: synchronous internal HTTP (`C3`–`C9` in the contract summary), the user's
  Supabase JWT forwarded unchanged, bounded retries with idempotency keys.
- Audit writes go through a per-service outbox drained to the audit service; there are **no message
  queues** anywhere.

## The gateway route table

`services/gateway/domain/src/route.rs` is the declared truth. A route that is not declared is
denied — there is no implicit pass-through, so adding capabilities means adding a declaration. Each
declaration names the owning service, the method and path pattern, and (where it differs from the
address default) its per-route rate budget.

## Deployment

`infra/swarm-stack.yml` describes production: every service, both frontends and the observability
stack as replicated Swarm services with health checks and rolling updates. `deployment/localhost`
is a single-machine Compose stack for development. Production is the only environment tier.