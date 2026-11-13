# Security Design — u3-support-access

**Upstream:** `security-requirements.md` (NFR9.1–NFR9.2, NFR10.1–NFR10.7, NFR11.1, NFR19.1–NFR19.4,
NFR22.1–NFR22.2, NFR23.1–NFR23.3), `functional-spec.md` (W1–W4, the metadata boundary),
`contract-summary.md` (C1/C2, C5, C9, C10), `components.md` (SupportAccess).

## Trust boundaries

| Boundary | Trusted party | Rule |
|---|---|---|
| User → gateway → Identity | The user, holding a Supabase session | The gateway forwards the JWT unchanged; Identity validates it against JWKS (signature, issuer, audience, expiry) before any grant is read (NFR10.1). Identity never trusts a role or user id from a header or body |
| Identity → Account / Portfolio (support metadata) | Identity, holding the `identity` service token | Route-scoped allowlist: the token opens exactly the two support-metadata routes on those services and nothing else (NFR9.1). No user JWT is sent — no user is present on that hop, and none is needed for an indexed lookup by id |
| Identity → audit service | Internal service over TLS | Identifiers and enum codes only; the audit store is insert-only (NFR23.2) |
| Identity → its own Postgres | The `identity_app` role | Own schema only; `support_audit_outbox` granted to `identity_app` and `identity_migrator`, to no one else (NFR23.1) |

## Authentication and authorisation flow

1. **Authenticate** — the existing middleware validates the JWT and produces `CallerIdentity`
   (subject → internal user id, role from `app_metadata.role`). A validation failure is 401 before
   any support rule runs.
2. **Classify** — `support` may request a grant and read metadata under one; `admin` may approve
   and revoke. No other role reaches a support route; a wrong role is 403 `role-required`.
3. **Evaluate the grant** — holder check (`requestedBy` = caller, NFR10.3), status check at the
   database clock (`active` and `expires_at > now()`), target not `pending_deletion`. The role is
   never inherited from the grant and the grant never satisfies an ownership predicate (NFR10.4).
4. **Non-disclosure** — an unknown id and a foreign grant produce one uniform
   `403 support-access-refused` (NFR10.6); the precise reason exists only in the audit context.
   Approve/revoke keep 404 for an unknown id because the admin check runs first.
5. **Closed projection** — the request to each owner names only its allowlisted fields
   (NFR10.5); the response DTO has no field for an email address, provider subject, venue,
   provider account id, error detail, or any content field. Nothing is filtered after the fact,
   because there is nothing to filter.

**The one directed edge.** Identity → Account/Portfolio for metadata is one-directional. The two
new endpoints must not call back into C4 `/internal/authorize` (NFR9.2): the decision already ran
in Identity, and a callback would create a request cycle and a shared failure domain between the
two directions.

## Rate limiting

The gateway is the only limiter, and the shipped one is **per observed client IP**, not per user:
`middleware::rate_limit` keys on the address it observed (never a caller-supplied header) with a
120/min authenticated budget and a stricter anonymous one, and `Route` has no per-route field
today. Two of the four support routes are manual triggers, so this unit makes the limit real
rather than leaving it to the global budget (NFR11.1).

**The new field.** `Route` gains `max_requests_per_minute: Option<u32>`; when it is `None` the
route behaves exactly as it does today. `RateLimiter::check` gains a per-route check that runs
**in addition to** the existing IP budget, never instead of it:

| Route | Per-route budget |
|---|---|
| `POST /v1/support-grants` (creation) | 20/min |
| `GET /v1/support-grants/{id}/metadata` (read) | 60/min |

**The key.** The per-route bucket is keyed on the request's bearer token, hashed (SHA-256) — not
on the JWT's `sub` claim. The gateway forwards the token unchanged and does not validate it
(`memory/project.md`), so no claim it reads is trustworthy; a token hash is a value the caller
cannot forge into another caller's bucket, needs no signature check, and is bounded in memory. A
Supabase access token rotates roughly hourly, so the bucket resets then — acceptable for a
fair-use ceiling on a staff trigger.

**Why the IP budget stays.** The token hash is a fairness control, not the enforcement boundary.
Anyone can mint random tokens to obtain fresh buckets, so the 120/min per-IP authenticated budget
(and the anonymous limit) continues to apply to every request, support routes included. The
composite is therefore: the existing IP budget **and**, when the route declares one, the token
budget. Either can answer 429 with the standard RFC 7807 body and a `Retry-After` header.

**Acceptance.** Two gateway HTTP boundary tests: the 21st creation in a minute for one token
answers 429 while a second token still passes, and the per-IP budget still answers 429 for a
flood of distinct tokens from one address.

## Audit design

### Three mechanisms, one rule

| Event | Mechanism | Failure behaviour |
|---|---|---|
| Accepted request / approve / revoke | `support_audit_outbox` row inserted **in the same transaction** as the grant write | The transaction rolls back; the mutation is refused |
| Refused request / approve / revoke / read | Its own outbox row, **committed before** the refusal is returned | The refusal is replaced by `503 audit-unavailable` |
| Served metadata read | Synchronous `HttpAuditClient` append (200 ms, one attempt) **before** the projection is returned | The read answers `503 audit-unavailable` |

The rule behind all three: no caller learns the outcome of a support operation before its audit
intent exists. For mutations "exists" means committed with the mutation; for a refusal and a served
read it means written before the response.

### Field set per event

There is no free-form `context`; every event writes a fixed shape, so a value that must not appear
has nowhere to go (NFR22.1, NFR19.3):

| Action | Fields |
|---|---|
| `identity.support_grant.requested` | `actor` (agent user id), `action`, `target` (target user id, `grantId`), `occurredAt`, `outcome` (`success`/`denied`/`failed`), `context.reasonCode` (enum only), `correlationId` |
| `identity.support_grant.approved` / `.revoked` | `actor` (admin user id), `action`, `target` (`grantId`, target user id), `occurredAt`, `outcome`, `context.reasonCode`, `correlationId` |
| `identity.support_read.served` | `actor`, `action`, `target` (`grantId`, target user id), `occurredAt`, `outcome: success`, `context.sources` (the two source names that answered), `correlationId` |
| `identity.support_read.denied` | `actor`, `action`, `target` (`grantId` when known, else the requested id), `occurredAt`, `outcome: denied` or `failed`, `context.refusalCode` (enum), `correlationId` |

The grant's `reason` text is **never** in an audit record — it stays on the grant row for a human
reviewer. No token, credential, email address, subject, or wallet address is in any field, and the
metric labels carry only bounded enums (NFR22.2).

### Delivery

At-least-once with the outbox row id as the idempotency key; a dead row is replayed by the
operator statement in NFR1.5. The outbox table is the only support data that leaves the identity
schema, and it leaves as C10 events, never as a database grant.

## Secrets

- The `identity` service token is provisioned through the existing registry/Vault path, held in
  memory, attached to the two hops, and never logged or forwarded onward (NFR23.3).
- No new secret is introduced; no credential appears in a migration, a config file, a log line,
  or an error body.

## Input validation

- Path ids are UUIDs; a non-UUID is 400 at the boundary, never a query.
- The grant `reason` is required, non-blank, 1–500 characters (the schema's own check), and is
  never echoed back in an error or an audit record.
- The metadata read accepts only the grant id in the path; there is no caller-supplied user id, so
  a caller cannot ask for a different target through a parameter.
- Admin actions validate the id and the role before any write; an unknown id is 404, a wrong status
  is 409, and neither reveals the grant's contents.

## Threat considerations

| Threat | Control |
|---|---|
| Grant id enumeration | Uniform 403 for unknown and foreign ids; precise codes only in the audit context (NFR10.6) |
| Privilege escalation through a grant | The grant is consulted only by the metadata read; every ownership predicate ignores it (NFR10.4) |
| Audit suppression | A response is not produced until its audit intent is written (above) |
| Service-token replay | Route-scoped allowlist, no user identity in the token, and a read-only request shape |
| Fan-out leakage | The request shape is the control: each owner is asked only for its allowlisted fields |
| Stale authorisation | Role and grant state are read per request; nothing is cached (NFR24.1) |
| Credential/content access through support | No support route names a credential, holding, balance, valuation, transaction, or order (NFR10.7) |
| Caller-supplied identity | No support route reads a user id, role, or owner id from a body or query parameter |