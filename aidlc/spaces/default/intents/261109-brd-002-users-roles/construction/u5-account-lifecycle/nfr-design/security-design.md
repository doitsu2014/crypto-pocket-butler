# Security Design — u5-account-lifecycle

**Upstream:** `security-requirements.md` (NFR9–NFR24), `functional-spec.md` (W1–W5), `contract-summary.md` (C3, C7, C9).

## The tombstone, made real

The subject hash is not on `users`; it is `user_personal.auth_subject_hash`. So erasure is a
**move, not a delete**, and its order is chosen so a failure is always retryable:

1. **Delete the Supabase Auth user first** (as the existing worker does). A failure here leaves the
   account fully intact and retryable; doing it last would leave an untraceable dangling identity
   once the local identity is gone.
2. In one transaction, guarded against a concurrent restore: `SELECT status FROM users WHERE id = $1 FOR UPDATE`; if the status is no longer `pending_deletion`, **abort and cancel the request** — the user restored. Otherwise:
   `UPDATE users SET erased_subject_hash = (SELECT auth_subject_hash FROM user_personal WHERE user_id = $1), status = 'deleted', anonymised_at = now(), display_name = NULL, email = NULL, email_verified = false WHERE id = $1 AND status = 'pending_deletion'` — the second guard means the row cannot be overwritten if it moved between the read and the write.
3. `DELETE FROM user_personal WHERE user_id = $1` — the hash and ciphertext leave together, and only after step 2 copied the hash.
4. Destroy the shredding key.
5. Delete consent events and preferences through `identity_retention` (u4's role).

A restore therefore wins any race: it takes the same row lock, and either it commits first (the
sweep's guarded update matches nothing and the request is cancelled) or the sweep commits first (the
account is `deleted`, and `pending_deletion→active` is no longer a permitted edge — the restore gets 409).
There is no window in which a restored account is erased.

A still-valid token is matched through `erased_subject_hash` and denied (BR5.8) — **but only if the
lookup reads it**, and today nothing does. That is the precondition this design carries:

| Change | Where | Why |
|---|---|---|
| `find_by_subject_hash` and `find_by_auth_subject` consult `users.erased_subject_hash` before concluding the subject is unknown | `services/identity/infrastructure/src/repositories/users.rs` (u1's registry) | Today both join `user_personal.auth_subject_hash`, which erasure deletes; a `None` result makes `resolve_with_origin` **provision a brand-new account** for the deleted user's token |
| A tombstone match is a distinct outcome — `SubjectErased` — that `resolve_with_origin` maps to **403 denied**, never to provisioning | `services/identity/application/src/principal.rs` | Without it, BR5.8/AC4.5.1 cannot hold however the row is shaped |
| The tombstone lookup is listed in the component inventory | `logical-components.md` → `TombstoneLookup` | So the change has a named owner rather than living in prose |

The erasure step order also matters: `erased_subject_hash` is set **before** `user_personal` is
deleted, in the same transaction, so there is no window in which the subject is in neither place.
The tombstone holds no personal data: a hash of a provider subject is a pseudonymous identifier,
kept only because deleting it would let a valid token pass unrecognised.

## Trust boundaries

| Boundary | Trusted party | Rule |
|---|---|---|
| Gateway / caller → C3 | The caller's validated JWT plus the service-token allowlist | The account is the token's own, or the caller is admin |
| Admin operations | `admin` role from the token (u2) | Never a body field |
| Sweep → other services | `identity_app` plus the retention role for consent | Each service's own erasure endpoint |
| Sweep → Supabase Auth | The Identity service's admin credential | One delete per account |

## Authorisation

- Admin disable/restore: role from the token (NFR9.2).
- Owner request/restore: own account only; another id answers 404 (NFR9.3).
- The machine refuses an illegal edge whatever the role (NFR10.1).
- **Precondition on u2 (NFR10.2):** the read-versus-write predicate and `account:connect` for
  `pending_deletion`. Until it ships, a `pending_deletion` account is denied everything — stricter
  than BR5.7 and safe.

## Data protection

- Stored: the tombstone's five columns, the transition log, the request row, the outbox payload.
- Logged: account id, edge, actor id — never the reason text, email, subject, or token (NFR22.2).
- Audit payload field set is fixed; the adapter refuses a token- or wallet-shaped value.

## Input validation

- Reason 1–500 characters on admin operations (422 otherwise).
- Status values from the closed set; the schema CHECK is the last line.
- The transition request names an edge, not a target status, so a caller cannot invent one.

## Threat considerations

| Threat | Control |
|---|---|
| A disabled user restoring themselves | Restore is admin-only; the owner path exists only for `pending_deletion` |
| An erasure that leaves the token usable | The tombstone keeps the hash and the status |
| A replayed erasure corrupting state | Every step is idempotent (BR5.6) |
| A stolen app credential mass-erasing | Erasure is leased, bounded per batch, alerting on failures; consent deletes need the retention role |
| Legal hold ignored | Recorded as an unmet Must with an owner — not pretended to be satisfied |
