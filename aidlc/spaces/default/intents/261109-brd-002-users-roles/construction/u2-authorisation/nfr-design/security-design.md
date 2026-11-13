# Security Design — u2-authorisation

**Upstream:** `security-requirements.md` (NFR9.1–NFR10.6, NFR19.1–NFR19.3, NFR22.1, NFR23.1–NFR23.2), `functional-spec.md` (W1–W5), `contract-summary.md` (C4, C9).

## Trust boundaries

| Boundary | Trusted party | Rule |
|---|---|---|
| Caller → decision (in-process) | The calling module, which has already validated the token | The decision re-reads the identity from the validated `CallerIdentity`; it never accepts a role or subject from a request body |
| Service → C4 endpoint | An allowlisted service holding its own token | Service token + per-operation allowlist **and** the user's JWT, validated again against JWKS (NFR9.2). **This is a contract change, recorded deliberately:** C4's declared body fields `callerSubject` and `roleClaim` are no longer the source of identity — the subject and role come from the validated JWT, and a body that carries either field is **rejected with 400**, never ignored and never trusted. The operation values are the catalogue's, the added status is 503, and the added reasons are `not_declared` and `audit_unavailable`. The C4 contract and the C4 model must be updated together before Build and Test implements the route. |
| Decision → audit service | Internal service over TLS | One write per denial, two per role change; failure fails closed |
| Decision → Supabase Auth admin API | The Identity service's own credential | One write per role change, after the intent entry |
| Decision → Postgres (status read) | The Identity service's own role | Own schema only (NFR23.1) |

## Authorisation architecture

1. **Identity** — validated token → subject; subject → internal user id; `app_metadata.role` →
   `Role`, with anything unrecognised becoming `owner` (NFR10.2).
2. **Catalogue** — the operation name maps to a class; an unknown name is `undeclared` and
   denied (NFR10.3). The class is fixed by the catalogue, never by the request.
3. **Evaluation** — status, then support, then class: administrative by role alone; collections
   by filter; single rows by owner comparison (NFR10.4).
4. **Non-disclosure** — a refused row and a nonexistent id produce the same 404 with the same
   body (NFR10.5).
5. **Separation** — support never owns, never reads portfolio contents, never reaches an
   administrative operation (NFR10.6). **The one exception is the support role's own operation:**
   `support:metadata:read` is a class of its own (`support-metadata`), decided by the u3 grant
   check rather than by an ownership comparison — otherwise u3 could never grant the metadata the
   support console exists to show. BR2.14 still denies support at every owner-scoped,
   collection, and administrative class.

## The fail-closed join

The denial is not returned until its audit entry is written. The decision takes an `AuditSink`
port; the adapter writes to the audit service; a port error becomes `audit_unavailable` and the
caller answers 503. A caller cannot return a denial without the attempt, because there is no
path from evaluation to a response that skips the port.

## Data protection

- No token, subject plaintext, credential, secret, or full wallet address is logged, audited, or
  returned (NFR22.1, NFR19.3).
- The decision holds identifiers in memory for one call and persists nothing.
- No new Postgres role or schema access; the role claim is written only through the provider's
  admin API (NFR23.1, NFR23.2).

### The audit field set, per event

There is no free-form `context`. Each event writes a fixed field set, so a value that must not
appear has nowhere to go:

| Event | Fields |
|---|---|
| `authorisation.denied` | `actor` (user id, service name, or `system`), `action` (the operation name), `target` (the resource kind and the caller's own user id — never another user's id), `occurredAt`, `outcome: denied`, `context.reason` (the `DenialReason` enum only), `correlationId` |
| `identity.role.change.requested` | `actor` (the admin's user id), `action: identity.role.change.requested`, `target` (the subject's user id), `occurredAt`, `outcome: success`, `context.previousRole`, `context.newRole`, `context.reason` (the admin's reason text, length-bounded), `correlationId` |
| `identity.role.changed` | the same fields with `outcome: success` or `failed` and `context.failureCause` (an enum, never a raw provider message) |

**Redaction at the adapter.** Before the write, the audit adapter runs a fixed deny-list over
every string field — a value matching a token-shaped pattern (`eyJ`-prefixed JWT), a
wallet-address shape (0x + 40 hex), or a key/secret key name — and refuses the write with
`audit_unavailable` rather than sending it. Redaction is a backstop for a field set that already
has no room for such a value; it is not the control, the field set is.

## Input validation

- The operation name and class come from a closed catalogue; an unknown value is denied, not
  coerced.
- `resourceOwnerId` must be a UUID when the scope is `resource`; absence is a deny.
- The reason on a role change must be present and non-blank, length-bounded at the boundary.
- A service that is not on the allowlist for an operation, or a malformed service token, is
  refused before the decision runs.

## Threat considerations

| Threat | Control |
|---|---|
| Privilege via a non-standard claim | Only `app_metadata.role` is read; everything else is `owner` |
| Caller asserts someone else's owner id | The service is allowlisted and the asserted id must match the validated JWT's user; a mismatch is a defect at the C4 boundary |
| Probing for user ids | Refusal is indistinguishable from not-found |
| Denial without a record | The audit write gates the response |
| Service-token theft | The user's JWT is required beside it; a service token alone authorises nothing user-scoped |
| Partial privileged change | Intent entry first, outcome second; a failure is recorded as `failed` |
