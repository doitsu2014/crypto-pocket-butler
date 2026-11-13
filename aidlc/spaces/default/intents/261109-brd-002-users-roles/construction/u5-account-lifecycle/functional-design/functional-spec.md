# Functional Specification — u5-account-lifecycle

Implements **US4.1–US4.5** and the status-change half of **US7.2** from `stories.md`. U5 owns the
account status machine, the deletion request, and the expiry-and-erasure job.

## What already exists (checked against the schema and code)

| Fact | Source | Consequence |
|---|---|---|
| `users.status` CHECK allows `pending, active, disabled, pending_deletion, deleted` | `migrations/0004_user_registry_extensions.up.sql` (u1 widened it) | The five-value machine is storable without a new status migration |
| `users_deleted_shape` requires `anonymised_at` when status is `deleted`, null otherwise | `0001_init.up.sql` | Erasure **anonymises in place**; it does not delete the row |
| `deletion_requests` already exists with `status requested/in_progress/completed/failed`, `lease_owner`, `lease_until`, `progress`, `completed_at` | `0001_init.up.sql` | Migration `0006` **alters** it (below) rather than creating a new table |
| A sweep loop exists in `identity-worker` with `IDENTITY_SWEEP_INTERVAL_SECONDS` (default 60) and a lease-based claim | `services/identity/src/config.rs`, `application/src/deletion.rs` | The expiry job reuses that loop, interval and lease |
| The deletion worker already deletes the Supabase Auth user and the `user_personal` row | `application/src/deletion.rs` | The cascade keeps those steps and adds consent/preferences |
| `contract-summary.md` C7 names the owner path `DELETE /deletion-requests` and the deadline `recoveryEndsAt` | C7 | This spec uses those names; C7 gains the 409/410 responses it lacks |

**Migration `0006_account_lifecycle`** (u5's schema change, a versioned pair):
- `users.status`: no change needed (u1's `0004` already widened it).
- `deletion_requests`: add `recoverable_until timestamptz not null`, `cancelled_at timestamptz`,
  `erased_at timestamptz`, `attempt_count int not null default 0`, `next_attempt_at timestamptz not
  null default now()`; map `status` to the design's two open/terminal meanings (`requested` and
  `in_progress` are open; `completed` sets `erased_at`; `failed` is an open row with an attempt
  count); add `status_transitions` (append-only) and its indexes.
- The down reverses those columns and drops `status_transitions`; it refuses to run while any open
  request exists, so a rollback cannot silently strand a deletion.

## The five statuses, and who owns each edge

`pending` stays a stored status — the column's CHECK has carried it since `0001`, and
`contract-summary.md`'s open question is answered here: **u1 provisions `pending`; u5 owns every
transition out of it.** Until the confirmation flow (U8) exists, the registry's verified-email
promotion performs `pending→active` through this unit's guarded transition path, audited like any
other edge; when U8 ships it calls the same path.

```
pending --provider verified (u1, via u5's transition path)--> active
active --admin disable (reason)--> disabled
disabled --admin restore (reason)--> active
active --user request--> pending_deletion
pending_deletion --user restore within window--> active
pending_deletion --window ends (job)--> deleted   [erasure completes]
```

No other edge is permitted (BR5.1). `deleted` is terminal.

## Workflows

### W1 — Admin disables or restores

1. An admin calls disable (or restore) with a reason.
2. The caller must be `admin` (BR5.2); the reason 1–500 characters.
3. The edge must be permitted (BR5.1); anything else is **409**.
4. In one transaction: write `users.status`, insert the `StatusTransition`, and insert a
   `status_audit_outbox` row carrying the audit payload and an idempotency key (BR5.9).

**Error paths:** non-admin → 403; no reason → 422; illegal edge → 409, nothing written.

### W2 — A user requests deletion

1. The user calls delete on their own account.
2. Any open request ⇒ **409**.
3. In one transaction: insert/update the `deletion_requests` row with
   `recoverable_until` = midnight UTC on the 30th calendar day, write
   `users.status = pending_deletion`, insert the transition and its outbox row.
4. The response carries `recoveryEndsAt` (C7's name).

**Error paths:** second request → 409; a status that cannot reach `pending_deletion` → 409 naming it.

### W3 — The user restores

1. The user calls `DELETE /internal/users/{id}/deletion-requests` (C7's owner path — restore is
   the owner cancelling their request; the admin restore in W1 is a different operation).
2. If `now < recoverable_until`: set `cancelled_at`, write `users.status = active`, insert the
   transition and outbox row.
3. Else: **410** — the window has closed and the account is due for erasure.

**Error paths:** restoring an account that is not `pending_deletion` → 409.

### W4 — The expiry job

Runs in the existing `identity-worker` sweep loop (`IDENTITY_SWEEP_INTERVAL_SECONDS`), claiming a
batch with the existing lease (`lease_owner`, `lease_until`) rather than a held row lock, and for
each due request:

1. Re-read the status; if it is no longer `pending_deletion` (the user restored concurrently),
   **close the request** (`cancelled_at` set) so it is never re-claimed, and skip.
2. Transition `pending_deletion→deleted`, insert the transition and outbox row.
3. **Anonymise the `users` row in place; never delete it.** The row is the denial tombstone the
   schema already enforces:
   - survives: `id` (ownership references elsewhere resolve to it), `provider_subject_hash` (the
     only way a valid token is matched and denied — AC4.5.1), `status`, `anonymised_at`, `created_at`;
   - cleared: `display_name` and `email` null, `email_verified` false, `provider_subject_cipher`
     destroyed, the shredding key destroyed, the `user_personal` row deleted.
4. Delete the account's data in each service that owns it, through that service's own erasure path:
   the **Supabase Auth user**, consent events and preferences (u4's `identity_retention` role), and
   any other service with an erasure endpoint. A service with none is recorded on the run as *not
   yet erasable*, never claimed as erased.
5. Set `erased_at`, reset `attempt_count`, and write the run's outbox row naming the account and what
   was erased.

**Retry and bound.** A failed step leaves the request open with `attempt_count + 1` and a backoff in
`next_attempt_at` (1, 2, 4, 8, 16 minutes, capped at 1 hour). After **five** attempts the row is left
open and counted by `deletion_erasure_failures_total`, which alerts at any non-zero value over 15
minutes. A second run over an already-erased request is a no-op that reports success (BR5.6).

### W5 — Read-only during the window

1. The account's status reaches u2's decision as it does today.
2. For `pending_deletion`, **reads and the cancellation in W3 are allowed; writes are refused with
   403** and a body naming the restore path (BR5.7). A new source connection is a **write-class
   operation** added to u2's catalogue by name (`account:connect`), so the decision can classify it.
3. **This needs a change in u2, recorded here:** its denial predicate changes from "deny
   everything" to "deny every write-class operation and allow read-class ones" for
   `pending_deletion`. Without that change the read-only rule has no enforcement owner.

## Error catalogue

| Condition | Status | Behaviour |
|---|---|---|
| Non-admin disable/restore | 403 | Refused and audited |
| Missing reason | 422 | Field named, nothing written |
| Illegal transition edge | 409 | Refused; the attempt audited |
| Second deletion request | 409 | Refused; the open request returned |
| Restore after the deadline | 410 | Refused; the account is due for erasure |
| Write during pending_deletion | 403 | Refused; the restore path named |
| Erasure step fails | — | Request stays open with backoff; alerts after five attempts |

## Audit mechanism (the thing BR5.9 previously hand-waved)

Audit is a separate service over `POST /internal/events` (C9, 202 Accepted), and message queues are
forbidden, so no transaction can span the two. The mechanism is a **`status_audit_outbox` row
written in the same transaction as the status change**, drained by the same worker loop to C9 with an
`Idempotency-Key`, exactly as u4 does for consent. What that guarantees: **100% of accepted
transitions have a durable audit intent**; delivery is a separate SLI with a bounded lag and an
alert, and is not a promise the audit service can break. AC7.2.1/AC7.2.2 are covered to that degree,
not to "the audit row exists by the time the response returns".

## The legal-hold gap, recorded as an unmet Must

FR17.4 says erasure "shall not touch records under legal hold or statutory retention", and AC4.4.2
tests it. BRD-017 owns the register; neither exists. This release therefore **erases unconditionally**,
chosen at this stage's question, and that choice **fails FR17.4/AC4.4.2 as written**. It is recorded
here as an accepted gap with two separate owners:

- **Legal holds** — BRD-017's register; when it lands, the expiry job gains a hold check and records
  the exception on the run.
- **Statutory retention** — a compliance input, independent of BRD-017; today no record is exempt,
  which is a compliance risk the owner (compliance) carries until the retention schedule exists.

AC4.4.2 stays `N/A` in `traceability.json` with this reason, not `OK`.

## Integration points

- **u1-user-registry:** owns the `users` row and the status value; u5 writes it only through the
  transition path. u1's `0004` already allows every value.
- **u2-authorisation:** evaluates status denial from that column — **and must gain the
  read-versus-write predicate and the `account:connect` operation name** (W5).
- **u4-preferences-consent:** erasure deletes consent events and preferences through
  `identity_retention`.
- **Supabase Auth:** the erasure deletes the Auth user.
- **C3 / C7:** the transition surface and u5's operations; C7 gains the 409/410 responses and uses
  `recoveryEndsAt`.
- **C9:** every transition, through the outbox.

## What this unit does not own

The status value set and its denial semantics are u1's and u2's; u5 owns every transition. The
per-entity cascade for other services is BRD-017's: each service's erasure endpoint is called where
one exists, and a missing one is recorded, never assumed.
