# Scalability Design — u3-identity

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Scaling architecture

- Stateless service: three replicated Swarm tasks behind the gateway; no session affinity, no
  in-memory session or token cache. Any request can be served by any task.
- Horizontal scale-out adds a task; scale-in removes one. The rolling update is health-checked.
- The worker is a separate process (or a role in the service) that claims records; more workers can
  run because the lease compare-and-set serializes each record.

## Load distribution

| Figure | Value |
|---|---|
| Registered users | 10,000 |
| Peak concurrent sessions | 200 |
| Peak principal-resolution rate | 50 rps |
| Replicas | 3 |
| Pool per replica | 20 connections (60 total) |
| Scale-out trigger | p95 principal > 50 ms for 5 min, or CPU > 70% for 5 min |

## Data partitioning

- No partitioning at launch. `User` and `ConsentRecord` are keyed by user and small; `DeviceSession`
  is pruned when a session ends; `ExportRequest`/`DeletionRequest` are bounded by user activity.
- If the user base outgrows the launch review, the first lever is the connection budget and
  replicas, then time-based partitioning of `DeviceSession` activity rows.

## Concurrency and leases

- `ExportRequest` and `DeletionRequest` each carry `leaseOwner` and `leaseUntil` (60 s).
- Claim: `UPDATE ... SET leaseOwner = :me, leaseUntil = now() + 60s WHERE id = :id AND (leaseOwner
  IS NULL OR leaseUntil < now())` — one row, atomic.
- Re-claim after a crash: the same statement on the expired lease. Two workers cannot both update
  the row, so no double execution.
- At most one active deletion and one `building` export per user; a second request returns the
  active one.

## Capacity thresholds

- The launch capacity review validates the figures above against the Supabase plan and the gateway
  rate limits and records the result.
- Auto-scaling is the Swarm service's replica policy driven by the trigger above; no custom
  autoscaler.

NFR references: NFR1.1, NFR1.2, NFR20.1, NFR20.2, NFR20.3, NFR20.4.
