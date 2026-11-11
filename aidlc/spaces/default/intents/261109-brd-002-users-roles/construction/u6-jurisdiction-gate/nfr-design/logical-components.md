# Logical Components — u6-jurisdiction-gate

**Upstream:** `functional-spec.md`, `nfr-requirements/*`, `components.md`.

## Component inventory inside the Identity service

| Logical component | Responsibility | Depends on | Failure domain |
|---|---|---|---|
| `JurisdictionGate` (domain) | Decision rules BR1.1–BR1.10; list/version value objects; no I/O | Nothing | Pure logic; cannot fail externally |
| `CheckJurisdiction` (application) | Orchestrates validate → provider → list → decide → audit; enforces the budget and the fail-closed path | Domain, ports (`GeoLocator`, `ListRepository`, `AuditSink`, `Clock`) | The unit's main failure surface |
| `PublishList` (application) | Validates, checksums, stores a new immutable version; audits | Domain, `ListRepository`, `AuditSink` | Write path; isolated from the read path |
| `PgListRepository` (infrastructure) | Reads/writes `jurisdiction_list_versions` and `jurisdiction_rules` in the Identity schema | Postgres | Database failure |
| `HttpGeoLocator` (infrastructure) | Provider client with the 800 ms/one-retry budget and circuit breaker | External provider | Provider failure |
| `HttpAuditSink` (infrastructure) | Audit event write with 300 ms/one-retry/idempotency key | Audit service | Audit failure |
| `JurisdictionHttp` (http) | Two routes: check and publish; DTO mapping; RFC 7807 problems; metrics | Application | Request boundary |

## Failure domains and blast radius

- **Provider failure** → affected checks return `503`; sign-ups and continued use pause. Blast radius: every sign-up and continued-use check, not other identity operations.
- **Audit failure** → affected checks return `503`; no decision is completed without evidence. Blast radius: the same set.
- **List unreadable** → the replica reports not-ready; the swarm stops routing to it. Blast radius: that replica.
- **Publish failure** → the previous version stays active; checks are unaffected. Blast radius: none.

## Isolation strategy

- The gate is one module of the Identity service, not a separate deployable; its failure domain is bounded by its routes and its dependencies, and it holds no shared mutable state with other units.
- Its database tables are namespaced (`jurisdiction_*`) inside the Identity schema, so another unit's migration cannot touch them.
- The read path (check) and the write path (publish) share only the repository; a publish cannot block a check beyond the refresh interval.

## Shared resources

| Resource | Shared with | Isolation |
|---|---|---|
| Identity Postgres schema | Other Identity units | Separate tables and a dedicated repository; no cross-unit table access |
| Audit service | All services | HTTP client with its own budget; failures are local to the gate |
| API Gateway | All services | Route declarations only |
