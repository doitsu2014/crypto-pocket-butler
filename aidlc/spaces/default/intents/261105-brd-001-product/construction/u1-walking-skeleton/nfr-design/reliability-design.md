# Reliability Design — u1-walking-skeleton

## Resilience patterns

| Pattern | Where | Configuration |
|---|---|---|
| Timeout | Every internal call | connect 2 s, read 5 s, total 10 s |
| Retry with backoff | Idempotent reads and keyed audit appends | 3 attempts, exponential backoff (100/200/400 ms) with full jitter |
| Circuit breaker | Gateway → registry; gateway → Identity; registry → audit | open after 5 consecutive failures, 30 s open, 3 half-open probes |
| Bulkhead | Per-dependency connection pools | separate sqlx pool and HTTP client per dependency |
| Graceful degradation | Registry unavailable | callers keep last-known identity and label it stale |
| Fail closed | JWKS unavailable beyond the staleness window | 503, never unvalidated routing |

## Health checks

- **Readiness:** process up, database reachable, JWKS loaded (gateway), cache warm-or-empty.
- **Liveness:** process responsive; no downstream dependency.
- Deployment health check gates the Swarm rolling update; a failing check rolls back.

## Recovery

- RPO ≤24 h, RTO ≤4 h for the registry schema (Supabase PITR where configured).
- Restore drill before launch; the drill is recorded.
- Migrations are versioned up/down pairs; a rollback applies the down migration.

## Failure modes

| Failure | Blast radius | Behaviour |
|---|---|---|
| One gateway replica dies | single replica | Swarm restarts it; remaining replicas serve |
| One registry replica dies | single replica | same |
| Registry database unavailable | whole registry | 503; callers degrade to last-known identity |
| JWKS endpoint unavailable | gateway | cached keys serve up to 10 min, then 503 |
| Cache invalidation poll fails | single replica | 60 s TTL bounds staleness |
| Audit endpoint unavailable | audit trail | bounded retry, then local structured log and an error metric |

## JWKS staleness window

Cached keys serve for **10 minutes** after the last successful fetch; at the end of the window the
gateway fails closed with 503. The test advances the clock past 10 minutes with the JWKS endpoint
down and asserts the 503.
