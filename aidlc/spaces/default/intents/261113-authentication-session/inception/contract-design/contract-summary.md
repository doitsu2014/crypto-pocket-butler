# Contract Summary — Authentication & Session Management

**Intent:** BRD-003 Authentication & Session Management
**Contract version:** `/v1` additive compatibility line
**Unit boundaries:** U1 `u1-identity-auth-domain`; U2 `u2-identity-gateway`; U3 `u3-spa-auth-sessions`; U4 `u4-auth-integration-proof`.

**Upstream topology:** `unit-of-work.md` defines these four Units and their responsibilities. `unit-of-work-dependency.md` records the direct DAG: U1 has no prerequisites; U2 depends on U1; U3 depends on U2; U4 depends on U1, U2 and U3. The contracts below correspond to those inter-Unit boundaries and external consumers; the DAG does not prescribe a delivery order beyond its dependencies.

The interfaces below are the agreements between Units and external consumers. The specs define
paths, payloads, status codes, ownership, and failure behavior, not a service split: Identity, gateway
and the SPA remain within their existing deployables. Supabase Auth is an external provider, not a
new application service.

## Contracts table

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---|---|---|---|
| C1 | U1 Identity domain/auth application | U2 Identity HTTP adapters | In-process ports and typed application results | U1 owns ports and semantics; U2 owns HTTP mapping |
| C2 | U2 API Gateway public route surface | U3 SPA | HTTPS `/v1` REST; user's Supabase JWT | U2 owns the gateway contract; U3 consumes it |
| C3 | U2 Gateway / Identity public user routes | External: browser session | HTTPS `/v1` REST; unchanged user JWT forwarded to Identity | U2 owns route declaration and forwarding; Identity owns handler |
| C4 | U1 Identity auth application | U2 Identity HTTP and infrastructure adapters | Supabase Auth Admin API via `AuthAdmin` port | U1 owns intent and errors; U2 adapter owns provider mapping |
| C5 | U1 Identity auth application | U2 internal HTTP adapter | Identity service-token-protected internal REST | Identity owns operation semantics; U2 owns HTTP boundary |
| C6 | U1 Identity auth application | U2 Notification adapter / Notification service | Synchronous internal HTTP; bounded retry + idempotency key | U1 owns alert decision; Notification owns delivery |
| C7 | U1 Identity auth application | U2 Audit outbox adapter / Audit service | Existing transactional outbox and append-only audit HTTP contract | Identity owns event creation; Audit owns immutable storage |
| C8 | U4 integration proof | U1, U2, U3 | Browser/API automated test contract; no runtime API | U4 owns proof artifacts/tests; each Unit owns implementation |
| E1 | External: Supabase Auth | U3 SPA | Supabase SDK direct-browser auth (sole direct external SPA exception) | Supabase defines provider surface; U3 owns client integration |
| E2 | External: Supabase JWKS | U2 gateway and Identity validators | HTTPS JWKS retrieval and cache | Provider owns JWKS; each validator owns fail-closed validation |

---

## Contract ownership and compatibility rules

1. **The SPA calls only the API Gateway for product APIs.** Supabase Auth is the sole direct browser
   exception for authentication flows. No SPA-to-Identity direct route exists.
2. **The gateway forwards the user's Supabase JWT unchanged** in `Authorization`. The gateway and
   every service independently validate signature, issuer, audience, expiry, and the same documented
   clock-skew allowance. A JWT is never replaced by a gateway-minted identity token.
3. **Privileged gateway-to-Identity operations use the existing Identity service token separately**
   from the unchanged user JWT. User identity is derived from the validated JWT; a request body must
   never supply `callerSubject` or `role` as authority.
4. **Identity owns policy.** Signup-gate, jurisdiction, MFA/AAL2, user/device ownership, recovery
   authorization, and login-alert eligibility are decided by Identity. U3 presents choices and
   outcomes; it is not an authority for these rules.
5. **The gateway route table is deny-by-default.** Every SPA-called path is declared with one owning
   handler. Auth endpoints carry stricter per-route rate budgets than ordinary reads.
6. **`/v1` changes are additive and backward compatible.** Consumers ignore unknown response fields.
   Removing a field, changing a status meaning, or changing authorization semantics requires a new
   API version and a coordinated consumer migration.
7. **Errors use the existing `application/problem+json` RFC 7807 shape** with stable `type`, title,
   status, optional safe detail, field path/reason pairs, and correlation id. Error bodies never
   include rejected values, tokens, credentials, factor seeds, recovery codes, full fingerprints,
   DSNs, or upstream provider detail.
8. **Side-effecting retries are bounded and idempotent.** Use exponential backoff with jitter for
   transient failures only, a stable idempotency key per logical operation, and no retry for
   validation, authorization, or other permanent 4xx refusals. A retry must not issue two
   revocations, recovery completions, or login alerts.
9. **No message queues are introduced.** Internal service calls are synchronous HTTP with bounded
   retries and idempotency keys, consistent with team practice.

---

## C1 — Identity domain ports consumed by U2 adapters

This boundary is in-process, not HTTP. The port signatures are described at the semantic level;
concrete Rust types belong to Functional Design and the Unit's implementation.

| Port operation | Request shape | Result | Failure behavior |
|---|---|---|---|
| `evaluate_signup_gate` | Authenticated subject; country; terms accepted; risk acknowledged; age confirmed; idempotency key | Gate result and opaque authorization reference | `validation_refused` for missing choice; `jurisdiction_refused` for unsupported region; never provision on refusal |
| `get_gate_status` | Authenticated user id | Required/missing gate items and outcome | Not found is non-enumerating; provider/database outage is recoverable `503` |
| `get_principal` | Validated subject from JWT | Internal user id, role, status, email verification, MFA requirement and AAL | Disabled user is denied; unknown valid subject triggers idempotent lazy creation |
| `list_sessions` | Authenticated subject; requested owner id; page cursor | Session rows: id, label, coarse location, last-seen, current marker, state | Owner mismatch returns non-disclosing `404`; no token or fingerprint returned |
| `revoke_session` | Authenticated subject; owner id; session id; current-session confirmation; idempotency key | Revocation outcome and remaining access-window notice | Provider refusal leaves local row unchanged; already-revoked is idempotent success |
| `revoke_other_sessions` | Authenticated subject; idempotency key | Per-session success/failure result | Partial failure is reported by session; no false aggregate success |
| `enrol_totp` / `verify_totp` | Authenticated subject; provider factor reference; code (verification only) | Factor state and AAL | Invalid/expired code is `422`; provider outage is safe `503`; code/secret excluded from logs |
| `open_recovery_case` | Recovery contact; reason; idempotency key | Opaque case id; neutral user-facing outcome | Same response for known/unknown accounts; rate-limited refusal includes retry hint |
| `complete_recovery` | Case id; authorized actor; approved identity-proof result; idempotency key | Completion state and revocation results | Unauthorized approver denied; failed proof changes nothing; partial revocation is explicit |
| `record_login_event` | Validated subject; device label; coarse location; event time; idempotency key | Alert decision and delivery outcome | Notification failure never rolls back sign-in; failure is surfaced to operations |

`callerSubject`, user role, tokens, factor secrets, recovery codes, and full device fingerprints are
never accepted as caller-controlled authority fields in an application command body.

---

## C2 — U3 SPA to public gateway

All paths below are under `/v1`. The SPA supplies `Authorization: Bearer <Supabase JWT>` for protected
routes, plus `x-correlation-id`; it never calls the Identity service directly.

```yaml
openapi: 3.1.0
info:
  title: Crypto Pocket Butler Authentication API (SPA to Gateway)
  version: v1
servers:
  - url: /v1
security:
  - SupabaseBearer: []
paths:
  /signup-gate:
    post:
      operationId: evaluateSignupGate
      security:
        - SupabaseBearer: []
      x-rate-limit: auth-strict
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [countryCode, termsAccepted, riskAcknowledged, ageConfirmed]
              properties:
                countryCode: { type: string, minLength: 2, maxLength: 2 }
                termsAccepted: { type: boolean, const: true }
                riskAcknowledged: { type: boolean, const: true }
                ageConfirmed: { type: boolean, const: true }
      responses:
        '200':
          description: Gate accepted; response contains opaque single-use authorization reference.
          content:
            application/json:
              schema:
                type: object
                required: [gateAuthorization]
                properties:
                  gateAuthorization: { type: string, format: opaque }
        '401': { $ref: '#/components/responses/Problem' }
        '422': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /signup-gates/me:
    get:
      operationId: getMySignupGateStatus
      x-rate-limit: auth-strict
      responses:
        '200':
          description: Current user's gate status and missing server-required items.
          content:
            application/json:
              schema:
                type: object
                required: [status, missing]
                properties:
                  status: { type: string, enum: [required, passed, blocked] }
                  missing: { type: array, items: { type: string } }
        '401': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /users/me/sessions:
    get:
      operationId: listMySessions
      x-rate-limit: auth-strict
      parameters:
        - in: query
          name: cursor
          schema: { type: string }
      responses:
        '200':
          description: Caller-owned device sessions only.
          content:
            application/json:
              schema:
                type: object
                required: [items]
                properties:
                  items:
                    type: array
                    items: { $ref: '#/components/schemas/DeviceSession' }
                  nextCursor: { type: string, nullable: true }
        '401': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /users/me/sessions/{sessionId}/revoke:
    post:
      operationId: revokeMySession
      x-rate-limit: auth-strict
      parameters:
        - in: path
          name: sessionId
          required: true
          schema: { type: string }
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [confirmCurrent]
              properties:
                confirmCurrent: { type: boolean }
      responses:
        '200':
          description: Session already revoked or revoked now.
          content:
            application/json:
              schema: { $ref: '#/components/schemas/RevocationResult' }
        '404': { $ref: '#/components/responses/Problem' }
        '409': { $ref: '#/components/responses/Problem' }
        '401': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /users/me/sessions/revoke-others:
    post:
      operationId: revokeOtherSessions
      x-rate-limit: auth-strict
      parameters:
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      responses:
        '200':
          description: Per-session results; current session remains active.
          content:
            application/json:
              schema:
                type: object
                required: [ended, remaining]
                properties:
                  ended: { type: integer, minimum: 0 }
                  remaining:
                    type: array
                    items: { $ref: '#/components/schemas/SessionFailure' }
        '401': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /mfa/totp/enrolments:
    post:
      operationId: startTotpEnrolment
      x-rate-limit: auth-strict
      parameters:
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      responses:
        '201':
          description: One-time enrollment material. Secret is returned once only.
          content:
            application/json:
              schema:
                type: object
                required: [factorId, secret, otpauthUrl]
                properties:
                  factorId: { type: string }
                  secret: { type: string, writeOnly: true }
                  otpauthUrl: { type: string, writeOnly: true }
        '401': { $ref: '#/components/responses/Problem' }
        '409': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /mfa/totp/enrolments/{factorId}/verify:
    post:
      operationId: verifyTotpEnrolment
      x-rate-limit: auth-strict
      parameters:
        - in: path
          name: factorId
          required: true
          schema: { type: string }
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [code]
              properties:
                code: { type: string, pattern: '^[0-9]{6}$', writeOnly: true }
      responses:
        '200':
          description: Factor verified; recovery codes are returned once.
          content:
            application/json:
              schema:
                type: object
                required: [aal, recoveryCodes]
                properties:
                  aal: { type: string, enum: [aal2] }
                  recoveryCodes:
                    type: array
                    items: { type: string, writeOnly: true }
        '422': { $ref: '#/components/responses/Problem' }
        '401': { $ref: '#/components/responses/Problem' }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /auth/verification/resend:
    post:
      operationId: resendVerification
      x-rate-limit: auth-strict
      responses:
        '202': { description: Neutral accepted response; does not reveal account existence. }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
  /recoveries:
    post:
      operationId: openRecoveryCase
      security: []
      x-rate-limit: auth-strict
      parameters:
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [contact, reason]
              properties:
                contact: { type: string, format: email }
                reason: { type: string, minLength: 1, maxLength: 1000 }
      responses:
        '202': { description: Neutral response, identical for known and unknown accounts. }
        '429': { $ref: '#/components/responses/Problem' }
        '503': { $ref: '#/components/responses/Problem' }
components:
  securitySchemes:
    SupabaseBearer:
      type: http
      scheme: bearer
      bearerFormat: JWT
  schemas:
    DeviceSession:
      type: object
      required: [sessionId, deviceLabel, lastSeenAt, current, state]
      properties:
        sessionId: { type: string }
        deviceLabel: { type: string }
        coarseLocation: { type: string, nullable: true }
        lastSeenAt: { type: string, format: date-time }
        current: { type: boolean }
        state: { type: string, enum: [active, revoked] }
      additionalProperties: true
    RevocationResult:
      type: object
      required: [revoked, residualWindowSeconds]
      properties:
        revoked: { type: boolean }
        residualWindowSeconds: { type: integer, minimum: 0, maximum: 3600 }
      additionalProperties: true
    SessionFailure:
      type: object
      required: [sessionId, deviceLabel, reason]
      properties:
        sessionId: { type: string }
        deviceLabel: { type: string }
        reason: { type: string }
      additionalProperties: true
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string, format: uri }
        title: { type: string }
        status: { type: integer }
        detail: { type: string }
        errors:
          type: array
          items:
            type: object
            required: [field, reason]
            properties:
              field: { type: string }
              reason: { type: string }
        correlationId: { type: string }
      additionalProperties: true
  responses:
    Problem:
      description: RFC 7807 application/problem+json. No rejected value, token, secret, fingerprint, or upstream detail.
      content:
        application/problem+json:
          schema: { $ref: '#/components/schemas/Problem' }
```

### C2 failure and security rules

- Missing/invalid bearer token: `401`; insufficient role: `403`; ownership outside reach: `404`;
  state conflict (e.g. current session without confirmation): `409`; invalid gate/code: `422`;
  throttled: `429` with `Retry-After`; dependency outage: `503` with stable non-revealing problem type.
- The SPA sends a stable `Idempotency-Key` for each logical revocation, TOTP enrolment/verification,
  and recovery submission. Repeating the same key and body returns the original result; reusing the
  key with a different body is `409`.
- `DeviceSession` includes only coarse location and device label; never IP address, user-agent string,
  or full device fingerprint.
- `RevocationResult.residualWindowSeconds` is bounded by the 1-hour access-token expiry and is
  communicated in the UI; no server-side deny-list is implied by this contract.

## C3 — Public gateway route table to Identity handlers

Every SPA path in C2 is added to the gateway's deny-by-default table with Identity as its sole owner.
Gateway-to-Identity forwarding has two credentials with distinct roles:

- `Authorization: Bearer <user Supabase JWT>` forwarded unchanged and validated by Identity.
- `x-service-token: <Identity service token>` for gateway-authorized privileged route invocation,
  validated through the existing service-token mechanism.

No `callerSubject` or `role` body field is accepted as identity. Identity derives subject, role and
status from the validated JWT and the internal registry. The gateway consumes the Identity principal
and authorization interface for edge policy (including MFA/AAL2 and disabled status); it does not
own or cache these policy decisions in an independent authority.

Public route contract mapping:

| SPA route (C2) | Identity owner | Gateway declaration | Auth class |
|---|---|---|---|
| `POST /v1/signup-gate` | Identity Signup Gates | required | Auth-strict; pre-provisioning gate |
| `GET /v1/signup-gates/me` | Identity Signup Gates | required | User JWT |
| `GET /v1/users/me/sessions` | Identity Device Sessions | required | User JWT; owner only |
| `POST /v1/users/me/sessions/{sessionId}/revoke` | Identity Device Sessions | required | User JWT; owner only |
| `POST /v1/users/me/sessions/revoke-others` | Identity Device Sessions | required | User JWT; owner only |
| `POST /v1/mfa/totp/enrolments` | Identity MFA Policy | required | User JWT; AAL step-up where needed |
| `POST /v1/mfa/totp/enrolments/{factorId}/verify` | Identity MFA Policy | required | User JWT |
| `POST /v1/auth/verification/resend` | Identity Email Verification | required | User JWT or neutral public action per handler policy |
| `POST /v1/recoveries` | Identity Recovery | required | Public, rate-limited; neutral response |

`POST /v1/recoveries` is the only public unauthenticated recovery entry. Its handler must not reveal
whether a user exists. All other paths are authenticated. Paths absent from the table remain denied.

## C4 — Identity to Supabase Auth Admin API

The existing `AuthAdmin` port is the application boundary. U2's infrastructure adapter calls the
provider Admin API. The provider error vocabulary is translated into typed `ExternalError` outcomes;
provider text, keys, URLs, and raw bodies never cross the port.

```yaml
openapi: 3.1.0
info:
  title: Supabase Auth Admin operations used by Identity
  version: provider-configured
paths:
  /admin/users/{userId}/sessions/{sessionId}:
    delete:
      operationId: revokeProviderSession
      parameters:
        - in: path
          name: userId
          required: true
          schema: { type: string }
        - in: path
          name: sessionId
          required: true
          schema: { type: string }
      responses:
        '204': { description: Provider session revoked or already absent. }
        '401': { description: Service-role credential invalid; map to fatal configuration error, never echo detail. }
        '404': { description: Session absent; map to idempotent success only when provider semantics confirm absence. }
        '429': { description: Rate limited; retry with bounded backoff and same operation idempotency key. }
        '5XX': { description: Provider unavailable; map to recoverable ExternalError and do not mutate local session row. }
```

Provider API path is illustrative of the operation; the adapter uses the configured Supabase endpoint
and the versioned provider SDK/API actually pinned in the implementation. No endpoint is
hard-coded. Identity calls provider revocation **before** marking its local session row revoked.

## C5 — Gateway / support operations to Identity

Privileged Identity operations use the existing service token in addition to a user JWT where a
user is being acted on. The service token establishes caller service authority; the user JWT
establishes the subject and user context. A support actor's role and grant are checked by Identity.

| Operation | Caller | Identity check | Result |
|---|---|---|---|
| Resolve principal | Gateway | JWT signature/claims; internal user status | `Principal` (user id, role, status, MFA policy state) |
| List another user's sessions | Support Console via Gateway | Admin role plus active support grant/reason if support path | Session view with coarse labels only |
| Revoke another user's session | Support Console via Gateway | Admin role, active grant and audit context; ownership/admin policy | Revocation outcome; no existence disclosure to unauthorized caller |
| Continue recovery case | Support Console via Gateway | Authorized support agent and distinct approval per existing grant policy; actual recovery approver remains open in requirements Q4 | Case state and allowed next action |

All operations produce an append-only audit event with actor, action, target, time, context and
outcome. A denied call does not disclose another user's existence.

## C6 — Identity to Notification for in-app login alert

The call is made only after Identity classifies a successful sign-in as first-seen device. The
"unusual location" detector is still an open requirements question; until it is specified, it does
not trigger an implementation path.

```yaml
openapi: 3.1.0
info:
  title: Notification in-app alert contract
  version: internal-v1
paths:
  /internal/notifications:
    post:
      operationId: createLoginAlert
      parameters:
        - in: header
          name: x-service-token
          required: true
          schema: { type: string, writeOnly: true }
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [subjectUserId, kind, deviceLabel, occurredAt]
              properties:
                subjectUserId: { type: string }
                kind: { type: string, const: new_device_sign_in }
                deviceLabel: { type: string, maxLength: 120 }
                coarseLocation: { type: string, nullable: true, maxLength: 80 }
                occurredAt: { type: string, format: date-time }
                correlationId: { type: string }
              additionalProperties: false
      responses:
        '201': { description: Notification created. }
        '200': { description: Idempotent replay; prior notification returned. }
        '401': { description: Invalid service token. }
        '409': { description: Idempotency key reused with a different payload. }
        '429': { description: Rate limited; Retry-After is returned. }
        '503': { description: Notification unavailable. Identity preserves sign-in success and records delivery failure. }
```

No access token, refresh token, IP, full user agent, full device fingerprint, email credential or
factor data is included in the alert payload. The Idempotency-Key is a stable digest of the logical
sign-in event id, not a user secret.

## C7 — Identity to append-only Audit

Identity writes the event intent in its own outbox in the same database transaction as the state
change, then the existing drainer calls Audit's append endpoint with an idempotency key. This
preserves the project's transactional outbox pattern without adding a message queue.

```yaml
openapi: 3.1.0
info:
  title: Audit append contract for Identity auth events
  version: internal-v1
paths:
  /internal/audit/events:
    post:
      operationId: appendSecurityEvent
      parameters:
        - in: header
          name: x-service-token
          required: true
          schema: { type: string, writeOnly: true }
        - in: header
          name: Idempotency-Key
          required: true
          schema: { type: string, minLength: 16 }
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [actor, action, target, occurredAt, context, outcome]
              properties:
                actor: { type: string }
                action: { type: string }
                target: { type: string }
                occurredAt: { type: string, format: date-time }
                context: { type: object, additionalProperties: true }
                outcome: { type: string, enum: [success, refused, failed] }
                correlationId: { type: string }
              additionalProperties: false
      responses:
        '201': { description: Event appended. }
        '200': { description: Idempotent replay; prior event returned. }
        '401': { description: Invalid service token. }
        '409': { description: Idempotency key reused with a different event. }
        '422': { description: Payload contains a prohibited sensitive field. }
        '503': { description: Audit unavailable; Identity retains the outbox entry and retries boundedly. }
```

The outbox may not contain secrets or provider tokens. Audit rejects any payload field carrying a
credential, token, recovery code, factor seed or full device fingerprint. Audit records are append
only.

## C8 — U4 integration proof contract

U4 consumes the interface contracts above and provides evidence, not runtime API. Its acceptance
proof includes:

1. Every SPA auth/session path in C2 exists in the gateway route table and maps to one Identity
   handler (or the explicitly external Supabase Auth SDK path).
2. The gateway forwards the user JWT unchanged and supplies the service token only for privileged
   internal operations; the service derives subject/role from the JWT.
3. Identity ownership checks prevent one user reading or revoking another user's sessions; the
   authorized admin path is covered once its story-level requirement is reconciled.
4. Revocation provider-first ordering is tested, including provider failure and idempotent replay.
5. Login-alert retry is bounded/idempotent and never rolls back a successful sign-in.
6. RFC 7807 error responses contain correlation id and no rejected value, token, secret, or upstream
   detail.
7. Browser end-to-end auth tests use the approved network-mocking setup and `yarn e2e` gates any auth
   surface change.

U4's tests may use fixtures and mocks. They may not access another service's schema, bypass the
route table, or turn the SPA into a direct service client.

## Open questions

| Contract | Question | Blocks |
|---|---|---|
| C2/C3 | Is the absolute 30-day session cap measured from first sign-in or last refresh? | U1/U2 implementation detail; requirements Q3 remains open |
| C5 | Does support-assisted recovery reuse the existing distinct-admin support grant approver or need a dedicated recovery approver? | U1 recovery authorization and C5 route policy |
| C6 | What measurable rule defines an "unusual location" beyond a first-seen device? | U1 alert eligibility; requirements Q5 remains open |
| C3 | What exact `x-service-token` provisioning/rotation contract is used by Gateway and Identity? | U2 deployment/configuration; use the existing Vault-backed mechanism |
| C2 | Which exact gateway route path spellings replace existing SPA paths (`/mfa/enrol`, `/signup-gate`, `/users/.../sessions`)? | U2/U3 must agree before implementation; this spec proposes canonical paths above |
| C2 | Admin list/revoke acceptance criteria are not yet represented by a story; reconcile FR4.7 with the story map before pinning admin operations. | U2/U4 scope |

## References

- BRD-003 Authentication & Session Management, FR-AUTH-01…22, BR-AUTH-01…12, AC-AUTH-01…12.
- Existing code: `services/gateway/domain/src/route.rs`, `services/identity/http/src/routes.rs`,
  `services/identity/application/src/sessions.rs`, `web/src/api/client.ts`.
- Existing contracts: `aidlc/spaces/default/knowledge/aidlc-shared/problem-details.md`,
  `aidlc/spaces/default/knowledge/aidlc-shared/service-map.md`.
