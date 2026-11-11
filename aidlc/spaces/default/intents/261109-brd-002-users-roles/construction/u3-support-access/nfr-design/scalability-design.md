# Scalability Design — u3-support-access

**Upstream:** `scalability-requirements.md` (NFR20.1–NFR20.9), `functional-spec.md`,
`tech-stack-decisions.md`, `memory/team.md` (replicated Swarm services).

## Scaling model

Support access is staff traffic at a projected ≤ 1,000 grants/day and ≤ 20,000 reads/day. The
design scales by **adding Identity replicas**, not by sharding or caching:

- Every route is stateless: the role comes from the request's token, the grant from the database,
  and nothing is held in process between requests.
- The service runs as the Swarm stack's existing replicated Identity service (3 replicas for the
  identity API). A replica restart loses nothing; no request is pinned to a replica.
- The database is the single shared resource. At the projected volume it is nowhere near its
  write capacity — the requirement is that the write path takes **one row's lock at most** and
  never scans.

## Correctness under concurrency

The concurrency design is all in the schema and the statements, not in the application:

| Race | Mechanism |
|---|---|
| Two agents request a grant for the same target | Partial unique index on `(support_agent_id, user_id) WHERE status IN ('pending_approval','active')`; the loser takes the unique violation and answers 409 (NFR20.1) |
| Two admins approve the same grant | Conditional `UPDATE … WHERE id = :id AND status = 'pending_approval'`; zero rows affected → 409 (NFR20.2) |
| A revoke races an expiry or a second revoke | Conditional `UPDATE` on the current status → 409 (NFR20.3) |
| A stale pending request races its own expiry | The expiry and the insert share one transaction and touch only that (agent, user) row pair (NFR20.4) |
| N replicas drain the same outbox | `FOR UPDATE SKIP LOCKED` plus the 30 s claim lease (NFR20.5) |

There is no application-level lock, no advisory lock, and no serializable isolation anywhere in
the unit: each race is decided by one statement.

## Data layout

- `support_access_grants`: primary key on `id`, which is how the metadata read loads its grant; the
  existing `support_grants_read_idx` on `(user_id, status, expires_at DESC)` serves the per-user
  lookup and the live-grant evaluation over a handful of rows; and the new partial unique index.
  No second user index is added and none is dropped (NFR20.7).
- `support_audit_outbox`: mirrors `consent_audit_outbox` — a partial index on
  `(next_attempt_at) WHERE state = 'pending'` for the claim and one on
  `(delivered_at) WHERE state = 'delivered'` for the 24-hour cleanup. Delivered rows are deleted, so
  the table stays at roughly one day of volume (NFR20.8).
- **Grant rows survive erasure.** NFR20.6 allows deletion only with the target user's account, and
  this deployment has no such deletion: `identity_app` holds `SELECT, INSERT, UPDATE` on
  `support_access_grants` and no `DELETE`, users are anonymised in place rather than removed under
  migration 0006, and the erasure path issues no grant delete. The `user_id` foreign key therefore
  never fires, and the grant row stays as identifier-only evidence. That is the requirement's own
  position — grant history is evidence, not personal data — and it is why ~365,000 rows/year needs
  no partitioning and no archival.

## Capacity thresholds

| Signal | Threshold | Response |
|---|---|---|
| Read p95 / p99 | above 150 / 300 ms for 10 min | Investigate the slow source or the database; **not** a cache (NFR20.9's rejection) |
| Source error rate | > 0.2% per source over 10 min | The owner's incident, not Identity's scale problem |
| Outbox oldest age | > 15 min | Check the drainer and the audit service |
| DB write latency | support writes p95 above 80 ms | Check the pool and long transactions; the unit adds no second pool |

No autoscaling rule is introduced: the projected load does not approach the service's capacity, and
a rule would react to a signal that is far below the point where scaling helps. The read histogram
and the source-error counters are the evidence that says whether the projection was right.

## Load distribution

The gateway distributes each request independently; the metadata read fans out to two owners
concurrently. The fan-out is bounded to two in-flight calls per request, so Identity's outbound
connections to Account and Portfolio grow linearly with read concurrency and never exceed the
number of in-flight requests. The gateway's per-route limit (NFR11.1) caps that concurrency at the
source, which is why the rate limit is part of the scalability story and not only the security one.