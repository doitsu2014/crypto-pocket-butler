# Reliability Design — u3-support-access

**Upstream:** `reliability-requirements.md` (NFR1.1–NFR1.11, NFR18.1–NFR18.4),
`functional-spec.md` (W1–W5), `contract-summary.md` (C10), `observability-requirements.md`.

## Timeout and retry policy

Every outbound call is bounded and none of them retries inside a request:

| Call | Connect | Request | Attempts | On failure |
|---|---|---|---|---|
| Account metadata read | 1 s | 120 ms | 1 | 503 `metadata-source-unavailable` |
| Portfolio metadata read | 1 s | 120 ms | 1 | 503 `metadata-source-unavailable` |
| Audit append (served read) | 1 s | 200 ms | 1 | 503 `audit-unavailable` |
| Support outbox delivery | 1 s | 2 s | 1 per row, per pass | `next_attempt_at` backoff; `dead` after 3 |
| Grant / target-user statements | — | 500 ms (`SET LOCAL`) | 1 | 503 |
| Mutation statements | — | 2 s (`SET LOCAL`) | 1 | 503 |

**No write is retried automatically.** The grant endpoints are not idempotent and Identity does
not derive an operation identity from the gateway's idempotency header, so a client retry is a
second request. BR3.3 is the safety net: while a live grant exists, the duplicate answers 409
rather than creating a second grant.

**No read is retried inside the deadline.** A retry of a 120 ms source call would consume most of
the remaining 500 ms and still miss the p99; a slow source is refused, not waited for.

## Failure containment

- **Per-path fail-closed.** A mutation needs its own transaction and the audit service's outbox
  write; it cannot commit without both. A refusal needs its outbox row committed; a served read
  needs its append. Each failure stops at the request that caused it.
- **No cascading fan-out.** The metadata read calls two owners and stops. The two new endpoints
  are forbidden to call back into Identity (NFR9.2), so there is no cycle to loop in and no shared
  failure domain between the two directions.
- **No cross-request state.** Nothing is cached, nothing is held in a task after the request
  returns, and a cancelled request (deadline) cancels its in-flight source calls.
- **No shared pool with the drainer.** The drainer uses the same pool but its own client; a slow
  audit service makes outbox delivery lag, never a user request (mutations do not wait for
  delivery).

## The outbox state machine

```
insert ──▶ pending ──deliver ok──▶ delivered ──24h──▶ deleted
              │
              ├─ transient failure ──▶ pending (next_attempt_at = backoff)
              ├─ 4xx ────────────────▶ dead (last_error recorded)
              └─ 3rd failed attempt ─▶ dead ──operator replay──▶ pending (attempts reset)
```

- Claim: `SELECT … WHERE state = 'pending' AND next_attempt_at <= now() ORDER BY created_at
  FOR UPDATE SKIP LOCKED LIMIT 5`, with the shipped 30 s lease.
- Idempotency: key = outbox row id, stable across attempts and replay, so C10 appends once.
- Retention: delivered rows deleted 24 h after delivery by the drainer's cleanup.
- Replay: the documented `UPDATE` statement (NFR1.5), run by an operator; it resets `attempts`,
  `next_attempt_at`, and `last_error` on the same row.

## Health and readiness

- `/healthz` — process up only; this is the Swarm rolling-update gate. It must not depend on any
  downstream, or one outage takes every replica out of the update.
- `/readyz` — database and audit store reachable. Account and Portfolio are **reported but do not
  fail readiness**: their outage refuses the metadata read (the designed behaviour) and must not
  remove Identity's replicas for every other route.
- The drainer's heartbeat is a metric (`support_outbox_last_pass_timestamp_seconds`), not a
  readiness signal: a lagging drainer does not make Identity unready, because mutations stay
  correct while delivery lags.

## Graceful degradation

There is **no degraded mode** for the metadata read: it serves the full projection or refuses with
a named 503. A partial projection would look authoritative while silently missing an owner, which
is worse than an explicit refusal for an investigator. The only designed degradation is temporal:
an accepted mutation's audit delivery may lag, bounded by the drainer's pass interval and surfaced
by the backlog gauges.

## Recovery and rollback

| Scenario | Design |
|---|---|
| Migration rollback | `0007_support_access.down.sql` re-adds the pre-0007 constraint as `NOT VALID`, drops the outbox and the partial index; historical rows stay exactly as recorded, so the down step never blocks on the constraint. The outbox is exported first — see below |
| Release rollback | The Swarm stack's existing rolling policy with `failure_action: rollback` on the `/healthz` gate (NFR18.3) |
| Replica loss | Nothing to recover: no in-process state, no cache, no leader. A restart resumes draining from the table |
| Database restore | Grants and outbox rows are in the Identity schema and inherit its backup (RPO ≤ 24 h, RTO ≤ 4 h, NFR18.1); no unit-owned store exists |

## The outbox cannot be dropped with an intent still in it

A rollback removes the code that would ever deliver a support intent, so dropping
`support_audit_outbox` as the migration's first act would destroy evidence the design promises
exists. The rollback procedure therefore runs in this order, and the down migration enforces the
order instead of trusting it:

1. **Export before rollback.** The release rollback procedure runs the documented export statement
   — the same shape as the NFR1.5 replay `SELECT`, plus a post to the audit service — over every
   row whose `state` is `pending` or `dead`, and records the result. A row that cannot be delivered
   is written to the operator's audit export artefact, not left in a table that is about to vanish.
2. **The down migration refuses to drop a non-empty outbox.** Its first statement raises if any
   row remains in `pending` or `dead`; only an outbox with nothing undelivered is dropped. A
   rollback therefore fails loudly at the migration step rather than silently losing an intent,
   which is the same failure the at-least-once rule exists to prevent.
3. **The constraint restore still never refuses.** Re-adding `support_grants_approval_shape` as
   `NOT VALID` keeps every historical grant row as recorded; the only condition the down step can
   reject is undelivered audit evidence, and that rejection is itself the designed behaviour.

This refines NFR18.2's "never refuses" from the constraint to the outbox: the rollback still never
blocks on grant history, and it blocks only on data that would otherwise be lost.

## What this unit does not add

No circuit breaker library, no retry queue, no second drainer, no health-check endpoint, no
failover mechanism. Each of those would be a new failure mode for a staff workflow whose worst
case is a refused read; the existing primitives (timeouts, the outbox, the Swarm rollback) cover
it.

No circuit breaker library, no retry queue, no second drainer, no health-check endpoint, no
failover mechanism. Each of those would be a new failure mode for a staff workflow whose worst
case is a refused read; the existing primitives (timeouts, the outbox, the Swarm rollback) cover
it.