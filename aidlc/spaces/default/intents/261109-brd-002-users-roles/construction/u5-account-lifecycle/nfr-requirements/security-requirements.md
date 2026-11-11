# Security Requirements — u5-account-lifecycle

**Upstream:** `functional-spec.md` (W1–W5), `rules.md` (BR5.1–BR5.10), `requirements.md` (FR17, FR20, NFR22–NFR24).

## Authorisation

| ID | Requirement |
|---|---|
| NFR9.1 | Every caller's token is validated against Supabase JWKS before any status operation. |
| NFR9.2 | Admin disable/restore requires the `admin` role from the token (u2's decision), never a body field. |
| NFR9.3 | A deletion request and its restore act only on the caller's own account; another user's id answers 404. |
| NFR10.1 | The transition surface refuses an edge the machine does not permit, whatever the caller's role. |
| NFR10.2 | The read-only rule (BR5.7) is enforced by **u2's decision**, which must gain a read-versus-write predicate for `pending_deletion` and an `account:connect` operation name. This is a stated **precondition on u2**, not an assumption: until u2 ships it, a `pending_deletion` account is denied everything, which is stricter than BR5.7 and safe. |
| NFR10.3 | A status is read fresh from the `users` column on every request — no cache in this unit — so a transition takes effect at the account's next request (the freshness target for NFR24). |

## Data protection

| ID | Requirement |
|---|---|
| NFR22.1 | The subject hash does **not** live on `users`: it is `user_personal.auth_subject_hash`, beside the ciphertext `auth_subject_protected`. Migration `0006` therefore adds `users.erased_subject_hash text UNIQUE NULL`; at erasure the job copies `user_personal.auth_subject_hash` into it, then deletes the `user_personal` row (hash and ciphertext together), destroys the shredding key, and anonymises `users` — display name and email null, `email_verified` false. Surviving columns: `id`, `erased_subject_hash`, `status`, `anonymised_at`, `created_at`. A still-valid token is matched through `erased_subject_hash` and denied; `users_deleted_shape` is satisfied (`deleted` with `anonymised_at` set). |
| NFR22.2 | No transition logs the reason text, the email, the subject, or a token; the log carries the account id, the edge, and the actor id. |
| NFR22.3 | The deletion request and transition rows are retained as evidence; the personal data they are about is erased by the tombstone anonymisation above. |
| NFR23.1 | `identity_app` reads and writes only the Identity schema; erasure steps that live in another service go through that service's API. |
| NFR23.2 | The consent and preference deletes run through `identity_retention` (u4's role), never `identity_app`. |

## Integrity

| ID | Requirement |
|---|---|
| NFR19.1 | Every transition writes a `status_audit_outbox` row in the same transaction; the worker drains it to C9 with an idempotency key. |
| NFR19.2 | A transition whose outbox row cannot be written is refused — no status change is accepted without its audit intent. |
| NFR19.3 | No audit record carries a credential, token, secret, or full wallet address; the payload field set is fixed. |
| NFR24.1 | The `users.status` column and the newest `status_transitions` row are written in one transaction, so a reader can never see a status the log does not explain. |
| NFR24.2 | The expiry job and a user's restore are serialised by the lease: a restored account is never erased, and a claimed request is never double-erased. |

## Threat considerations

- **Self-restore by a disabled user:** impossible — restore is admin-only (W1) and the owner path exists only for `pending_deletion`.
- **Erasure by a compromised app role:** the anonymisation and cascade run as the app role; the consent/preference delete needs `identity_retention`, so that step is narrowly privileged.
- **Stranded deletion:** the request row and its attempt counter make a stuck erasure visible rather than silent.
- **Legal hold:** not implemented this release; the gap is recorded in `functional-spec.md` as an unmet Must with a named owner.
