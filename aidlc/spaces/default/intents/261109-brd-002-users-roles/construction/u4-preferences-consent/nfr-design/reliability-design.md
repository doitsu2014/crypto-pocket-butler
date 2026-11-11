# Reliability Design — u4-preferences-consent

**Upstream:** `reliability-requirements.md` (NFR1.1–NFR1.6, NFR18.1, NFR24.1), `security-design.md`, `contract-summary.md` (C6, C9).

## Pattern choices

| Pattern | Applied? | Configuration | Why |
|---|---|---|---|
| Timeout | Yes | The Identity pool's real bounds are `acquire_timeout` **5 s** and `statement_timeout` **10 s** (`services/identity/infrastructure/src/db.rs`). The withdrawal transaction sets `SET LOCAL statement_timeout = '2s'` so a lock wait and its work cannot run to the pool's 10 s ceiling; the state query sets `SET LOCAL statement_timeout = '100ms'`, matching u2's pattern | The pool bound alone leaves a withdrawal able to hold for seconds against an 80 ms p95 |
| Transaction boundary | Yes | A withdrawal's advisory lock, version read, insert, and outbox insert are one transaction | State and audit intent commit together |
| Outbox + drainer | Yes | Rows carry `state` (`pending`/`delivered`/`dead`), `attempts`, `next_attempt_at`; selected `WHERE state = 'pending' AND next_attempt_at <= now()` with `FOR UPDATE SKIP LOCKED` | Retry state is what makes "three attempts" mean something |
| Retry with backoff | Drainer only | Three attempts, exponential backoff with jitter, on transport errors and 5xx only; a 4xx marks the row `dead` immediately and alerts | A retried post cannot duplicate once C9 honours the idempotency key (a recorded contract amendment) |
| Circuit breaker | No | — | Postgres failure fails closed; a breaker adds state without changing the outcome |
| Fallback | No | — | The gate must not answer permissively when it cannot read |
| Fail-closed | Yes | Gate read failure → 503 | BR4.6 cannot be honoured on a guess |

## The drainer

- Runs as part of the Identity service's worker loop (no new container).
- Picks undelivered rows oldest-first, posts each with its idempotency key, marks `delivered_at`.
- A row that exhausts three attempts stays undelivered, is counted, and alerts at 15 minutes old.
- A manual replay drains the same rows; nothing is deleted to silence an alert (NFR1.6).

## Health checks

`/healthz` and `/readyz` are the service's; the unit adds no probe. The database check covers the
three tables, and an outbox backlog is an alert rather than a readiness failure — a delayed audit
post must not take the service out of rotation and stop the very writes that would drain it.

## Degradation matrix

| Dependency | Effect | User-visible |
|---|---|---|
| Postgres read | Gate and preferences fail | 503; the connection is refused |
| Postgres write | Grant/withdrawal fails | Error; nothing partial |
| C9 down | Outbox accumulates | Nothing user-visible; alert at 15 minutes |
| C9 rejecting (4xx) | Rows marked dead, counted, alerted | Alert; a bad payload is a defect, not a retry |
| **Drainer down or wedged** | Outbox grows; withdrawals keep succeeding | The backlog is visible without the drainer: a `consent_outbox_oldest_age_seconds` series that goes **stale** is alerted with an `absent()` rule, and the readiness-gated `consent_outbox_pending` gauge is reset to absent so the staleness is what fires |

## Recovery

- Preferences, consent events, and the outbox are ordinary rows under the platform's Postgres
  backup (RPO ≤ 24 h, RTO ≤ 4 h).
- A restored outbox replays its undelivered rows; a restored consent table plus a missing outbox
  row would be a restore-time inconsistency only if the backup were taken mid-transaction, which
  Postgres does not do.
