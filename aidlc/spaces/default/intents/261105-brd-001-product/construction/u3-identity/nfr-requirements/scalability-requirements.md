# Scalability Requirements — u3-identity

Inputs: `functional-design/functional-spec.md`, `functional-design/rules.md`,
`inception/requirements-analysis/requirements.md`, and `inception/contract-design/contract-summary.md`.

## Targets

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR20.1 | Identity scales horizontally with no session affinity | Stateless instances behind the Swarm service; rolling update health-checked | NFR20 |
| NFR20.2 | Principal resolution scales with gateway traffic | Indexed lookup by `authUserId`; no per-request cross-service call | NFR20 |
| NFR20.3 | Export and deletion workers scale without duplicate execution | Status transition is the single-flight guard; extra instances poll the same tables | NFR20 |
| NFR20.4 | Launch capacity is reviewed against Supabase Postgres limits | Connection pool sized from expected concurrent requests; recorded at launch | NFR20 |

## Load projection and scaling trigger

| Figure | Launch bound |
|---|---|
| Registered users | 10,000 |
| Peak concurrent sessions | 200 |
| Peak principal-resolution rate | 50 requests/second at the gateway |
| Identity replicas | 3 (stateless) |
| Postgres pool per replica | 20 connections (60 total, under the Supabase budget) |
| Scale-out trigger | p95 `resolvePrincipal` > 50 ms for 5 minutes, or CPU > 70% for 5 minutes, adds one replica |

## Load and growth

- The dominant load is principal resolution, proportional to gateway request rate.
- Consent, preference, session, export, and deletion traffic is per-user and bounded by the
  authenticated user's own actions.
- The `User`, `ConsentRecord`, `ExportRequest`, and `DeletionRequest` tables grow with the user
  base; `SupportAccessGrant` grows with support activity. No partitioning is planned at launch; the
  indexes on `authUserId` and `(userId, purpose)` are the readers that matter.

## Concurrency

- At most one active `DeletionRequest` per user and at most one `ExportRequest` in `building` per
  user; a second request waits or returns the active one.
- The worker's status transition (`pending` → `building`, `requested` → `in_progress`) and each
  re-claim after an expired lease are compare-and-set operations, so two workers cannot process the
  same record at once.

## Capacity review

- The launch capacity review is a recorded launch gate, not a code change: it checks the figures in
  the load projection above against the deployed replica count, the gateway rate limits, and the
  Supabase connection budget. It scales out when the trigger fires.
