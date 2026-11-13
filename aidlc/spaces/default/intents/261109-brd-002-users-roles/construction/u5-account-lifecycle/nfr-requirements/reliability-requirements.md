# Reliability Requirements — u5-account-lifecycle

**Upstream:** `functional-spec.md` (W4), `rules.md` (BR5.5, BR5.6), `requirements.md` (NFR18).

## Availability

| ID | Requirement |
|---|---|
| NFR1.1 | The user-facing operations are available whenever the Identity service and Postgres are; one dependency. |
| NFR1.2 | A transition that cannot write its outbox row fails the whole transaction (NFR19.2) — fail closed, never an unaudited status change. |
| NFR1.3 | The sweep's failure never blocks a user operation: it uses a lease, not a held lock, and retries with backoff. |
| NFR1.4 | The service's existing `/readyz` reports the database; the sweep's backlog is an alert, not a readiness condition. |

## Fault tolerance

| Failure | Effect | Behaviour |
|---|---|---|
| Postgres read/write down | All operations fail | 503; nothing partial |
| C9 down | Outbox grows | Nothing user-visible; the delivery alert fires |
| A cascade step fails | Request stays open | `attempt_count + 1`, backoff 1/2/4/8/16 min in `next_attempt_at`; the claim query selects only rows with `next_attempt_at <= now()`; after 5 attempts the row moves to **`status = 'failed'`** (a terminal, parked state) and alerts |
| The worker dies mid-erasure | The lease expires | Another pass reclaims and re-runs; every step is idempotent (BR5.6) |
| A restored account races the sweep | The sweep re-reads and skips | The request is closed as cancelled, never re-claimed forever |

## Idempotency and recovery

| ID | Requirement |
|---|---|
| NFR18.1 | Every cascade step deletes by user id and tolerates an absent row; a second full run over an erased request is a no-op that reports success (BR5.6). |
| NFR18.2 | The tombstone anonymisation is itself idempotent: re-running it leaves the same cleared fields and the same surviving identifiers. |
| NFR18.3 | Rows live in the Identity schema under the platform's Postgres backup (RPO ≤ 24 h, RTO ≤ 4 h). A restore mid-erasure is safe because every step is idempotent; the request row's `attempt_count` and `erased_at` are the resume point. |

## Failure-mode checklist

| Question | Answer |
|---|---|
| Unavailable dependency? | Postgres (fail closed) and C9 (deferred through the outbox). |
| Response time doubles? | The latency alert fires at the budget. |
| Throughput exceeds capacity? | Batch size and replicas; no state to migrate. |
| Corrupted data? | A status outside the CHECK is refused by the schema; a transition whose edge is illegal is refused by the machine. |
| Graceful path? | None for a write; the sweep degrades by retrying, never by skipping silently. |
| Blast radius? | One account per transition; the sweep is per-account and leased. |
| Recovery? | Platform restore; the sweep resumes from the request row. |
