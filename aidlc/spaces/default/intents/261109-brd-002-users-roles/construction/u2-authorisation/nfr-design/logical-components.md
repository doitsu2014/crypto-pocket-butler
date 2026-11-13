# Logical Components — u2-authorisation

**Upstream:** `security-design.md`, `reliability-design.md`, `performance-design.md`, `functional-spec.md`, `components.md` (Authorisation).

## Component inventory

| Component | Layer | Responsibility | Depends on |
|---|---|---|---|
| `AuthorisationDecision` | application module | The catalogue lookup and the evaluation order; returns a `Decision` | `OperationCatalogue`, `StatusReader` (port), `AuditSink` (port), `ObservationSink` (port) |
| `RoleChange` | application use case | Owns BR2.10's ordering: intent audit write, provider write, outcome audit write; aborts before the provider write | `AuthorisationDecision`, `RoleWriter` (port), `AuditSink` (port), `ObservationSink` (port) |
| `OperationCatalogue` | application value | Maps an operation name to its class (owner-scoped, collection, administrative, confirmation, support-metadata, undeclared) | none |
| `StatusReader` (port) | application trait | The caller's status by user id | implemented by `StatusReaderLocal` |
| `StatusReaderLocal` | infrastructure adapter | One indexed read on the Identity service's own pool, 200 ms acquire / 100 ms statement, no retry | Identity Postgres (`identity_app`) |
| `StatusCache` | infrastructure helper behind the port | Outage-only, 30 s TTL, 10 000 entries, LRU; consulted after a failed or timed-out read | `StatusReader` |
| `AuditSink` (port) | application trait | Writes one audit entry; a failure is surfaced, never swallowed; the adapter owns the fixed field set and redaction | implemented by `AuditSinkClient` |
| `AuditSinkClient` | infrastructure adapter | C9 internal HTTP write with TLS and a 250 ms ceiling | Audit service |
| `RoleWriter` (port) | application trait | Writes `app_metadata.role` for one user | implemented by `SupabaseRoleWriter` |
| `SupabaseRoleWriter` | infrastructure adapter | The Supabase Auth admin API call | Supabase Auth |
| `ObservationSink` (port) | application trait | Records the decision counter, the duration histogram, and the cache gauges | implemented by `DecisionMetrics` |
| `DecisionMetrics` | infrastructure adapter | Emits `authorisation_*` metrics to the registry | metrics registry |
| `AuthorizeRoute` | http adapter | `POST /internal/authorize`: service token + allowlist + user JWT; rejects `callerSubject`/`roleClaim` in the body; calls the decision | `AuthorisationDecision` |
| `RoleChangeRoute` | http adapter | The admin role-change endpoint: authentication and DTO mapping only | `RoleChange` |

## Import direction

Every arrow points inward: `http` and `infrastructure` depend on `application`; `application`
depends only on its own ports and the `domain` value types. The decision reaches metrics, audit,
status, and the provider write **only through ports it declares**, so no application module
imports an adapter and the boundary gate stays green.

## Boundaries and failure domains

- **One failure domain inside the Identity process.** A panic or an exhausted pool affects the
  Identity service's replicas only; no other service links the code.
- **Callers outside Identity see only the C4 route.** Their failure domain ends at their own
  client; a 503 from the endpoint never propagates further than the request that caused it.
- **The audit store and the status source are the only external failure domains**, each with its
  own timeout and its own designed behaviour (fail-closed, or cache-then-fail-closed).

## Blast radius

| Failure | Blast radius |
|---|---|
| Decision bug | Every request in the Identity service that calls it — the widest case, contained to one service |
| Status source down | All callers' authentication decisions, bounded by the 30 s cache per caller |
| Audit store down | Every denial, and every role change; the allow path is unaffected |
| C4 endpoint down | Account, Portfolio, and any external C4 consumer; Identity's in-process callers are unaffected |

## Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| Identity Postgres (`identity_app`) | The rest of the Identity service | Own schema, own role; the decision borrows the service's pool |
| Audit service (C9) | Every service | Per-service token; the decision writes, never reads |
| Supabase Auth admin API | The platform | One credential, held by the Identity service |
| C4 endpoint | U3, Account, Portfolio | Per-service token plus allowlist plus the user's JWT |

## What this unit deliberately does not own

No database table, no cache tier, no queue, no background job, no new container. The decision is
a module plus adapters, and its only persisted effects are the provider role claim and the audit
entries it writes.
