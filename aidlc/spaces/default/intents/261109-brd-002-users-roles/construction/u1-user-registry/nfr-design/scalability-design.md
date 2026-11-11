# Scalability Design — u1-user-registry

**Upstream:** `scalability-requirements.md` (NFR20.1–NFR20.4), `functional-design/functional-spec.md`.

## Scaling approach

- **Stateless replicas** behind the gateway; all state in the Identity Postgres schema.
- **Indexed lookups** by subject hash and internal id; admin exact match on email.
- **Horizontal scaling** adds throughput linearly; 500 resolutions/s per replica is the design point.

## Capacity thresholds

| Resource | Limit | Action |
|---|---|---|
| Users | 1M without schema change | Partition `users` by hash prefix only beyond 10M |
| Resolution throughput | 500/s per replica | Scale replicas; the database is the shared limit |
| Provisioning writes | One row per new user | Idempotent under retry |

No cache tier is introduced: an indexed primary-key read is already within the latency target, and a cache would add a consistency problem for status changes (NFR24.1).
