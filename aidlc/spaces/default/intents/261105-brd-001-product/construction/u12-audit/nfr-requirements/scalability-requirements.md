# Audit — Scalability Requirements

## Load projection

The append rate is driven by the rest of the system, not by operators. At launch:

| Source of load | Estimated volume |
|---|---|
| Security events (sign-in, credential change, support access, admin change) | low hundreds per day |
| Domain events (portfolio, ledger, notification, job runs) | low tens of thousands per day |
| Periodic job and retention events | hundreds per day |
| Admin queries and exports | tens per day |

Peak append rate is estimated at 20 events per second, an order below the 200 per second the
performance requirements leave headroom for.

## Requirements

| ID | Requirement |
|---|---|
| NFR20.1 | The service scales horizontally: every replica is stateless, holds no in-process cache that another replica would need, and can serve any request. |
| NFR20.2 | The audit tables are append-dominant. Indexes are created for the query shapes that exist — action prefix, target, `occurredAt` range, `sensitivity`, and `recordedAt` for retention — and no index is added speculatively. |
| NFR20.3 | Table partitioning by `recordedAt` month is designed in from the first migration so a retention sweep drops or deletes one partition at a time instead of scanning the whole table. Partitioning is declared in the schema even while there is only one partition. |
| NFR20.4 | Data growth is bounded by retention: at 20 events per second sustained, six months is roughly 320 million rows, so the partition strategy is what keeps sweeps and queries predictable rather than a later optimisation. |
| NFR20.5 | Retention runs delete in batches of 10 000 rows with a pause between batches, so a sweep never holds a long transaction against the write path. |
| NFR20.6 | The export path streams from a cursor into the service's own storage, never materialising the whole result set in memory, and draws from **its own connection budget**: at most 2 of the replica's connections, released between batches, with the same 10-second statement timeout as the query path. An export therefore cannot starve the append path, which is the outcome NFR1.9 exists to prevent. |
| NFR20.7 | Launch capacity: 3 replicas, 0.5 vCPU and 512 MB each. Connection budget per replica: 8 total, of which 5 for appends, 1 for operator queries, and 2 for exports. |
| NFR20.8 | Database capacity: the audit schema is expected to reach roughly 320 million rows and needs a Supabase instance sized for it — the launch instance class and its connection ceiling are recorded here as a review item against Supabase's published limits before launch, together with the retention service's separate 2-connection budget. The claim that capacity is reviewed is a launch gate, not a deferral: no release proceeds while the instance class is unrecorded. |

## Sources

- `construction/u12-audit/functional-design/functional-spec.md` — WF4, WF5, WF6
- `construction/u12-audit/functional-design/rules.md` — BR4.1, BR4.2, BR5.2
- `inception/requirements-analysis/requirements.md` — NFR14, NFR20
- `inception/contract-design/contract-summary.md` — C10 query and export operations
