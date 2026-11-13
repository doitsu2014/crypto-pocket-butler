# Tech Stack Decisions — u3-support-access

**Upstream:** `codekb/crypto-pocket-butler/technology-stack.md`, `functional-spec.md` (where each
decision lives), `memory/project.md` (Mandated/Forbidden), the existing Identity service's
`infrastructure` and `http` crates.

The stack is fixed by project rule. This file records only what this unit adds or leans on, and
why.

| Concern | Choice | Rationale | Alternative rejected |
|---|---|---|---|
| Persistence | `sqlx` against the Identity schema; migrations `0007_support_access.{up,down}.sql` | The grant table and the existing `support.rs` are already sqlx; the schema change is three statements | An ORM — forbidden by `project.md`; a second store — the grants are Identity's evidence |
| Concurrency control | A partial unique index plus conditional `UPDATE`s | Enforced by the database in one statement, so two requests cannot both win; no application-level locking | `SELECT ... FOR UPDATE` then insert — rejected: a read-then-write race inside the same service, and the index is cheaper |
| Audit delivery | A `support_audit_outbox` drained by the **existing** `OutboxDrainer` with one more sink | U4 and U5 already run this pattern; the claim, lease, backoff, dead state and cleanup are written and tested, and a second drainer would be the same code racing itself | A second background loop — rejected: duplicate timers, duplicate cleanup, two skills for one job; a synchronous append for mutations — rejected: it makes every accepted mutation depend on the audit service's availability |
| Read-path audit | Synchronous append through the existing `HttpAuditClient`, failing closed | A served read is not a mutation with a row to reconcile, and it must not be returned before its record exists (BR3.13) | An outbox row for reads — rejected: reads would then be delivered after the fact, and the whole point is that a served read cannot be un-audited |
| Internal HTTP client | `reqwest` 0.12 with `rustls-tls`, one `Client` per source, per-request 120 ms timeout, one attempt | Already the stack's internal client and already what `HttpAuditClient` uses; a per-request timeout is a builder call | `hyper` directly — rejected: more code for the same behaviour; a shared client with a 5 s default — rejected: it would blow the 500 ms deadline |
| Metadata aggregation | Identity calls two new service-token-guarded internal endpoints and composes the projection | The field owners are Account and Portfolio; Identity composes so the decision, the refusal code and the audit stay in one place | Gateway-side aggregation — rejected: it moves a business decision into the router and duplicates the audit; a materialised projection table — rejected: a second copy of another service's data, which the no-cross-schema rule forbids |
| Service authentication for the new hops | The existing `require_service` router and per-service token registry, with a route-scoped `identity` allowlist | One trust mechanism across the platform; the allowlist keeps the token from opening other internal routes | A user JWT — rejected: no user is present on that hop; mutual TLS — rejected: new infrastructure for one edge |
| Rate limiting | The gateway's existing per-user limiter, extended with an optional per-route `max_requests_per_minute` field delivered by this unit's gateway work | One limiter, one policy source; the field is what lets the two expensive manual triggers be stricter than the global 120/min without a second limiter (NFR11.1) | A limiter inside Identity — rejected: it would be a second, drifting rate-limit policy and could be bypassed by any caller skipping the gateway |
| Metrics | The hand-rolled Prometheus exposition in `http/src/metrics.rs` | Already the service's convention, and its counters are what the existing gauges read | The `prometheus` crate — rejected: a new dependency for six counters |
| Tracing | `tracing` spans with `tracing-opentelemetry`, as the service already exports | No new dependency; the fan-out shows as child spans under the request's trace | Per-source metrics-only — rejected: then a slow owner and a failed audit append look alike |
| Job scheduling | The service's existing tokio interval for the drainer | One timer, already tuned | A separate scheduler process — rejected: new deployment unit for a 20-row pass |
| Frontend | None — the metadata panel is U9, approval is U10 | U3 is a service unit | Building a console screen here — rejected: it would duplicate U9/U10 and split their acceptance criteria |

## Dependencies added by this unit

None beyond what the Identity service already declares. The work uses `sqlx` (migrations and the
existing queries), `axum` (four routes on the existing router), `reqwest` (the two new hops, same
client style as `HttpAuditClient`), `uuid` (outbox idempotency key), `serde`/`serde_json` (the
projection and the C10 payload), `async-trait` (the source port), `tracing`/`tracing-opentelemetry`
(spans), and `rust_decimal` is **not** used — no money value appears in this unit's payloads.

## Constraint check

- Dependencies point inward only: the rules and the projection shape live in `domain`; the use
  cases and the source port in `application`; sqlx repositories, the two source clients and the
  outbox sink in `infrastructure`; the four routes, DTOs and the problem envelope in `http`
  (`project.md` Mandated, `.claude/rules/development.md`).
- No ORM other than sqlx; no message queue; no cross-schema grant; no RLS; no new secret in source
  (the service token comes from the existing registry/Vault path).
- The boundary gate (`scripts/check-boundaries.sh`) gains no outward edge: `application` defines
  the source port, `infrastructure` implements it.
- The schema change ships as an up/down pair, and the down migration re-adds the pre-0007
  constraint as `NOT VALID`, so historical rows stay as recorded and a rollback never blocks
  (NFR18.2).