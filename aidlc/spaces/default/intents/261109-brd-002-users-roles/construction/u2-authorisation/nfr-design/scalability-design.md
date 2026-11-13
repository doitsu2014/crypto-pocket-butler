# Scalability Design — u2-authorisation

**Upstream:** `scalability-requirements.md` (NFR20.1–NFR20.4), `performance-design.md`, `functional-spec.md`.

## Scaling architecture

- **Stateless.** No session, no cross-replica counter, no shared cache. A replica is added, not
  grown (NFR20.2).
- **Scale-out unit.** One Identity API replica, 0.5 CPU / 512 MB as configured; the C4 endpoint
  scales with it because it is the same process.
- **Data partitioning.** None. The unit owns no data. The status read is a single indexed row on
  the caller's own schema; the audit store partitions itself.

## Capacity thresholds

| Threshold | Value | Action |
|---|---|---|
| Sustained CPU per replica | 70% | Add a replica (NFR20.3) |
| Decision p95 latency | Above the path budget for 10 minutes | Investigate before scaling; a latency rise with a warm dependency is a code problem |
| Status-read error rate | > 1% | The dependency is degrading; the outage cache absorbs the blip |
| Cache size | 10 000 entries, LRU | The cap is a memory guard, not a capacity plan; normal traffic reads status |

The 2 000 decisions/s per replica figure is an **assumption from the configured replica count
and CPU**, recorded as such in `scalability-requirements.md`; Build and Test's first real traffic
is the evidence that replaces it (NFR20.4).

## Load distribution

- Requests are distributed by the platform (Swarm routing mesh) across the Identity replicas; the
  decision adds no affinity requirement because it holds no per-user state.
- The C4 endpoint's callers keep one pooled client each (2–10 connections), so a burst from one
  caller is bounded at that caller.

## Growth

- Cost per decision is constant in users, resources, and grants (NFR20.1): no table to scan, no
  per-user index to warm.
- The first growth lever is replica count. The second would be a shared cache, and it is
  deliberately not taken: a distributed cache adds a failure mode to solve a problem the
  outage-only cache already bounds.
