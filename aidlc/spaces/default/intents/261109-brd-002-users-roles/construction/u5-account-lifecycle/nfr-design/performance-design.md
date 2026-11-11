# Performance Design — u5-account-lifecycle

**Upstream:** `performance-requirements.md` (NFR2.1–NFR2.6), `functional-spec.md` (W1–W5), `tech-stack-decisions.md`.

## Decisions

| Decision | Choice | Why |
|---|---|---|
| Admin disable/restore, request, restore | One transaction: `users.status`, the transition insert, the outbox insert | Three writes, one round trip; the response waits for the outbox row so a status change is never unrecorded |
| Status read for u2 | None added — u2 reads the `users` column it already reads | No hop on the hot path |
| Sweep claim | The existing lease (`lease_owner`, `lease_until`), batch of 100, `WHERE next_attempt_at <= now()` | A lease is cheap to acquire and cannot be held across HTTP calls the way a row lock can |
| Cascade | Sequential per account, each step bounded by a client timeout | Sequential keeps the per-account duration under the lease and makes a failed step attributable |
| Batch sizing | 100 × p95 400 ms ≈ 40 s, inside the 60 s interval | The p95, not the hard ceiling, sizes the batch |

## Budgets as designed

| Path | Budget |
|---|---|
| Admin disable/restore | p95 ≤ 60 ms |
| Deletion request | p95 ≤ 60 ms |
| Owner restore | p95 ≤ 60 ms |
| Sweep, per account | p95 ≤ 5 s, ceiling 60 s (lease 5 min with heartbeat) |
| Sweep, batch of 100 | p95 ≤ 60 s; the batch is sized on the p95, and a batch that overruns the interval is a sizing signal |

## What is deliberately absent

No cache of the status (freshness is a requirement), no parallel cascade per account (a parallel
fan-out would make the per-account duration unbounded and the failure attribution useless), and no
background prefetch of due requests — the claim query is indexed and cheap.
