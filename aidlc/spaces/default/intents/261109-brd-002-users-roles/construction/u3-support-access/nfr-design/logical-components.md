# Logical Components — u3-support-access

**Upstream:** `functional-spec.md` (W1–W5), `security-design.md`, `reliability-design.md`,
`observability-design.md`, `components.md` (SupportAccess), `aidlc/spaces/default/codekb/crypto-pocket-butler/code-structure.md`.

This unit extends the existing `SupportAccess` component rather than adding one. The grant
lifecycle (`domain/src/support.rs`, `application/src/support.rs`,
`infrastructure/src/repositories/support.rs`) already exists; the unit adds the metadata read, the
support audit outbox, and the wiring that makes all four routes fail closed.

## Component inventory

| Component | Layer | Responsibility | Depends on |
|---|---|---|---|
| `support` (grant rules) | `domain` | `SupportAccessGrant` state machine (`pending_approval`, `active`, `denied`, `expired`, `revoked`), the 60-minute window, requester/approver distinctness, reason bounds, and whether a grant authorises a read at a given instant | `ids`, `error`, `audit` — no framework, no clock of its own (time is passed in) |
| `support` (use cases) | `application` | `SupportService::request/approve/revoke` (existing) plus `read_metadata`: evaluate the grant, fan out to the two owners, append the audit intent before returning | `SupportGrantRepository`, `SupportMetadataSource`, `AuditIntentWriter`, `AuditAppender`, `UserRepository`, `Clock` |
| `SupportGrantRepository` | `application` (port) | Persistence of grants and their outbox rows in one transaction; the conditional updates and the partial-unique conflict | — |
| `SupportMetadataSource` | `application` (port) | One owner's allowlisted projection for a user id; returns the owner's refusal as a typed error, never a body | — |
| `AuditIntentWriter` | `application` (port) | Inserts a support audit event into the outbox in the caller's transaction (accepted mutations and refusals) | — |
| `AuditAppender` | `application` (port) | Synchronous append of a served-read event; returns failure rather than falling back to the queue | — |
| `PgSupportGrantRepository` | `infrastructure` | sqlx implementation: `INSERT … ON CONFLICT` on the partial unique, `UPDATE … WHERE id = $1 AND status = $2`, the read index lookup, the outbox insert on the same `PgConnection` | `SupportGrantRepository`, `sqlx` |
| `SupportAuditOutbox` | `infrastructure` | The `support_audit_outbox` sink for the existing `OutboxDrainer`: claim with `FOR UPDATE SKIP LOCKED`, its own batch (5) and client, the 24 h cleanup, and the backlog gauges | `AuditClient`, `PgPool` |
| `HttpSupportMetadataSource` | `infrastructure` | The Account and Portfolio metadata clients: one `reqwest` client per owner, `connect_timeout(1s)`, request timeout 120 ms, one attempt, allowlisted fields only | `SupportMetadataSource`, the `identity` service token |
| `SupportObservation` | `application` (port) | The support events a use case emits: grant outcome, read outcome and duration, source error, audit-write failure, outbox delivery outcome | — |
| `SupportOutboxBacklog` | `application` (port) | A scrape-time snapshot of the support outbox: pending count, oldest age, and last completed pass | — |
| `Metrics` | `http` | Implements `SupportObservation` alongside the existing observation ports and renders the `support_*` series; the `/metrics` handler asks `SupportOutboxBacklog` for the scrape snapshot. Wired at the composition root in `src/bin/api.rs` | `SupportObservation`, `SupportOutboxBacklog` |
| `SupportGrantsRoutes` | `http` | The three mutation routes (already present) and the new metadata read; DTOs that map to/from domain types; RFC 7807 problem bodies | `SupportService`, `CallerIdentity` from the auth middleware |
| `RouteLimits` | `http` (gateway) | The optional `max_requests_per_minute` on the gateway's `Route` and its per-user limiter path (NFR11.1) | the gateway's existing limiter |

## Import direction

```
http (SupportGrantsRoutes, RouteLimits)
        │
        ▼
application (SupportService::read_metadata, ports)
        ▲
        │
infrastructure (PgSupportGrantRepository, SupportAuditOutbox,
                HttpSupportMetadataSource, SupportMetrics)
```

`http` and `infrastructure` depend on `application`; `application` depends on `domain`; `domain`
depends on nothing but its own modules. The new ports are declared in `application/ports.rs`, and
every implementation lives in `infrastructure` — no concrete type crosses inward, and
`scripts/check-boundaries.sh` enforces it mechanically.

**Where the metrics live.** They follow the shipped observation pattern, not a new one:
`application` declares the port, and the `http` crate's `Metrics` implements it and renders the
text (as it already does for `ObservationSink`, `ConsentObservationSink` and the drainer's
observation). `infrastructure` never imports the `http` crate, so the outbox sink reports its
delivery outcomes through the same port instead of writing to a `http` type. The `/metrics`
handler's scrape read of the outbox goes through `SupportOutboxBacklog`, an application port the
repository implements — `http` never touches `sqlx` or the pool directly.

## Boundaries and failure domains

| Boundary | Failure domain | What it does not affect |
|---|---|---|
| Identity ↔ Account | The metadata read refuses (503) | Every other Identity route; the account service itself |
| Identity ↔ Portfolio | The metadata read refuses (503) | Every other Identity route; the portfolio service itself |
| Identity ↔ audit service (synchronous append) | Served reads and refusals answer 503 | Mutations, whose intent is a local outbox row |
| Identity ↔ audit service (outbox delivery) | Delivery lags; the mutation already succeeded | Request latency entirely |
| Identity ↔ its own Postgres | Every support route; nothing else if the pool is healthy | — |

The two fan-out edges are the only new outbound dependencies, and both are **read-only,
one-directional, and forbidden to call back** into Identity (NFR9.2). There is no cycle, so no
direction can amplify an outage through the other.

## Blast radius

- A failure in this unit refuses support operations. It cannot widen: no support route writes
  another service's data, and no route is on a product-path critical path.
- The migration touches one table and one constraint. It adds an index, a table, and its grants;
  it drops nothing and narrows nothing on `users` or any other Identity table.
- The gateway change is one optional field on `Route`; a route without it behaves exactly as it
  does today, which is why the other 40-odd routes are untouched.

## Shared resources

| Resource | Who else uses it | What this unit does |
|---|---|---|
| `PgPool` (identity_app) | every Identity route and worker | uses it; adds no second pool and no dedicated connection |
| `OutboxDrainer::run_once` | the consent and status sinks | adds one sink; leaves their batch sizes, clients, and leases unchanged |
| `IdentityMetrics` | every Identity route | adds series; changes no existing name or label |
| Gateway limiter | every gateway route | adds an optional per-route limit; the global default is unchanged |
| Audit service | consent and status delivery, authorisation decisions | adds a support event type and the synchronous append path |

## What this unit deliberately does not own

- **The support console or the SPA.** The routes are the boundary; no frontend change is in this
  unit.
- **The Account and Portfolio read endpoints.** They are built by their own units under the
  contract in `contract-summary.md`; this unit consumes them and owns only the client and the
  allowlist.
- **The audit service's storage, retention, and replay tooling.** It owns the store; this unit owns
  the intent and its delivery.
- **The gateway's authentication, routing table, and global limit.** It owns them; this unit adds
  one field.
- **The expiration sweeper's schedule.** Expiry is evaluated at read time and written on the next
  mutation; a background sweep is not introduced here.