# Security Requirements — u3-support-access

**Upstream:** `functional-spec.md` (W1–W4, metadata boundary), `rules.md` (BR3.1–BR3.13),
`requirements.md` (FR16, FR20.3/20.4, NFR22–NFR24), `contract-summary.md` (C1/C2, C5, C9, C10),
`components.md` (SupportAccess).

U3 is the one place in the product where a person who is not the user may read something about
that user. Every requirement below narrows that exception: who may ask, what may come back, what
must be recorded, and what must fail when a control cannot be applied.

## Authentication

Sub-number parents follow the BRD-001 requirements record this intent continues: NFR9 is
transport and secret protection, NFR10 is token and session security, NFR11 is abuse protection.

| ID | Requirement |
|---|---|
| NFR9.1 | The two new Account and Portfolio support-metadata endpoints sit on the existing `require_service` router and accept **only** the per-service token, with a **route-scoped** allowlist of `identity`. There is no user JWT on that hop because no user is present on it: the service token is the whole trust boundary, so it is scoped to one route rather than to "any registered service". |
| NFR9.2 | Those two endpoints must **not** call back into Identity's C4 `/internal/authorize`. The support-metadata edge is one-directional (Identity → Account/Portfolio), so no request cycle and no shared failure domain exists between the two directions. |

## Authorisation model

| ID | Requirement |
|---|---|
| NFR10.1 | Every support request arrives with the user's Supabase JWT, forwarded unchanged from the gateway, and Identity validates it against JWKS — signature, issuer, audience, expiry — before any grant is read. A failure is a 401 from the service's own middleware, never a 403 from a rule. |
| NFR10.2 | `support` for a request and `admin` for an approve or revoke are read only from `app_metadata.role` in the validated claims; no other claim, table, or header grants either, and U3 never infers a role from another field (project constraint; U2's decision). |
| NFR10.3 | A grant is usable only by the agent named as its `requestedBy` (BR3.8). An admin is an approver and a revoker, never a reader under a grant. |
| NFR10.4 | A grant is never an ownership credential (BR3.11): the Account, Portfolio and Identity ownership predicates never consult a grant, and portfolio contents, holdings, balances, valuations, transactions and orders stay refused under a valid grant. |
| NFR10.5 | The metadata projection is a closed allowlist (BR3.9, BR3.10): user id, display name, email-verification flag, account id/label/source type/sync status/last-synced-at, portfolio id/label. The email address, provider subject, venue, provider account id, error detail and every content field are absent by construction — never requested from their owners, not filtered after the fact. |
| NFR10.6 | Grant existence is not disclosed to a caller who does not hold it: an unknown id and a foreign grant answer one uniform `403 support-access-refused`; the grant's holder gets the precise code (BR3.7, BR3.8). The precise code is still recorded in the audit context. |
| NFR10.7 | The support route set itself is the boundary against credential and content access: no support route names a credential, holding, balance, valuation, transaction or order, and a support caller on any such route is refused (BR3.10). |

## Abuse protection

| ID | Requirement |
|---|---|
| NFR11.1 | The four `/v1/support-grants` routes are covered by the gateway's existing per-user limit (120 requests/minute default). The two manual triggers — grant creation and the metadata read — get a **stricter per-route limit**, and **this unit owns delivering it** because this unit already adds the four gateway route declarations: the gateway `Route` type gains an optional `max_requests_per_minute`, the limiter applies the route's value when present and the global default otherwise, and the two routes declare 20/min (creation) and 60/min (read). Acceptance: a 21st creation in one minute answers 429 from the gateway with the RFC 7807 envelope, and every other route keeps the global limit. Verified by the gateway's own HTTP boundary test in this unit's Build and Test. |

## Data protection and least privilege

| ID | Requirement |
|---|---|
| NFR22.1 | A grant row, an audit event, a log line and a metric label carry identifiers only. The `reason` text is stored on the grant because a human reviewer needs it, and it is **never** copied into an audit `context`, a log line, a metric label, or an error detail. |
| NFR22.2 | No support path logs or returns a token, a credential, a secret, the email address, the provider subject, or a full wallet address; the metadata projection is the only user-shaped payload the unit returns, and it is the closed allowlist of NFR10.5. |
| NFR23.1 | Every read and write goes through the `identity_app` role on the `identity` schema. The new `support_audit_outbox` is granted to `identity_app` (select/insert/update/delete, needed by the drainer's claim and mark) and to `identity_migrator`; no cross-schema grant is added, and no other service reads the table. |
| NFR23.2 | The audit service stays append-only through its insert-only role. U3 never writes to the audit schema; it posts C9 events and stores only its own outbox rows. |
| NFR23.3 | The `identity` service token used by the two new internal endpoints is provisioned in the service-token registry (Vault-backed) and appears in the Account and Portfolio route allowlists. It is never a user token, never logged, and never forwarded onward. |

## Audit and fail-closed behaviour

| ID | Requirement |
|---|---|
| NFR19.1 | Every request, approval, revocation, served read and refusal is audited with actor, action, target, timestamp, identifier-only context and an outcome from C9's `success`/`denied`/`failed` vocabulary (BR3.13). The five actions are `identity.support_grant.requested`, `.approved`, `.revoked`, `identity.support_read.served`, `identity.support_read.denied`; `failed` is used when the operation could not complete, including a metadata source that could not answer. |
| NFR19.2 | An accepted mutation cannot exist without its audit intent: the grant row and its `support_audit_outbox` row commit in one transaction, and the mutation is refused if that row cannot be written. A refusal has no mutation to share, so its outbox row is written in its own transaction **before** the refusal is returned, and a failed write answers `503 audit-unavailable` instead of the clean refusal. A served read appends synchronously and fails closed with 503 if the append fails. |
| NFR19.3 | No audit record carries a credential, token, secret, full wallet address, or the grant's reason text. The C10 client's payload carries identifiers and the refusal code only (BR3.13). |
| NFR19.4 | Audit delivery is at-least-once with a stable idempotency key per outbox row (`support_audit_outbox.id`), so a retried or replayed delivery appends once. A row that exhausts its attempt budget becomes `dead` with its last error recorded, and a manual replay resets its retry state rather than creating a second row. |

## Threat considerations

- **Grant id enumeration:** an unknown id and a foreign grant answer identically (NFR10.6), so a
  support agent cannot probe which grants exist. Approve and revoke keep 404 on an unknown id
  because the admin check runs first — only an admin, who is not the enumeration case, can reach
  that answer.
- **Privilege escalation through a grant:** a grant authorises one read of one user's metadata and
  nothing else; it is not consulted by any ownership predicate (NFR10.4), so support cannot become
  an owner by holding one.
- **Audit suppression:** a caller cannot obtain a refusal without the audit attempt, because the
  refusal is not returned until its outbox row is written (NFR19.2).
- **Service-token replay against the new endpoints:** the token is scoped to one route and one
  service name; it cannot be used for any other internal route, and it carries no user identity, so
  a replayed call can only read the metadata shape NFR10.5 already limits.
- **Fan-out leakage:** Identity asks each owner only for its allowlisted fields, so a mistake in
  Identity cannot widen what Account or Portfolio returns — the request shape, not a response
  filter, is the control (BR3.10).
- **Stale authorisation:** a role change or a revocation takes effect at the next request because
  neither the role nor the grant state is cached (NFR24.1, BR3.7).

## Unit-level applicability of the inception NFRs

| Inception NFR | Applies | How |
|---|---|---|
| NFR21 audit retention | Indirectly | The audit service enforces six months; U3 keeps its own grant rows as evidence and its outbox rows until delivered or dead |
| NFR22 sensitive identifiers | Yes | NFR22.1, NFR22.2 |
| NFR23 least privilege | Yes | NFR23.1–NFR23.3 |
| NFR24 role and status freshness | Yes | NFR24.1 (below) |
| NFR25 accessibility | No | The support console and admin screens are U9 and U10; this unit has no user-facing surface |
| NFR26 localization | No | This unit returns machine-readable fields and ISO-8601 UTC timestamps; the consoles format them |

Inherited from the BRD-001 record (not part of this intent's own NFR21–NFR26 table): **NFR11
abuse protection** applies and is NFR11.1; NFR9 (transport and secret protection) is NFR9.1–NFR9.2;
NFR10 (token and session security) is NFR10.1–NFR10.7.

**Grant-table retention.** `support_access_grants` is product data, not an audit record: NFR21's
six-month rule governs the audit store that C10 writes. Grant rows are kept as the authorisation
history (about 365,000 rows a year at the projected volume) and are removed with the target user's
account — the `user_id` foreign key means an erasure cannot complete without deleting them (NFR20.6).
That is the whole retention position; there is no archival tier and no sweep.

## Freshness

| ID | Requirement |
|---|---|
| NFR24.1 | The role is recomputed from the signed token on every request and the grant's status is read from the row on every request; nothing about a grant is cached in process. A revocation or an expiry takes effect at the caller's next request, without a cache refresh or a sweeper. |