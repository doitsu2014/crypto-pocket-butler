# Architecture Decisions — Domain Design

**Intent:** Authentication & Session Management (BRD-003)
**Status:** Proposed — these decisions describe the logical component boundaries accepted in the
Domain Design interview; implementation details and physical schema are deferred to later stages.

---

## ADR-001: Extend the existing Identity and edge components

### Context

The authentication request crosses four existing boundaries: the browser SPA, the public API
gateway, Identity, and Supabase Auth. Identity already owns internal users, jurisdiction gates,
device-session list/revoke, support grants, authorization, account lifecycle and audit outbox
coordination. The gateway already validates Supabase JWTs and forwards them unchanged. The SPA
already owns the sign-in experience. BRD-003 adds TOTP/MFA policy, sign-up gate completion,
recovery, alerts, and session UI; it does not require a new independent service lifecycle.

The project has one schema and one Postgres role per service, no cross-schema grants, synchronous
internal HTTP only, and a no-message-queue rule. A new auth service would therefore add an API and
ownership boundary while still needing Identity for user status and support authorization.

### Decision

Extend the existing `SpaAuthentication`, `ApiGateway`, and `IdentityAuthDomain` components. Keep
Supabase Auth as the external identity authority. Do not create a separate Auth service, a new
Identity sub-service, or separately deployed MFA/recovery/session services in this Domain Design.

### Consequences

**Positive**
- Uses existing user, session, gate, support and audit owners rather than duplicating them.
- Keeps the SPA's single public API path and existing direct Supabase Auth exception.
- Avoids a new internal network hop for each auth-related interaction.

**Negative**
- Identity's application component grows to include MFA policy and recovery-case behavior; later
  stages must keep it cohesive and preserve the existing crate boundaries.
- Identity's current route table is incomplete for the SPA auth calls and must be extended before
  the auth slice is considered complete.

**Alternatives Rejected**
- A separate Auth service: would duplicate or call Identity for user, gate, support and audit state,
  increasing coupling and schema boundaries without an independent lifecycle requirement.
- Separate MFA, Recovery and Session components/services: these behaviors share user ownership,
  support authorization and audit invariants; splitting them now would make one identity lifecycle
  depend on several new boundaries.
- Put auth policy in the SPA: client-side choices cannot enforce trusted-side access or jurisdiction
  policy.

**Alternatives Considered:** separate Identity sub-component, new Auth service, fully split services.

**Source:** Domain Design interview Q1; requirements FR1–FR10; existing Identity and Gateway CodeKB.

---

## ADR-002: Identity owns the app's device-session view and revocation workflow

### Context

BRD-003 requires users to list and revoke devices, and requires a revocation to affect the next
request. Existing Identity code has a `SessionService`, a `DeviceSessionRepository` port, and an
`AuthAdmin` adapter that asks Supabase Auth to revoke a provider session. Supabase Auth is the
provider authority, but the application's current listing needs an Identity-owned device view.
Requirements FR3.8 explicitly accepts a residual token-validity window up to access-token expiry;
this ADR does not change that decision or introduce a server-side deny-list.

### Decision

Identity's `IdentityAuthDomain` owns the application-facing `DeviceSession` entity, the list/revoke
use cases, ownership checks and audit outcome. Supabase Auth remains authoritative for whether the
provider session is valid and receives the provider-revoke call. The operation order is provider
revoke, local state update, then audit; if provider revocation fails, local state remains unchanged.
The SPA presents only the view and initiates the command; the gateway routes it and forwards the
user's JWT unchanged.

### Consequences

**Positive**
- Device list data can include the app's device label, last-seen and current-session marker without
  depending on an external API that does not provide the required list.
- One owner applies the same user/admin authorization and audit rules for list and revoke.
- Provider revocation and the app's local session projection stay explicitly related.

**Negative**
- Identity's view can be stale if provider-side revocation occurs outside the application; the
  unresolved provider-initiated logout question remains for a later decision.
- The accepted residual window remains: an already-issued JWT may pass stateless validation until
  its expiry, up to one hour.

**Alternatives Rejected**
- Supabase Auth owns all device state and Identity only proxies: rejected because the provider does
  not expose the app's required device list in the existing design evidence.
- Gateway owns listing or revocation: rejected because device ownership, provider coordination and
  audit are Identity business rules, not edge routing rules.
- A new Session Management component: rejected by interview Q2; existing Identity use cases and
  ports already own this lifecycle.

**Alternatives Considered:** provider-only session view, gateway-owned revocation, new Session
Management component.

**Source:** Domain Design interview Q2; existing `services/identity/application/src/sessions.rs`;
requirements FR3.8, FR4.2–FR4.7.

---

## ADR-003: Identity owns trusted-side signup-gate evaluation

### Context

The sign-up path requires terms, risk acknowledgement, age confirmation and jurisdiction gating
before account provisioning. The SPA currently has a gate flow but `SignInPage` submits fixed
jurisdiction/age values; the trusted side must evaluate the actual choices. Identity already serves
the signup-gate endpoint and owns jurisdiction policy and user provisioning. The gateway is
 deny-by-default and the team's affirmed completion rule requires every SPA-called path to have a
declared gateway route and owning handler.

### Decision

The SPA collects the user's real choices and submits them through the gateway. Identity owns the
trusted-side evaluation and the `SignupGate` record, and only Identity may authorise provisioning
from the gate result. The gateway declares the route to Identity, applies the auth-specific rate
budget, and forwards the user's token unchanged where the route requires it. A gate authorisation
must not be reusable for a second account (requirements FR6.5; the specific carrier and enforcement
mechanics remain for Contract/Functional Design).

### Consequences

**Positive**
- Consent and jurisdiction are enforced even if a client bypasses the SPA.
- The owner that provisions the internal user also owns the pre-provisioning policy check.
- Route declaration becomes a concrete, testable part of slice completion.

**Negative**
- The current SPA's fixed values must be replaced with user choices.
- Gateway route declarations, rate budgets and handler availability must remain in sync with SPA
  paths.

**Alternatives Rejected**
- SPA-owned gate policy: rejected because the browser is untrusted and can be bypassed.
- A new Compliance service: rejected because the gate is already part of Identity and no independent
  data owner or lifecycle was established.
- Gateway-owned gate records: rejected because the gateway owns routing and token validation, not
  user consent or jurisdiction business state.

**Alternatives Considered:** browser-only validation, separate Compliance service, gateway-owned
record.

**Source:** Domain Design interview Q3; requirements FR6.1–FR6.5; current Identity signup-gate route.

---

## ADR-004: Identity owns recovery cases and coordinates existing support and auth boundaries

### Context

The person chose a support-assisted recovery path when both email and device are lost. Identity
already owns support grants, authorization, user lifecycle, session revocation and audit
coordination. The Support Console is the human interface, not the source of truth for user recovery
state. Recovery must re-prove identity, be audited, and revoke prior sessions; secrets must never be
exposed.

### Decision

Identity owns the `RecoveryCase` lifecycle and coordinates the existing support authorization,
Supabase Auth admin operation, session revocation and Audit Trail. The Support Console presents the
case to an authorized human and invokes Identity operations through the gateway; it does not store
recovery state as its own domain. The exact approving role and second-approver rule remain an open
product/compliance question and are not invented here.

### Consequences

**Positive**
- The recovery state change, session revocation and audit intent can be coordinated by the existing
  identity owner.
- The support console remains a presentation surface and carries no duplicate recovery database.
- A reviewer can trace who requested, verified, approved and completed recovery.

**Negative**
- Identity now has a recovery-case lifecycle that must be tested for timeout, abandoned case,
  duplicate request and partial provider failure in Functional Design.
- The precise authorization/approval policy remains a dependency for later design.

**Alternatives Rejected**
- Support Console owns recovery state: rejected because the console is a separate frontend with no
  schema ownership and must not become an alternate authority for Identity records.
- New Recovery service: rejected because the flow shares Identity's user, support-grant, session and
  audit ownership.
- Recovery as an operation inside Device Sessions without a case entity: rejected because recovery
  has its own re-proof, support action, status and audit lifecycle.

**Alternatives Considered:** console-owned state, new Recovery service, session-only operation.

**Source:** Domain Design interview Q4; requirements FR8.1–FR8.4; current Identity support/session
capabilities.

---

## ADR-005: Identity decides login alerts; Notification delivers them synchronously

### Context

The user chose in-app login alerts. The alert decision depends on authoritative sign-in/device
history, while Notification already owns in-app delivery and persistence. The project forbids message
queues and uses bounded synchronous internal HTTP with idempotency keys. Login alerts may not contain
tokens or full device fingerprints.

### Decision

Identity decides whether a successful sign-in warrants an alert and calls the Notification component
through an inward-defined port. The outbound adapter uses synchronous internal HTTP, a bounded timeout
and retry policy, and an idempotency key so retries do not create duplicate alerts. Notification owns
the `LoginAlert` record and in-app delivery. Alert payloads include only a device label, coarse
location and event time.

### Consequences

**Positive**
- The policy has access to Identity-owned session history and remains out of the client.
- Notification continues to own delivery and storage.
- Idempotent synchronous calls conform to the existing service communication practice.

**Negative**
- Sign-in handling has a dependency on Notification availability; failure handling must not undo a
  valid sign-in and must surface the missed alert for later recovery.
- Unusual-location detection remains undefined and must be settled before implementation.

**Alternatives Rejected**
- Gateway detects new devices: rejected because it does not own authoritative device history or
  identity policy.
- SPA detects and sends alerts: rejected because the client is untrusted and can be bypassed.
- Event bus: rejected because the project forbids message queues.

**Alternatives Considered:** gateway policy, client policy, asynchronous event bus.

**Source:** Domain Design interview Q5; requirements FR7.1–FR7.3; team communication rules.

---

## ADR-006: Identity owns MFA policy; Supabase Auth owns factor operations

### Context

The interview chose TOTP for all users, with AAL2 required for exchange-credential users. Supabase
Auth owns provider factors and challenges. Identity owns internal user status, exchange-credential
knowledge, authorization and the limited-session policy. The gateway and every service validate the
JWT independently. The SPA presents enrolment and step-up screens but is not a policy authority.

### Decision

Identity's `IdentityAuthDomain` owns the `MfaPolicyState` projection and decides when AAL2 is
required, whether a principal is limited, and what protected operations are refused. Supabase Auth
performs TOTP enrolment, verification, recovery-code operations and provider-side factor changes. The
SPA renders those flows and submits provider operations through the intended auth integration; the
gateway and Identity enforce the resulting AAL/policy on protected calls.

### Consequences

**Positive**
- A client cannot bypass the MFA policy by altering a browser flag.
- Provider-specific factor mechanics remain outside the Identity domain model.
- Identity can apply the same policy to SPA, support and internal callers.

**Negative**
- Identity must reliably derive whether exchange credentials exist without cross-schema access; it
  must use its own projection or the owning service's API.
- Factor status from Supabase Auth and the Identity policy projection need a defined refresh and
  failure rule in Functional Design.

**Alternatives Rejected**
- SPA-owned limited-session flag: rejected because client state is not trusted for authorization.
- Gateway owns all MFA policy: rejected because the policy depends on Identity-owned user status and
  exchange-credential context; the gateway remains a thin enforcement/routing boundary.
- A separate MFA service: rejected by interview Q6; it would split factor lifecycle from the user and
  authorization owner.

**Alternatives Considered:** browser policy, gateway-owned policy, separate MFA service.

**Source:** Domain Design interview Q6; requirements FR5.1–FR5.4 and FR3.4.
