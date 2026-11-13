# Tech Stack Decisions — u5-account-lifecycle

**Upstream:** `codekb/crypto-pocket-butler/technology-stack.md`, `functional-spec.md`, `project.md`, the existing migrations and worker.

The stack is fixed by project rule. This file records what this unit adds and what already exists.

## What already exists (checked)

| Fact | Source | Consequence |
|---|---|---|
| `users.status` CHECK already allows all five values | `migrations/0004_user_registry_extensions.up.sql` | No status migration needed |
| `users_deleted_shape` requires `anonymised_at` with `deleted` | `0001_init.up.sql` | Erasure anonymises in place; the row survives |
| `deletion_requests` exists with `status`, `lease_owner`, `lease_until`, `progress`, `completed_at` | `0001_init.up.sql` | `0006` alters it; it does not create a parallel table |
| A sweep loop with `IDENTITY_SWEEP_INTERVAL_SECONDS` (default 60) exists in `identity-worker` | `services/identity/src/config.rs` | The expiry job reuses it; no new scheduler |

## Decisions

| Concern | Choice | Rationale | Alternative rejected |
|---|---|---|---|
| Erasure end state | Anonymise the `users` row in place; keep it as the denial tombstone | The schema's `users_deleted_shape` already assumes it, and a valid token must still be denied | Deleting the row — rejected: nothing left to match the subject hash and deny |
| Migration `0006_account_lifecycle` | ALTER `deletion_requests` (add `recoverable_until`, `cancelled_at`, `erased_at`, `attempt_count`, `next_attempt_at`), map its status values, create `status_transitions` and `status_audit_outbox` | The table exists; a parallel one would split the evidence | A new table — rejected |
| Migration safety | Columns added nullable with a backfill, as u4's `0005` does; indexes built `CONCURRENTLY` through the statement-wise runner | Same live-table constraint | A single transactional script — rejected: it would hold the lock |
| Claiming due requests | The existing lease (`lease_owner`, `lease_until`) | Already in the schema and the worker; no held row lock across the cascade | `FOR UPDATE SKIP LOCKED` held across HTTP calls — rejected: a lease cannot be held for an hour |
| Audit delivery | `status_audit_outbox` in the same transaction, drained with an idempotency key | The audit service is separate and queues are forbidden | Calling C9 inline — rejected: a 202 is not a durable record |
| Retry bound | `attempt_count` + `next_attempt_at`, backoff 1/2/4/8/16 min, alert after 5 | The bound has to be stored or it is not a bound | Unlimited retry — rejected: silent loop |
| Read-only enforcement | u2's decision gains a read-versus-write predicate and an `account:connect` operation name | The rule needs an owner that can tell a read from a write | Enforcing in every caller — rejected: it would be re-implemented per service and drift |

## Dependencies added by this unit

None. `sqlx`, `chrono`, `serde`, `thiserror`, `tracing`, `reqwest` are already declared.

## Constraint check

- Dependencies point inward; ports inward, adapters outward; DTOs at the `http` boundary.
- One schema, one role (plus the erasure role u4 created and this unit borrows); no cross-schema access; no RLS; no queue.
- The migration ships as a versioned up/down pair; the down refuses while an open request exists.
