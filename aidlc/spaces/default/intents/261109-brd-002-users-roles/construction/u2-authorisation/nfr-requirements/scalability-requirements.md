# Scalability Requirements — u2-authorisation

**Upstream:** `functional-spec.md` (the in-process library and the C4 endpoint), `requirements.md` (NFR24), `contract-summary.md` (C4).

## Load projection

| Dimension | Target | Basis |
|---|---|---|
| Decisions per request | exactly one | Every authenticated request passes through W1 once |
| Identity API replicas | 3 (as configured in `infra/swarm-stack.yml`) | Existing stack; no change for this unit |
| Peak decisions per replica | **assumption: 2 000/s at 0.5 CPU** | Estimated from the configured replicas and CPU, **not measured**; Build and Test's first real run is the evidence, and a sustained reading at 70% of this figure is the trigger to add a replica (NFR20.3) |
| Status reads | one per decision in normal operation, none while the outage cache is warm | NFR24.1 |

## Scaling behaviour

- **Stateless.** The decision keeps no session and no shared counter. Adding a replica is the
  only scaling action (NFR20.2).
- **Outage cache only.** The per-process cache exists to survive a status-read blip, not to
  serve normal traffic. It is capped at **10 000 entries with least-recently-used eviction**;
  above the cap a new user simply misses and reads status, which is the normal path anyway.
- **C4 endpoint.** A thin adapter over the same in-process code: its capacity is the Identity
  API's capacity.
- **Role changes** are administrative and low-volume; they are not separately scaled.

## Capacity planning and growth

- The decision's cost does not grow with the number of users, resources, or grants: there is no
  per-user table to scan (NFR20.1).
- The 2 000 decisions/s figure is an assumption to confirm at Build and Test; until then it
  sizes the cache and the pool, not a promise (NFR20.4).
- Growth is handled by replica count, not by a larger cache or a shared tier.

## Concurrency

- The decision is read-only except for the audit write and the provider role write; neither is
  serialised across requests.
- Two concurrent role changes for the same user are last-writer-wins at the provider, and both
  audit entries are recorded, so the trail shows both attempts and their outcomes.
- A cache refresh under concurrent misses reads status at most once per user per window; a
  duplicate read is acceptable and produces the same result.
