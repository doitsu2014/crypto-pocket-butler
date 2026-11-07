# Scalability Design — u4-account

Inputs: `nfr-requirements/performance-requirements.md`, `security-requirements.md`,
`scalability-requirements.md`, `reliability-requirements.md`, `observability-requirements.md`,
`tech-stack-decisions.md`, and `functional-design/functional-spec.md`.

## Scaling architecture

- Stateless API, 3 replicas behind the gateway; no session affinity.
- Sync and import run in a separate Swarm service, `account-worker` (1 replica, 0.5 vCPU / 512 MiB);
  extra workers poll and compete on the run lease. The API only enqueues and acknowledges.
- Chain metadata is cached for 5 minutes (public data) and refreshed on expiry; no other cache.

## Load distribution

| Figure | Launch bound |
|---|---|
| Users | 10,000 |
| Accounts | ≤ 5/user (50,000) |
| Peak sync workers | 4 |
| C5 read rate | 50 rps |
| API pool | 12/replica (36) |
| Worker pool | 8 |
| Scale-out trigger | C5 p95 > 150 ms 10 min, or pool > 18/20 10 min → add a replica (ceiling 4) |

## Provider rate limits

- Per-venue concurrency limit and exponential backoff; a 429 is reported as `rate_limit` and the
  run retries on the next schedule.
- RPC calls are bounded per chain; a slow chain fails its run without affecting other chains.

## Data growth and retention

- Snapshots and runs retained 90 days, deleted by the same sweep; `ImportRowError` retained with its
  batch for 90 days.
- At launch ≈ 50,000 snapshots/day with a 90-day cap around 4.5M rows.

NFR references: NFR20.1, NFR20.2, NFR20.3, NFR20.4.
