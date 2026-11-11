# Scalability Design — u5-account-lifecycle

**Upstream:** `scalability-requirements.md` (NFR20.1–NFR20.3), `performance-design.md`, `tech-stack-decisions.md`.

## Scaling architecture

- **Stateless request paths** — the three user operations are single transactions (NFR20.1).
- **Bounded, leased batches** — the sweep claims at most 100 due requests per pass; a backlog
  lengthens the number of passes, never one pass (NFR20.2).
- **Constant cost per read** — every read is by `user_id`, or by the sweep's
  `(status, next_attempt_at)` index (NFR20.3).

## Indexes (migration 0006)

| Index | Purpose |
|---|---|
| **Drop** `0001`'s `deletion_requests_one_active_per_user` (keyed on `status`) | It encodes the old status vocabulary; keeping it beside the new rule is contradictory |
| `deletion_requests (user_id) WHERE cancelled_at IS NULL AND erased_at IS NULL` — **UNIQUE** | The one-open-request rule, in the design's own terms; the unique constraint is what enforces it |
| `deletion_requests (status, next_attempt_at) WHERE status IN ('requested','in_progress')` | The sweep's due-row claim |
| `deletion_requests (status) WHERE status = 'failed'` | Finding parked rows for operator triage; **not claimed by the sweep** |
| `users.erased_subject_hash` — partial, `WHERE erased_subject_hash IS NOT NULL`, built `CONCURRENTLY` | The tombstone lookup the u1 precondition makes on every subject miss; without it every deleted-subject request scans `users` |
| `status_transitions (user_id, occurred_at DESC, seq DESC)` | The newest transition per user |
| `status_audit_outbox (next_attempt_at) WHERE state = 'pending'` | The drainer's claim |

## Capacity thresholds

| Threshold | Value | Action |
|---|---|---|
| Due backlog | > 500 past-deadline open requests for 15 minutes | Alert; the sweep is not keeping up |
| Batch duration | p95 > 60 s | Reduce the batch or add a replica |
| `status_transitions` | 50 M rows | Review partitioning; not expected at 2–8 per account |

## Concurrency

- The lease makes a request claimable by one worker; a crashed claimer's lease expires.
- A restore and the sweep race: the sweep re-reads the status inside the lease and closes a
  restored request as cancelled (NFR24.2).
- Two admin operations: last-writer-wins on the status, and the edge check refuses the second when
  the first moved the status out of its `from` value.
- The transition insert and the status update share a transaction, so no reader sees a status
  without its log row (NFR24.1).
