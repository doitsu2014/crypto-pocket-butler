# Security Design — u4-preferences-consent

**Upstream:** `security-requirements.md` (NFR9–NFR24, NFR19.1–19.4, NFR22.1–22.4, NFR23.1–23.2), `functional-spec.md`, `contract-summary.md` (C6, C9).

## Trust boundaries

| Boundary | Trusted party | Rule |
|---|---|---|
| Gateway / caller → C6 | The caller's validated JWT plus the service-token allowlist | Identity re-validates the JWT; the user id comes from the token, never a body |
| Source-connection caller (`account-api`) | An allowlisted service with the user's forwarded JWT | The gate answers for the user in the token; a service asking about another user is refused |
| Unit → Postgres | `identity_app` | Own schema; `INSERT`/`SELECT` on `consent_records`, no `UPDATE`/`DELETE` |
| Erasure job → Postgres | `identity_retention` (NOLOGIN) | The only role that may delete consent rows, and only from the erasure path |
| Drainer → C9 | Internal service over TLS | One post per due outbox row, `Idempotency-Key` header; the drainer connects as `identity_app` and may `UPDATE` `consent_audit_outbox` (`attempts`, `state`, `next_attempt_at`, `delivered_at`) and `DELETE` delivered rows — the append-only revoke covers `consent_records` only, never the outbox |

## The append-only model, and its one exception

`consent_records` is append-only **for the application**: `REVOKE UPDATE, DELETE FROM
identity_app`, plus a `BEFORE UPDATE OR DELETE` trigger that raises for that role. The exception
is erasure: `identity_retention` may delete a user's consent rows when the account is erased
(FR17.3), and the trigger permits that role. Append-only is a rule about the application's
behaviour, not a claim that data can never leave — a design that said otherwise would contradict
the privacy requirement it sits under.

## The withdrawal transaction

A row lock on the newest event is **not enough**: under READ COMMITTED the second transaction's
statement snapshot predates the first's INSERT, so it would still read `granted`. The
serialisation point is a **per-(user, kind) advisory lock**, taken before the read:

```
BEGIN
  SELECT pg_advisory_xact_lock(hashtextextended($user_id || ':' || $kind, 0));  -- serialises the pair
  SET LOCAL statement_timeout = '2s';                                          -- bounds lock wait + work
  SELECT kind, action, policy_version, occurred_at, seq FROM consent_records
    WHERE user_id = $1 AND kind = $2
    ORDER BY occurred_at DESC, seq DESC LIMIT 1;
  -- no event yet            -> ROLLBACK, 409 "never granted"
  -- newest action withdrawn -> ROLLBACK, 200 no-op returning the current state, nothing written
  -- newest action granted   -> INSERT the withdrawn event carrying that grant's policy_version
                                INSERT the consent_audit_outbox row
                                COMMIT
```

The advisory lock is released at commit/rollback. Two simultaneous withdrawals therefore produce
**exactly one** event, and a concurrency test asserts it.

## The audit payload, mapped to C9

The outbox payload is enumerated and maps field-for-field onto C9's `/internal/events` schema:

| C9 field | Value for a withdrawal |
|---|---|
| `actor` | the user id from the token |
| `action` | `identity.consent.withdrawn` |
| `target` | the kind (`consent:<kind>`) |
| `occurredAt` | the event's `occurred_at` |
| `outcome` | `success` — the withdrawal committed; a failure to record would have rolled it back |
| `context` | `{ "previousState": "granted", "policyVersion": "<the grant's version>" }` |
| `correlationId` | the request's correlation id, carried into the outbox row |

No other field exists in the payload, so a token- or wallet-shaped value has nowhere to go; the
adapter's deny-list (`eyJ…`, `0x` + 40 hex) is a backstop that **refuses the delivery and marks
the row dead** rather than sending it.

**Contract amendments this needs (recorded, not assumed):** C9 must accept an `Idempotency-Key`
header on `/internal/events` and honour it, and its rule "a security-sensitive action is complete
only once the Audit write is accepted" is amended by the outbox model to "complete once the audit
intent is durably recorded locally; delivered asynchronously with a bounded lag". Both belong to
the audit service's contract owner and are listed as blocking preconditions alongside C6.

## Authorisation

- The user id is taken from the validated token; a body-supplied id is ignored.
- Another user's row answers 404 with the same body as an absent one (BR4.3).
- The gate is read-only and answers only for the token's user.

## Data protection

- Stored: user id, four preference values, consent events (kind, action, timestamp, policy
  version), outbox payload. No name, email, subject, credential, token, or wallet address.
- Logged: user id, field name, enum values. Never a free-form value, never a reason text.
- The audit payload's field set is fixed; the adapter refuses a token-shaped (`eyJ…`) or
  wallet-shaped (`0x` + 40 hex) value rather than sending it.

## Input validation

- Locale, base currency, timezone, notification keys: closed sets, refused with 422 naming the field.
- Consent kind: the four allowed values; anything else is 422.
- Policy version: required on a grant, length-bounded; a withdrawal never supplies one.
- The history page size is capped at 100.

## Threat considerations

| Threat | Control |
|---|---|
| Cross-user read via a crafted id | The id comes from the token; a mismatched path id is 404 |
| Rewriting consent history | No `UPDATE`/`DELETE` for the application, plus a trigger |
| Accepting a withdrawal that cannot be audited | The outbox commits with the event, or neither does |
| Fabricating a policy version on withdrawal | Resolved server-side from the newest grant |
| Two racing withdrawals both inserting | `FOR UPDATE` on the user's newest row serialises them |
| A caller ignoring `missing` and connecting anyway | The unit cannot force it; the contract names the caller and the defect is theirs |
