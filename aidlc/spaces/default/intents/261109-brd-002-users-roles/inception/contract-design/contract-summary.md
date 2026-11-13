# Contract Summary — Users, Roles & Permissions

Every boundary gets a REST/JSON contract with RFC 7807 problem bodies. The gateway's `/v1` surface is the only external boundary; the rest are internal service-to-service edges from the unit DAG.

**Upstream:** `unit-of-work.md`, `unit-of-work-dependency.md`, `components.md`, `requirements.md` (FR14–FR20, NFR21–NFR26).

## Contracts

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---|---|---|---|
| C1 | Gateway `/v1` | External: SPA (public web) | REST/JSON | Gateway service; routes declared by each provider unit |
| C2 | Gateway `/v1` | External: support console | REST/JSON | Gateway service; routes declared by each provider unit |
| C3 | User Registry (U1) | U2, U3, U4, U5, U7, U8, U10 | REST/JSON internal | U1 |
| C4 | Authorisation (U2) | `account-api` and `portfolio` (BR2.13), the release smoke check (`identity-smoke`), and the gateway for the admin role change; Identity's own modules call the decision in-process | REST/JSON internal | U2 |
| C5 | Support Access (U3) | U9, U10 | REST/JSON internal | U3 |
| C6 | Preferences & Consent (U4) | U7 | REST/JSON internal | U4 |
| C7 | Account Lifecycle (U5) | U7, U10 | REST/JSON internal | U5 |
| C8 | Jurisdiction Gate (U6) | U1, U8 | REST/JSON internal | U6 |
| C9 | Audit service | All service units | REST/JSON event write | Audit service |

## C1/C2 — Gateway `/v1` (public surface)

```yaml
openapi: 3.1.0
info:
  title: Gateway /v1 — users, roles and permissions
  version: "1.0"
  description: >
    The single entry for the SPA and support console. The gateway validates the Supabase JWT,
    applies a deny-by-default route table, forwards the token unchanged, and adds
    x-correlation-id and an idempotency-key for non-idempotent methods.
servers: [{ url: /v1 }]
security: [{ bearerAuth: [] }]
components:
  securitySchemes:
    bearerAuth: { type: http, scheme: bearer, bearerFormat: JWT }
  schemas:
    Principal:
      type: object
      required: [userId, role, status, nextStep]
      properties:
        userId: { type: string }
        role: { type: string, enum: [owner, admin, support] }
        status: { type: string, enum: [pending, active, disabled, pending_deletion, deleted] }
        displayName: { type: string }
        emailVerified: { type: boolean }
        nextStep: { type: string, enum: [none, verify_email, enrol_mfa, restore_account] }
    ProblemDetails:
      type: object
      required: [type, title, status]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        detail: { type: string }
        correlationId: { type: string }
paths:
  /principals/{authUserId}:
    get:
      summary: Resolve or provision the caller's principal
      parameters:
        - { name: authUserId, in: path, required: true, schema: { type: string } }
      responses:
        "200": { description: Principal, content: { application/json: { schema: { $ref: "#/components/schemas/Principal" } } } }
        "401": { description: Missing or invalid token, content: { application/problem+json: { schema: { $ref: "#/components/schemas/ProblemDetails" } } } }
  /users/{id}:
    get:
      summary: Read a user record (owner or admin)
      responses:
        "200": { description: User record }
        "403": { description: Role denial, content: { application/problem+json: { schema: { $ref: "#/components/schemas/ProblemDetails" } } } }
        "404": { description: Not found or not disclosed }
  /support-grants:
    post:
      summary: Request a support grant (support role)
      responses:
        "201": { description: Grant created, pending approval }
        "422": { description: Reason outside 1–500 characters }
    get:
      summary: List pending grants (admin) — C5 shape plus requestedAt
      parameters:
        - { name: status, in: query, required: false, schema: { type: string, enum: [pending_approval] } }
      responses:
        "200": { description: Array of grants; requestedAt and approvedAt included for the console }
        "403": { description: Caller is not an admin }
        "422": { description: A status value other than pending_approval }
  /support-grants/{id}/approve:
    post:
      summary: Approve a support grant (admin, not the requester)
      responses:
        "200": { description: Grant active with expiresAt }
        "403": { description: Self-approval refused }
  /support-grants/mine:
    get:
      summary: The caller's own support grants (support role; self-scoped from the JWT)
      responses:
        "200":
          description: >
            A bare array of Grant rows, no envelope. Each row carries requestedBy, the agent who
            asked, so an own-grant row is attributable. Newest first, at most 20 rows.
        "401": { description: Missing or invalid token }
        "403": { description: Caller is not a support agent }
  /support-grants/{id}/metadata:
    get:
      summary: Read the allowlisted metadata projection under a live grant (support role)
      parameters:
        - { name: id, in: path, required: true, schema: { type: string } }
      responses:
        "200": { description: The SupportMetadata projection — allowlisted fields only }
        "403": { description: No live grant for this caller and target, or grant expired or revoked }
        "404": { description: No such grant, or not disclosed }
  /admin/users/lookup:
    post:
      summary: Exact-email admin lookup (admin; audited; address in the body — D-S1)
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [email]
              properties:
                email: { type: string, format: email }
      responses:
        "200": { description: Admin user record including role }
        "403": { description: Caller is not an admin }
        "404": { description: No user has that address }
        "422": { description: Malformed body }
        "503": { description: The provider role cannot be read }
  /admin/users/{id}/role:
    put:
      summary: Change a user's role (admin; audited)
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [newRole, reason]
              properties:
                newRole: { type: string, enum: [owner, admin, support] }
                reason: { type: string, minLength: 1, maxLength: 500 }
      responses:
        "200": { description: Role change applied and audited }
        "403": { description: Caller is not an admin }
        "422": { description: Blank reason or unknown role }
  /admin/users/{id}/disable:
    post:
      summary: Disable an account without deleting history (admin; audited)
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [reason]
              properties:
                reason: { type: string, minLength: 1, maxLength: 500 }
      responses:
        "200": { description: "{ userId, status, transitionId, occurredAt }" }
        "403": { description: Caller is not an admin }
        "404": { description: No such user }
        "409": { description: Edge outside BR5.1 }
        "422": { description: Reason outside 1–500 characters }
        "503": { description: Audit write failed; nothing changed }
  /admin/users/{id}/restore:
    post:
      summary: Restore a disabled account (admin; audited)
      responses:
        "200": { description: "{ userId, status, transitionId, occurredAt }" }
        "403": { description: Caller is not an admin }
        "404": { description: No such user }
        "409": { description: Edge outside BR5.1 }
        "422": { description: Reason outside 1–500 characters }
        "503": { description: Audit write failed; nothing changed }
```

The U1 `GET /admin/users?email=` search is removed: an address must not reach a URL or an access
log (D-S1), and `/admin/users/lookup` replaces it. The two paths are separate entries, so the old
one now answers with the gateway's deny-by-default `404`.

- Undeclared paths return `404`; a missing or invalid token `401`; a role denial `403`. The SPA and support console never call a service directly.

## C3 — User Registry (U1), internal

```yaml
openapi: 3.1.0
info: { title: User Registry internal contract, version: "1.0" }
components:
  schemas:
    User:
      type: object
      required: [id, displayName, emailVerified, status, createdAt]
      properties:
        id: { type: string }
        displayName: { type: string }
        email: { type: string, format: email }
        emailVerified: { type: boolean }
        status: { type: string, enum: [pending, active, disabled, pending_deletion, deleted] }
        createdAt: { type: string, format: date-time }
        updatedAt: { type: string, format: date-time }
    AdminUserAnswer:
      type: object
      description: >
        The admin lookup answer (U10, W1). It is the User record plus the provider's current
        app_metadata.role, read at lookup time through the RoleReader port. A provider read
        failure is a 503 and no record — never a stale role.
      properties:
        role: { type: string, enum: [owner, admin, support] }
        emailVerified: { type: boolean }
        status: { type: string, enum: [pending, active, disabled, pending_deletion, deleted] }
        createdAt: { type: string, format: date-time }
        updatedAt: { type: string, format: date-time }
    PendingGrant:
      type: object
      description: >
        The C5 Grant shape as the console reads it from `GET /v1/support-grants` (W5): the same
        id, userId, requestedBy, reason, status and expiresAt, plus requestedAt and approvedAt,
        which the console's grant mapper requires.
      properties:
        requestedAt: { type: string, format: date-time }
        approvedAt: { type: string, format: date-time, nullable: true }
paths:
  /internal/users/by-subject/{subjectHash}:
    get:
      summary: Look up the internal user for a hashed provider subject
      responses:
        "200": { description: User }
        "404": { description: Not provisioned }
    put:
      summary: Provision or refresh a user from validated claims
      responses:
        "200": { description: Existing user refreshed }
        "201": { description: User provisioned }
        "422": { description: Jurisdiction gate has not cleared the provider }
  /internal/users/{id}/status:
    put:
      summary: Apply a status transition requested by Account Lifecycle
      responses:
        "200": { description: New status }
        "409": { description: Illegal transition }
```

## C4 — Authorisation (U2), internal

**One implementation, two delivery paths.** Every Identity module calls the
decision in-process; services outside Identity call the same decision through
`POST /internal/authorize`. No shared crate and no per-request network hop
inside Identity (U2 code generation, `code-generation-plan.md`).

**Caller identity and transport.** The endpoint accepts the calling service's
own token in `x-service-token` and the acting user's Supabase JWT, unchanged, in
`Authorization`. The subject and role come from that validated JWT only; a body
that carries `callerSubject` or `roleClaim` is refused with **400** — never
ignored, never trusted. A service token alone authorises nothing user-scoped.
The endpoint is reached directly on the internal network and is not in the
gateway's `/v1` route table.

```yaml
openapi: 3.1.0
info: { title: Authorisation internal contract, version: "2.0" }
components:
  securitySchemes:
    serviceToken: { type: apiKey, in: header, name: x-service-token }
    bearerAuth: { type: http, scheme: bearer, bearerFormat: JWT }
  schemas:
    Decision:
      type: object
      required: [allowed, reason, httpStatus]
      properties:
        allowed: { type: boolean }
        reason: { type: string, enum: [allowed, no_role, status_denied, not_owner, not_declared, audit_unavailable] }
        httpStatus: { type: integer, enum: [200, 403, 404, 503] }
        filterOwnerId:
          type: string
          description: Present on an allowed collection; every returned row must carry this owner.
    Operation:
      type: string
      description: >
        A catalogue name. A name outside the catalogue is denied not_declared,
        never coerced. The class is fixed by the catalogue: owner-scoped,
        collection, administrative (any admin:* name), confirmation, or
        support-metadata.
      enum:
        - principal:read
        - profile:read
        - profile:write
        - resource:read
        - resource:write
        - collection:list
        - collection:search
        - collection:export
        - guard:confirm
        - support:metadata:read
        - "admin:*"
    ProblemDetails:
      type: object
      required: [type, title, status]
      properties:
        type: { type: string, description: "Ends with the denial reason (for example /identity/not_owner)." }
        title: { type: string }
        status: { type: integer }
        detail: { type: string }
        correlationId: { type: string }
security: [{ serviceToken: [], bearerAuth: [] }]
paths:
  /internal/authorize:
    post:
      summary: Evaluate role, status, support, and ownership for one operation
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [operation]
              properties:
                operation: { $ref: "#/components/schemas/Operation" }
                resourceKind:
                  type: string
                  description: account, portfolio, transaction, snapshot, allocation, order, preference, export, or deletion request.
                scope: { type: string, enum: [resource, collection] }
                resourceOwnerId:
                  type: string
                  format: uuid
                  description: Required when scope is resource; a missing or ambiguous owner is denied not_owner.
      responses:
        "200": { description: Allowed, content: { application/json: { schema: { $ref: "#/components/schemas/Decision" } } } }
        "400": { description: "The body carries callerSubject or roleClaim, or is malformed" }
        "401": { description: "Missing or invalid service token, or missing or invalid user JWT" }
        "403": { description: "no_role, status_denied, or not_declared; also a caller or operation outside the per-service allowlist" }
        "404": { description: "not_owner — the same answer a nonexistent id receives (non-disclosure)" }
        "503": { description: "status_unavailable when the status source cannot answer, or audit_unavailable when the denial could not be recorded" }
  /internal/admin/role:
    put:
      summary: Change a user's app_metadata.role (admin only; audited)
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [userId, newRole, reason]
              properties:
                userId: { type: string, format: uuid }
                newRole: { type: string, enum: [owner, admin, support] }
                reason: { type: string, minLength: 1, maxLength: 500 }
      responses:
        "200": { description: "Role write accepted; identity.role.change.requested and identity.role.changed recorded" }
        "403": { description: "The caller is not an admin, or the calling service is not the gateway" }
        "422": { description: "Blank or over-long reason, an unknown role, or a non-UUID user id" }
        "503": { description: "The intent or outcome audit entry could not be recorded, or the provider write failed (identity.role.changed outcome=failed)" }
```

- **Consumers.** The only HTTP consumers are `account-api` and `portfolio` (the
  BR2.13 callers) and the release smoke check (`identity-smoke`). U3 (support
  access) and every other Identity module call the decision in-process; they are
  not C4 callers. The admin role change has two surfaces over the same use case:
  the gateway route `PUT /v1/admin/users/{userId}/role` (JWT-authenticated; the
  gateway requires the admin role at the edge and Identity re-checks it from the
  same token) and the internal `PUT /internal/admin/role` for a service caller.
- **Allowlist.** `POST /internal/authorize` admits only the services configured
  in `IDENTITY_AUTHORIZE_SERVICES` (default `account-api,portfolio,identity-smoke`),
  and the allowlist names operations, not classes: `account-api`/`portfolio` may
  ask for `principal:read`, `profile:read`, `profile:write`, `resource:read`,
  `resource:write`, `collection:list`, `collection:search`, and
  `collection:export`; `identity-smoke` may ask only for `principal:read`. Any
  other service, and any other operation for that service — including `admin:*`,
  `guard:confirm`, and `support:metadata:read` — is 403 before the decision runs.
- **Error envelope.** A denial is RFC 7807 `ProblemDetails` whose `type` ends in
  the reason; the allowed body is the `Decision` document.

## C5 — Support Access (U3), internal

```yaml
openapi: 3.1.0
info: { title: Support Access internal contract, version: "1.0" }
components:
  schemas:
    Grant:
      type: object
      required: [id, userId, requestedBy, approvedBy, reason, status, expiresAt]
      properties:
        id: { type: string }
        userId: { type: string }
        requestedBy: { type: string }
        approvedBy: { type: string }
        reason: { type: string, minLength: 1, maxLength: 500 }
        status: { type: string, enum: [pending_approval, active, denied, revoked, expired] }
        requestedAt: { type: string, format: date-time }
        approvedAt: { type: string, format: date-time, nullable: true }
        expiresAt: { type: string, format: date-time, nullable: true, description: "Null until approval; set at approval" }
    SupportMetadata:
      type: object
      description: The only fields a grant exposes; never holdings, balances, values, transactions, orders, or credentials.
      properties:
        userId: { type: string }
        displayName: { type: string }
        emailVerified: { type: boolean }
        accounts:
          type: array
          items:
            type: object
            properties:
              id: { type: string }
              label: { type: string }
              sourceType: { type: string }
              syncStatus: { type: string }
              lastSyncedAt: { type: string, format: date-time }
        portfolios:
          type: array
          items:
            type: object
            properties:
              id: { type: string }
              label: { type: string }
paths:
  /internal/support-grants:
    post:
      summary: A support agent requests a grant for one user record
      responses:
        "201": { description: Grant in pending_approval }
        "422": { description: Reason outside 1–500 characters }
    get:
      summary: List grants filtered by status (admin) — powers the console's Approvals panel (W5)
      parameters:
        - { name: status, in: query, required: false, schema: { type: string, enum: [pending_approval] } }
      responses:
        "200": { description: Array of Grant }
        "403": { description: Caller is not an admin }
        "422": { description: A status value other than pending_approval }
  /internal/support-grants/{id}/approve:
    post:
      summary: A different admin approves a grant
      responses:
        "200": { description: Grant active; expiresAt <= 60 minutes after approval }
        "403": { description: Requester equals approver }
  /internal/support-grants/mine:
    get:
      summary: The support agent's own grants (self-scoped from the token; S2)
      responses:
        "200":
          description: >
            A bare array of Grant rows, newest first, at most 20. Scoped to the caller's support
            agent id, never a request parameter; each row carries requestedBy.
        "403": { description: Caller is not a support agent }
  /internal/support-grants/{id}/metadata:
    get:
      summary: Read the metadata projection under an active grant
      responses:
        "200": { description: SupportMetadata }
        "403": { description: Grant expired, revoked, or unapproved }
```

## C6 — Preferences & Consent (U4), internal

```yaml
openapi: 3.1.0
info: { title: Preferences and Consent internal contract, version: "1.0" }
paths:
  /internal/users/{id}/preferences:
    get:
      summary: Read a user's preferences
      responses: { "200": { description: Preferences } }
    put:
      summary: Update locale, base currency, timezone, notifications
      responses:
        "200": { description: Updated }
        "422": { description: Unsupported locale, currency, or timezone }
  /internal/users/{id}/consent:
    get:
      summary: Read consent history
      responses: { "200": { description: Consent records } }
    post:
      summary: Grant or withdraw a consent kind (terms, risk, age, analytics)
      responses:
        "201": { description: Consent recorded with timestamp }
```

## C7 — Account Lifecycle (U5), internal

The console reaches the same two use cases over the gateway (U10, C1/C2):
`POST /v1/admin/users/{id}/disable` and `POST /v1/admin/users/{id}/restore`, each with body
`{"reason":"…"}` (1–500 characters) and answering `{userId, status, transitionId, occurredAt}`.
A reason outside the bounds is `422`; an edge outside BR5.1 is `409`; a failed audit write is `503`
and nothing changes.

```yaml
openapi: 3.1.0
info: { title: Account Lifecycle internal contract, version: "1.0" }
paths:
  /internal/users/{id}/disable:
    post:
      summary: Disable an account without deleting history (admin)
      responses:
        "200": { description: Status disabled; audit event emitted }
  /internal/users/{id}/restore:
    post:
      summary: Restore a disabled or pending-deletion account
      responses:
        "200": { description: Status active }
        "403": { description: Caller is not an admin (restore only) }
  /internal/users/{id}/deletion-requests:
    post:
      summary: Request user-initiated deletion; returns the 30-day recovery date
      responses:
        "201": { description: Status pending_deletion; recoveryEndsAt }
    delete:
      summary: Cancel a pending deletion
      responses:
        "200": { description: Status active }
```

## C8 — Jurisdiction Gate (U6), internal

```yaml
openapi: 3.1.0
info: { title: Jurisdiction Gate internal contract, version: "1.0" }
paths:
  /internal/jurisdiction/check:
    post:
      summary: Evaluate a self-declared country plus an IP signal
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [countryCode, ip]
              properties:
                countryCode: { type: string, pattern: "^[A-Z]{2}$" }
                ip: { type: string }
      responses:
        "200": { description: "allowed: boolean, restrictedBy: self_declared|ip|both, evidenceId" }
        "503": { description: Geolocation unavailable; nothing was stored }
```

## C9 — Audit event write

```yaml
openapi: 3.1.0
info: { title: Audit event internal contract, version: "1.0" }
paths:
  /internal/events:
    post:
      summary: Append one security-sensitive event
      requestBody:
        content:
          application/json:
            schema:
              type: object
              required: [actor, action, target, occurredAt, outcome]
              properties:
                actor: { type: string, description: User id, service name, or "system" }
                action: { type: string }
                target: { type: string }
                occurredAt: { type: string, format: date-time }
                context: { type: object }
                outcome: { type: string, enum: [success, denied, failed] }
                correlationId: { type: string }
      responses:
        "202": { description: Accepted for append }
```

Events never carry credentials, tokens, full wallet addresses, or secrets.

## Contract ownership rules

- **Owner:** the provider unit owns its spec and its route declarations; the gateway service owns the `/v1` surface and the deny-by-default table.
- **Additive changes:** a new optional field or a new endpoint is additive and ships within `/v1`; consumers ignore unknown fields.
- **Breaking changes:** a removed field, a renamed field, a changed type, or a changed status code is breaking; it requires a `/v2` path and a deprecation window announced in the same release.
- **Error envelope:** every provider returns RFC 7807 `ProblemDetails` with the correlation id; 401 for a missing or invalid token, 403 for a role denial, 404 for an undeclared route or a non-disclosure.
- **Timeouts and retries:** connect 5s, read 10s; at most 3 attempts with exponential backoff and jitter; retries only on idempotent calls carrying an idempotency key; never retry 400/403/404/422.
- **Audit:** a provider's security-sensitive action is only complete once the Audit write is accepted or the failure is surfaced (never swallowed).

## Open questions

| Contract | Question | Blocks |
|---|---|---|
| C1/C2 | The gateway route table currently declares only `/v1/assets`; the BRD-002 routes must be added per provider unit. Who confirms the ordered route list? | U1–U5 route work |
| C3 | Should `pending` remain a stored status alongside the four BRD statuses, and how do existing rows migrate? | U1, U5 |
| C4 | Is the authorisation decision a synchronous call per request or a shared library? (ADR-002 leaves it open.) | **Resolved in U2:** one in-process implementation inside Identity, plus the C4 endpoint as a thin adapter for callers outside the service. No shared crate, no in-Identity network hop. |
| C5 | Where do account and portfolio metadata come from — the Account and Portfolio APIs, or an aggregation path? | U3 |
| C8 | Which jurisdictions are on the launch list, and who maintains it? | U6 |
