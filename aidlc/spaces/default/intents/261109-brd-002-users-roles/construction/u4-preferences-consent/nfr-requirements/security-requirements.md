# Security Requirements — u4-preferences-consent

**Upstream:** `functional-spec.md` (W0–W5), `rules.md` (BR4.1–BR4.11), `requirements.md` (FR17.3, FR19, NFR22–NFR24), `contract-summary.md` (C6, C9), `services/identity/migrations/0001_init.up.sql`.

## Authentication and authorisation

| ID | Requirement |
|---|---|
| NFR9.1 | Every token is validated against Supabase JWKS before any preference or consent read or write (project rule). |
| NFR9.2 | The C6 surface accepts the user's forwarded JWT plus the service-token allowlist; the source-connection caller is `account-api`, the person's own surface is the gateway. |
| NFR10.1 | Every read and write is scoped to the caller's user id; another user's record answers exactly as a nonexistent one (BR4.3). |

## Data protection

| ID | Requirement |
|---|---|
| NFR22.1 | The unit stores a user id, four preference values, and consent events carrying a kind, an action, a timestamp, and a policy version. It stores no name, email, subject plaintext, credential, token, or wallet address. |
| NFR22.2 | A preference or consent value never appears in a log line; the log carries the user id and the field name only. |
| NFR23.1 | The unit reads and writes only the Identity schema through `identity_app`; no new role, no cross-schema access. |
| NFR23.2 | `consent_records` is append-only **for the application role**: migration 0005 explicitly `REVOKE`s `UPDATE, DELETE` from `identity_app`, installs a `BEFORE UPDATE OR DELETE` trigger that raises for that role, and introduces **one privileged retention role** — `identity_retention`, `NOLOGIN`, owned by the migrator — that alone may delete, and only through the erasure job below. Append-only is a rule about the application, not a rule that data can never be erased. |

## Erasure (reconciles append-only with FR17.3)

| ID | Requirement |
|---|---|
| NFR22.3 | When an account is erased, the erasure job (u5's privileged path, running as `identity_retention`) deletes that user's `consent_records` and `user_preferences` rows. The trigger permits `DELETE` for `identity_retention` and raises for `identity_app`. The decision is recorded here: consent history follows the account into erasure; only a legal hold (BRD-017, not yet approved) could keep it, and until that register exists the erasure runs unconditionally and its own audit entry records that it did. |
| NFR22.4 | The audit copies of past withdrawals follow the audit service's six-month retention (NFR21); they are not touched by the erasure job, which is the one place personal data outlives the account by policy and is recorded as such. |

## Integrity: the audit write on a withdrawal

The Critical case is a withdrawal whose audit entry cannot be written: the insert is append-only,
so it cannot be undone, and C9 answers 202 rather than a committed append. The design is therefore
**write-ahead inside our own transaction, then deliver**:

| ID | Requirement |
|---|---|
| NFR19.1 | A withdrawal writes **two rows in one Identity transaction**: the `consent_records` event and a `consent_audit_outbox` row carrying the audit payload and a caller-generated idempotency key. The API returns success only after that transaction commits — so an accepted withdrawal always has a durable audit intent. |
| NFR19.2 | A background drainer posts each outbox row to C9 with its idempotency key, marks it delivered, and retries transient failures with exponential backoff; a row that exhausts its retries stays in the outbox and raises an alert. The outbox is the reconciliation path, not a lossy buffer: an undelivered row is visible and replayable. |
| NFR19.3 | A failure to write the outbox row fails the withdrawal transaction — the event is not written either, so state and intent commit together or not at all. |
| NFR19.4 | No audit record carries a credential, token, secret, or full wallet address; the field set is fixed per event. |
| NFR24.1 | Consent state is read fresh on every gate call — the newest event decides — so a withdrawal takes effect at the next attempt with no cache to stale. |

**What this changes about the SLO.** "100% of withdrawals audited" now means "100% of accepted
withdrawals have a durable outbox row", which the transaction guarantees; delivery to C9 is a
separate SLI with a bounded lag and an alert, not a promise C9 can break.

## Threat considerations

- **Cross-user probe:** a foreign user id answers 404 with the same body as an absent one.
- **History tampering:** the application role has no `UPDATE`/`DELETE`; the trigger blocks it even if a grant is widened by mistake.
- **Silent withdrawal:** the outbox makes an unrecorded withdrawal impossible to accept silently.
- **Gate bypass by a caller:** the state query returns the state; a caller that ignores `missing` is that caller's defect.
- **Policy-version spoofing:** a withdrawal inherits the grant's version server-side.
