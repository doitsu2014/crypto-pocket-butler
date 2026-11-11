# Scalability Design — u6-jurisdiction-gate

**Upstream:** `scalability-requirements.md` (NFR20.1–NFR20.6), `functional-spec.md`.

## Scaling approach

- **Stateless replicas behind the gateway.** Each replica holds the active list version in memory and refreshes it every 10 s; no request depends on another replica's state.
- **Horizontal scaling.** Adding replicas multiplies check throughput linearly (200 checks/s per replica at the p95 target); the swarm scales the Identity service by replica count.
- **No partitioning.** The list is small (≤ 300 entries) and identical on every replica, so it is copied, not sharded.

## Capacity thresholds

| Resource | Limit | Action at the limit |
|---|---|---|
| In-memory list | 300 entries, ~30 KB | A larger list triggers a size check at publish; beyond 10× the design moves to a per-country lookup query |
| Provider calls | 600/min per replica | The client queues and sheds above the quota; shed requests return `503` (fail closed) |
| Decision cache | Bounded map, 10 000 entries, 5-minute TTL | Eviction is LRU; the cache never holds a raw IP |

## Replica refresh

Because the project forbids message queues, a publish cannot be broadcast. Each replica re-reads the active version on a 10 s timer and on startup; the maximum staleness after a publish is 10 s, and a replica serving a version older than 30 s raises the warning alert. A publish handled by one replica therefore propagates within the interval.

## Growth

At ten times the launch list size the stored model is unchanged; only the refresh payload grows. The unit's database load stays flat between publishes.
