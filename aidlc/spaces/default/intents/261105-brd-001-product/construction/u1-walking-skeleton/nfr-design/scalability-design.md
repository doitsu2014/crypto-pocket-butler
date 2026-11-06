# Scalability Design — u1-walking-skeleton

## Scaling model

- **Gateway and registry:** stateless replicas behind the Swarm routing mesh; scale horizontally
  by adding replicas. No sticky sessions.
- **Registry reads:** linear with replicas; the in-process cache multiplies effective read
  capacity.
- **Writes:** registry administration is low volume; a single writer path is sufficient.

## Capacity thresholds

| Signal | Threshold | Action |
|---|---|---|
| Gateway CPU | >70% sustained 10 min | add a gateway replica |
| Registry request rate | >70% of measured per-replica capacity | add a registry replica |
| Cache hit ratio | <80% | investigate key cardinality or invalidation churn |
| Quarantine rows | >10 000 per account | block new quarantine writes for that account and surface to the operator |

## Data growth

- The registry holds assets, chains, contracts, and sources: a small, slow-growing set.
- Bounded at 10× the launch set without a schema change; growth beyond that is a schema review,
  not an outage.
- Quarantine is the only unbounded table; per-account cap plus operator review bounds it.

## Launch capacity

Launch capacity is defined by the platform work as **300 RPS sustained for 60 minutes** at the
edge (the C9–C10 launch gate). This Unit's share is the registry read path at that rate; the
load test measures p95 at 300 RPS.

## Replica sizing

- Gateway: 2 replicas minimum, 4 at launch.
- Registry: 2 replicas minimum, 4 at launch.
- Each replica: 512 MiB memory, 0.5 vCPU, with the cache bounded at 10 000 entries.
