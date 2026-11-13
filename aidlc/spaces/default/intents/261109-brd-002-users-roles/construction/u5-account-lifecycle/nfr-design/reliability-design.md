# Reliability Design — u5-account-lifecycle

**Upstream:** `reliability-requirements.md` (NFR1.1–NFR1.4, NFR18.1–NFR18.3), `security-design.md`, `contract-summary.md` (C9).

## Pattern choices

| Pattern | Applied? | Configuration | Why |
|---|---|---|---|
| Transaction boundary | Yes | Status + transition + outbox in one transaction | A status change is never accepted without its audit intent |
| Lease-based claim | Yes | Existing `lease_owner`/`lease_until`, batch 100 | A lease survives a crash without a held lock |
| Retry with backoff | Sweep only | `next_attempt_at` = now + 1/2/4/8/16 min; claim filters `next_attempt_at <= now()` | The bound is data, not hope |
| Terminal parked state | Yes | After 5 attempts, `status = 'failed'`, `attempt_count` kept | A stuck erasure is visible, not re-picked forever |
| Idempotent cascade | Yes | Every step deletes by id and tolerates absent rows | A retried run must be safe |
| Timeout | Yes | Each cascade HTTP call bounded by the contract's connect/read budget | One slow service cannot hold the lease open |
| Circuit breaker | No | — | The cascade is retried by the sweep, not by a breaker |
| Fail-closed | Yes | A transition that cannot record its outbox row is refused | Never an unaudited status change |

## The sweep

```
claim:   SELECT … FROM deletion_requests
         WHERE status IN ('requested','in_progress')
           AND next_attempt_at <= now()
           AND recoverable_until <= now()
           AND (lease_until IS NULL OR lease_until < now())      -- a live lease is never re-claimed
         ORDER BY recoverable_until
         LIMIT 100
         FOR UPDATE SKIP LOCKED
         -- update lease_owner, lease_until = now + 5 min, status = 'in_progress'
         COMMIT
per row: re-read status under SELECT … FOR UPDATE → if not pending_deletion, cancel and skip
         Auth delete → guarded tombstone update → user_personal, key, consent, preferences
         on failure: attempt_count + 1, next_attempt_at = backoff; at 5 → status = 'failed'
```

The claim commits before any cascade work, so a long cascade never holds the row lock; the
`lease_until` predicate is what stops a second pass re-claiming a row another worker still owns.

**What "parked" means.** After five attempts the row moves to `status = 'failed'` **and sets
`cancelled_at`**, so it is no longer open: the unique open-request index stops blocking the user and
a new deletion request is accepted. The account stays `pending_deletion` until an operator resolves
the stuck erasure from the `failed` index — the recovery path is "operator looks at
`status='failed'`, fixes the failing step, re-opens the row", stated here rather than left as a
permanent lock-out. Parking without closing would leave the user read-only and unable to restore
(the window has closed) or re-request (409), which is the failure this rule exists to avoid.**Timeouts and the lease are sized together.** Each cascade HTTP call uses the contract's connect
5 s / read 10 s, and a step's retries are bounded to **one attempt per pass** (the backoff is the
retry mechanism, not an inner loop), so a step costs at most ~15 s. The per-account ceiling is
therefore **60 s**, not 3 s, and the lease is 5 minutes with a heartbeat renewal every 60 s — a
ceiling and a lease that cannot collide.

## Health checks

The service's existing `/healthz` and `/readyz`. The sweep's liveness is a metric
(`deletion_sweep_last_pass_timestamp_seconds`) with an alert, not a readiness failure — a stuck
sweep must not take the API out of rotation.

## Degradation matrix

| Failure | Effect | Behaviour |
|---|---|---|
| Postgres down | All operations fail | 503 |
| C9 down | Outbox grows | Delivery alert at 5 minutes |
| A cascade step down | Request retried | Backoff, then `failed` and an alert |
| Worker down | Nothing erased | The sweep-liveness alert fires |
| Restore racing the sweep | The sweep skips | The request is cancelled |

## Recovery

Rows live in the Identity schema under the platform's backup (RPO ≤ 24 h, RTO ≤ 4 h). A restore
mid-erasure is safe: every step is idempotent, and `attempt_count`/`erased_at` are the resume point.
