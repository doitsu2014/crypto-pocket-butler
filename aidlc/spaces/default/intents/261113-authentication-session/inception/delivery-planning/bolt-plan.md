# Bolt Plan — Authentication & Session Management

A **Bolt** is one build pass over a Unit that ends in a running result. This plan follows the approved
Unit dependency DAG defined in `unit-of-work-dependency.md` and uses the person's selected **risk-first, then value** heuristic. The Unit responsibilities and story ownership below are taken from `unit-of-work.md`. It does not
change the four Unit boundaries or their strict prerequisites. Delivery Planning recommends the
sequence; the Construction walk itself is governed by the approved unit-major, serial checkpoint
settings.

## Bolt 1 — U1: Identity auth domain

**Unit:** `u1-identity-auth-domain` (`service`; extends the existing Identity service).

**Scope:** signup-gate policy; TOTP/AAL2 and limited-session policy; device-session and revocation
use cases; support-assisted recovery case lifecycle; audit intents; Notification port and new-device
alert decision.

**Definition of Done**
- U1 code and tests remain inside Identity's existing Clean Architecture boundaries; outward effects
  use ports, defined inward and implemented outward.
- Sign-up, jurisdiction, MFA/AAL2, session and recovery policy have application tests against fakes.
- Session revocation calls the provider before local mutation and audit; provider failure leaves the
  local session unchanged; the accepted residual window (up to one hour) is explicit.
- Recovery-case outcomes are audited without passwords, factors, codes, tokens or full fingerprints.
- Alert eligibility is decided by Identity and invokes a Notification port with idempotency semantics.
- The session-cap anchor (first sign-in vs last refresh), recovery approver and other unresolved
  policy points are decided and recorded before the affected use case is considered complete.
- Verification: approved checkpoint command; any required integration level that cannot run is
  disclosed with the reason, not omitted silently.

**Confidence hypothesis:** existing Identity owners and ports can implement the policies without a
new service or shared schema, and meaningful rule coverage is possible before HTTP/UI integration.

**Expected demo:** application tests demonstrate gate acceptance/refusal, limited-session policy,
provider-first revocation, failure rollback of local state, and authorized recovery transitions.

**Owner:** `aidlc-developer-agent` (AI; classic scope skips Team Formation).

## Bolt 2 — U2: Identity and gateway boundary

**Unit:** `u2-identity-gateway` (`service`; Identity HTTP/infrastructure plus Gateway route table,
within their existing deployables).

**Scope:** handlers/adapters for the U1 use cases; declared auth/session gateway routes; rate budgets;
provider adapter; admin session access and recovery authorization contracts.

**Definition of Done**
- Every SPA-called path is declared in the deny-by-default gateway table and maps to exactly one
  owning handler; paths with no handler are either implemented or removed from SPA scope.
- User Supabase JWT is forwarded unchanged and validated by Gateway and Identity; privileged
  gateway-to-Identity operations use the existing service-token mechanism separately.
- Authorization, owner/admin checks, and non-disclosure behavior have HTTP tests.
- RFC 7807 problem responses do not echo rejected values, credentials, upstream details or secrets.
- Provider-first session revocation, idempotent replay, and provider-failure behavior are exercised.
- The unresolved verification resend auth policy, admin-session acceptance criteria, recovery
  approver, support recovery endpoints and route names are resolved before contracts/handlers are
  frozen.
- Verification: approved checkpoint command, including boundary checks and relevant HTTP tests.

**Confidence hypothesis:** the existing route table, Identity router and service-token path can
expose the new behavior without a backdoor SPA-to-service call or conflicting identity authority.

**Expected demo:** gateway-level tests show signup-gate, device-session, MFA and recovery paths
reaching Identity; an undeclared path is denied; user and admin authorization are distinct and audited.

**Owner:** `aidlc-developer-agent` (AI), with security/architect review at the Unit checkpoint.

## Bolt 3 — U3: SPA auth and sessions

**Unit:** `u3-spa-auth-sessions` (`ui`; existing `web/` package).

**Scope:** email/password and magic-link sign-in; gates with actual choices; MFA and limited-session
experience; fifth Settings → Sessions destination; revoke confirmations; recovery entry; in-app alert
rendering; outage and error states.

**Definition of Done**
- Every in-scope screen, state and error copy in the approved mockups and interaction contract is
  implemented or the design discrepancy is explicitly dispositioned.
- Tokens remain in memory; only the PKCE verifier may transiently use `sessionStorage` and is removed
  after callback completion.
- SPA product APIs call the Gateway only; magic-link and sign-in calls use Supabase Auth as the
  permitted browser exception.
- Session revocation confirmations name what ends and what remains; residual window is disclosed.
- Limited-session UI gives an explained redirect; server-side authorization remains authoritative.
- Keyboard, focus, screen-reader, contrast and responsive requirements from the accessibility checklist
  are met. The reviewed accent contrast issue must have an explicit disposition and implementation
  treatment before the primary button is considered complete.
- `yarn e2e` passes for auth-surface changes; required frontend checks pass under the approved
  checkpoint command.

**Confidence hypothesis:** current React shell, sign-up-gate components, settings layout and token
system can host the new flows without regressing current sessions or the first-render path.

**Expected demo:** browser journeys exercise password and magic-link sign-in, gates, MFA, the limited
session, sessions management, recovery entry and a provider outage.

**Owner:** `aidlc-developer-agent` (AI), with `aidlc-design-agent` / `aidlc-quality-agent` perspectives
at the Unit checkpoint.

## Bolt 4 — U4: Auth integration proof

**Unit:** `u4-auth-integration-proof` (`spec`; verification/test assets only, no runtime service).

**Scope:** route-completeness assertion, browser journeys, cross-boundary JWT/authorization,
retry/idempotency, audit sanitization, provider outage, alert delivery and recovery proof.

**Definition of Done**
- The complete set of SPA auth paths is declared at the gateway and has one owning handler; the
  route test fails on any extra undeclared call or missing handler.
- Browser journeys cover the approved in-scope flows and failure states, including `yarn e2e`.
- Integration evidence covers provider-first revocation, idempotency, the accepted residual window,
  disabled users, recovery authorization, audit sanitization and service outages.
- Every active acceptance criterion has a test witness or an explicit downstream owner and boundary;
  deferred OAuth/passkey stories remain marked out of scope.
- The approved checkpoint command runs the required gates; output records actual results, and names
  any unavailable level rather than implying it ran.
- The administrator session path, support recovery operations, resend auth policy and all open
  decisions have an approved disposition before U4 is closed.

**Confidence hypothesis:** the user-facing contract is coherent end to end and route-table closure,
security properties and error semantics match the approved artifacts.

**Expected demo:** run auth browser journeys with mocked Supabase/gateway, show route-completeness,
exercise authorization denial, and inspect sanitized audit/alert outcomes.

**Owner:** `aidlc-quality-agent` (AI) for verification; `aidlc-developer-agent` supplies test fixtures.

## Ordered sequence and topology

1. U1 Identity auth domain
2. U2 Identity/gateway boundary; depends on U1
3. U3 SPA auth and sessions; depends on U2
4. U4 integration proof; depends on U1, U2 and U3

This is both the risk-first sequence and the sole topological path in the approved DAG. No walking-
skeleton ceremony is added: the existing Asset Registry skeleton is already built; U4 proves the
authentication extension after its implementation exists.

## Construction settings and verification

- Iteration: `unit-major` — complete one Unit's applicable design stages, Code Generation and checkpoint
  before moving to the next Unit.
- Execution: `serial`; the DAG contains no independent Units for parallel work.
- Checkpoints: enabled; human approval remains required at each Unit checkpoint.
- Verification command: use the exact human-approved command recorded in the workflow checkpoint
  setting. It runs Rust formatting, Clippy, boundary checks, Cargo tests and coverage with the
  disposable Postgres database, then frontend tests/lint/build/format, auth-surface Playwright journeys
  and size checks in both packages as applicable. If the environment cannot run a required level, the
  checkpoint must name the omitted test and why.
