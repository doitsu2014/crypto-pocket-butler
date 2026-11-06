# Audit — Performance Requirements

The audit service sits on the write path of every other service, so its latency is paid by
journeys that have nothing to do with auditing. The targets below are deliberately tighter than
the dashboard's.

| ID | Requirement | Target | Rationale |
|---|---|---|---|
| NFR2.1 | Append latency, server side | p95 < 25 ms, p99 < 60 ms | An append is a single insert plus a validation pass. Every security-event append blocks its caller, so the caller's own budget shrinks by this much. |
| NFR2.2 | Session check, end to end | p95 < 60 ms, p99 < 120 ms, **including the read-event append it performs before answering** | Sits in front of every support read. The budget is set above NFR2.1 by design: the check writes the read event before it answers (BR3.3), so it is a session lookup plus one append, not a lookup alone. Measured as one span, not as two. |
| NFR2.3 | Admin query latency | p95 < 800 ms for a 30-day range with a cursor page of 100 | Operator work is not on a user journey; it may be slower, but not unbounded. |
| NFR2.4 | Export latency | p95 < 60 s for an export of up to 100 000 rows, run **synchronously** on the request that asked for it | Synchronous because no queue is available and no durable worker exists: an accepted-but-never-produced export is worse than a slow one. An export beyond the row cap is refused with a problem document telling the operator to narrow the filter, not quietly truncated. |
| NFR2.8 | Export recovery | An `AuditExport` left in `requested` for more than 10 minutes is marked `failed` with `replica_lost` when the service next starts | Keeps a replica restart from leaving a permanently pending export behind. |
| NFR2.5 | Validation cost | Rejection of a malformed envelope < 5 ms | Validation happens before any storage work and must not dominate the append. |
| NFR2.6 | Throughput | 75 appends per second per replica, so 225 per second at the launch capacity of 3 replicas, with p95 inside NFR2.1 | Stated per replica so it can be checked against the deployment that actually runs. A fourth replica is headroom for a retry storm, not a separate target. |
| NFR2.7 | Retention run impact | Runs stay under 30 minutes and never hold a lock that blocks an append for more than 200 ms | Sweeps delete in batches so the write path never stalls behind them. |

Resource constraints: each replica's Postgres connection pool is 8 connections, sized for the
append path rather than for analytical queries. Admin queries run against the same pool and are
rate-limited so they cannot starve appends.

## Sources

- `construction/u12-audit/functional-design/functional-spec.md` — WF1, WF3, WF4, WF5, WF6
- `construction/u12-audit/functional-design/rules.md` — BR2.1, BR5.1
- `inception/requirements-analysis/requirements.md` — NFR2, NFR17, NFR19
- `inception/contract-design/contract-summary.md` — C10 append and run-endpoint contracts
