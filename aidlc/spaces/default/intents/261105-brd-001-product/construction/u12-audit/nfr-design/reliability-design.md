# Audit — Reliability Design

## Resilience patterns, and where each applies

| Pattern | Where | Configuration |
|---|---|---|
| Timeout | Every database call | Append 2 s; query 10 s; export 60 s total, 10 s per batch |
| Bounded pool + fail fast | Connection pool | 8 per replica; a waiter older than the statement timeout is refused, not queued further |
| Retry with backoff | **Emitter side only, both classes** | 2 retries, 100 ms then 200 ms, jittered. The emitter knows its own event's `sensitivity`, so it knows whether a final failure blocks its action (security) or is logged and dropped (domain). The audit service never retries and never queues |
| Circuit breaker | The emitter's client to Audit | 5 consecutive failures opens for 30 s, then half-open with 3 probes. Security-event appends surface the open circuit to their caller as a failure of the caller's action |
| Idempotency | Append | `(emitting_service, idempotency_key)` unique; same payload returns the original id, different payload answers `409` |
| Health and readiness | Every replica | `/health` shallow; `/ready` checks the database and the migrations table |
| Graceful shutdown | Every replica | Stop accepting, drain in-flight appends for up to 10 s, exit |

No queue, no outbox, no background worker: the project forbids queues, and an in-process retry loop
would either hold the caller or die with the replica. That is why the retry lives with the emitter,
which owns its own request budget.

## Failure behaviour by event class

| Class | Derived from | On storage failure |
|---|---|---|
| Security | `sensitivity: security` | `503` with `type: https://errors.cpb/audit/unavailable`. The emitter retries twice with the same key; if it still fails, the caller's action must not commit |
| Domain | `sensitivity: routine` | Same `503`; the emitter retries twice with the same key, then logs and counts the loss and proceeds |

One status code, one retry rule, one place that decides what a final failure means — the emitter,
which knows whether its own action can survive an unrecorded event. The response body never tells
the emitter how important its event is; that would invite the downgrade BR1.2 exists to prevent.

A caller cannot choose the class: a contradicting `eventClass` is refused with `422`.

## Degradation ladder

1. Export budget shrinks first — exports are the least urgent path.
2. Admin queries are rate-limited down to 5 per minute.
3. Appends keep their full 5-connection budget; nothing else may take a connection from it.

If the database is unreachable, readiness fails, the Swarm stops routing to the replica, and the
remaining replicas report the outage rather than pretending to serve it.

## Recovery

| Scenario | Recovery |
|---|---|
| Replica lost mid-export | The `AuditExport` row stays `requested`; on the next service start, rows older than 10 minutes are marked `failed`/`replica_lost` |
| Retention run killed mid-sweep | Deleted batches stay deleted; the row records `failed`; the next run recomputes its cutoff |
| Database restored from backup | Verify the newest `recordedAt` and a known event id from a sample export before declaring the restore good (NFR18.2) |
| Migration failure | The down migration runs; the Swarm rolls back to the previous image digest |

RPO ≤ 24 h and RTO ≤ 4 h come from the shared Supabase backup regime; the restore drill is part of
the runbook, and the service's part is the verification above.

## Sources

- `construction/u12-audit/nfr-requirements/reliability-requirements.md` — NFR1.1 to NFR18.3
- `construction/u12-audit/nfr-requirements/performance-requirements.md` — NFR2.8
- `construction/u12-audit/nfr-requirements/tech-stack-decisions.md` — no queue, idempotency
- `construction/u12-audit/functional-design/functional-spec.md` — WF1, WF5, WF6
- `inception/contract-design/contract-summary.md` — C10, C11
