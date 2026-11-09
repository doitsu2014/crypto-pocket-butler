# Performance Design — u3-identity

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Caching

| Cache | What | Invalidation |
|---|---|---|
| JWKS cache | Supabase public keys, keyed by `kid` | Refreshed when an unknown `kid` arrives; periodic refresh at 1 hour |
| No principal cache | The principal is resolved per request from the validated JWT and one indexed lookup | n/a |
| No consent cache | Consent is read from the newest record; a short request-scoped read is enough | n/a |

Identity caches public keys only. It never caches a token, a principal, a consent decision, or a
session.

## Query and index design

- `User(authUserId)` unique index: the single hot-path lookup.
- `ConsentRecord(userId, purpose, recordedAt desc)`: newest decision per purpose.
- `DeviceSession(userId, lastActivityAt desc)` and unique `(userId, authSessionId)`.
- `ExportRequest(status, leaseUntil)` and `DeletionRequest(status, leaseUntil)`: worker claims.
- `SupportAccessGrant(userId, status, expiresAt)`: the read-path grant check.

## Connection pooling

- sqlx pool per replica: 20 connections, 5 kept warm, 5 s acquire timeout, 10 min idle, 30 min max
  lifetime. Three replicas keep the total at 60, inside the Supabase budget.

## Async processing

- Export and deletion run in the worker, not on the request path. A request wakes the worker; the
  sweep cadence is 60 s and a claimed record holds a 60 s lease.
- Worker outbound calls: connect 1 s, read 5 s, total 30 s, at most 3 attempts with jitter.

## Performance budgets

| Path | Budget |
|---|---|
| `resolvePrincipal` | ≤ 50 ms p95, including one indexed lookup and the JWKS check |
| `getConsent` | ≤ 50 ms p95 |
| Consent / preference write | ≤ 150 ms p95 |
| Session list | ≤ 400 ms p95 (Identity view; optional Auth reconciliation) |
| Support-read Audit append | ≤ 100 ms p95, 200 ms timeout |
| Export first progress | ≤ 2 s after the request wakes the worker |
| Deletion acknowledgement | ≤ 200 ms; execution in the worker |

## Frontend interaction

The SPA never blocks on a background job: it polls the export or deletion status and renders
progress. The polling port is the explicit, typed `key=value` parsing of the request/response DTOs
defined in the functional specification's operations section.

NFR references: NFR2.1, NFR2.2, NFR2.3, NFR2.4, NFR6.1, NFR13.3, NFR13.5, NFR16.1, NFR16.2.
