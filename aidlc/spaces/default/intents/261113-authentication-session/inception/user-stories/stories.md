# User Stories — Authentication & Session Management

**Intent:** implement BRD-003. **Source:** `inception/requirements-analysis/requirements.md`,
shaped by the six interview answers in `user-stories-questions.md`.

**Format:** each story is `As a [persona], I want [action], so that [benefit]`, with INVEST notes, a
MoSCoW priority taken from its BRD row, and Given/When/Then criteria including at least one error or
edge path. Personas are defined in `personas.md` (P1 Maya, P2 Maya lost access, P3 Sam support,
P4 Dana admin).

**IDs:** `US{n}.{m}` per area; criteria `AC{n}.{m}.{k}`.

**Reachability:** per interview Q4, every story that adds or changes a path the SPA calls carries a
criterion that the path is declared in the gateway's deny-by-default route table and served by an
owning handler. `US8.1` covers the paths that have no handler at all today.

---

## Area 1 — Signing in

### US1.1 Sign in with email and password

**As P1 Maya, I want to sign in with my email address and password, so that I reach my portfolio
without creating anything new.**

- **Priority:** Must Have (BRD FR-AUTH-03)
- **Requirements:** FR1.1, FR1.4
- **INVEST:** independent of other sign-in methods; valuable on its own; small; testable through the
  sign-in page and the session store.
- **Dependencies:** none; it is the first slice.

**Acceptance criteria**

- AC1.1.1 — Given valid credentials, when I submit the sign-in form, then a session is established
  from Supabase Auth and the SPA lands on the dashboard.
- AC1.1.2 — Given an invalid password, when I submit, then I stay on the sign-in page, see a
  non-revealing error that does not distinguish "no such account" from "wrong password", and the
  password field is cleared.
- AC1.1.3 — Given I submit, when the request is in flight, then the submit control is disabled so a
  double submit cannot create a second session.
- AC1.1.4 — Given the password I typed, when I submit, then the password is sent only to Supabase
  Auth and never to the gateway or any service.
- AC1.1.5 — Given a successful sign-in, when the dashboard loads, then the session token is held in
  memory and no token appears in `localStorage`, `sessionStorage`, IndexedDB, or the URL.

### US1.2 Sign in with a magic link

**As P1 Maya, I want to sign in by clicking a link emailed to me, so that I do not have to remember
a password on a device I do not trust.**

- **Priority:** Must Have (BRD FR-AUTH-03 in-scope list; interview Q1)
- **Requirements:** FR1.2, FR1.3, FR2.5
- **INVEST:** independently deliverable after US1.1; valuable; testable with the mocked provider and
  a browser journey.

**Acceptance criteria**

- AC1.2.1 — Given I enter my email and choose "send a sign-in link", when the provider accepts it,
  then I see a neutral confirmation that does not reveal whether an account exists.
- AC1.2.2 — Given I open the emailed link in the same browser, when the callback completes, then the
  session is established and I land on the dashboard.
- AC1.2.3 — Given the callback redirect, when the code exchange happens, then the PKCE verifier
  stored for the redirect is used and removed once the callback completes.
- AC1.2.4 — Given an expired or already-used link, when I open it, then I see a plain message that
  the link is no longer valid with a way to request a new one, and no session is created.
- AC1.2.5 — Given I request several links in a row, when I exceed the per-address rate limit, then
  the request is refused with a retry hint and no email is sent.

### US1.3 Protected pages send me to sign in and bring me back

**As P1 Maya, I want a protected page to take me to sign-in and then return me where I was going, so
that a session expiry does not make me lose my place.**

- **Priority:** Must Have (BRD FR-AUTH-01)
- **Requirements:** FR1.5
- **INVEST:** independent; small; testable as a routing rule.

**Acceptance criteria**

- AC1.3.1 — Given I am not signed in, when I request a protected page, then I am redirected to
  sign-in and the intended destination is remembered.
- AC1.3.2 — Given I signed in from that redirect, when the session is established, then I land on
  the page I originally requested, not the dashboard.
- AC1.3.3 — Given I request the remembered destination directly, when I am not signed in, then the
  destination is not treated as trusted input: only a same-application path is honoured, and an
  external or malformed destination falls back to the dashboard.
- AC1.3.4 — Given I am signed in, when I request a protected page, then no redirect happens.

---

## Area 2 — Session lifecycle

### US2.1 My session refreshes without me noticing

**As P1 Maya, I want my access token renewed in the background, so that a long working session never
interrupts me with a sign-in prompt.**

- **Priority:** Must Have (BRD FR-AUTH-06)
- **Requirements:** FR2.1, FR2.2
- **INVEST:** independent of the UI beyond the session store; testable by clock control.

**Acceptance criteria**

- AC2.1.1 — Given a valid refresh token and an access token that has expired, when the next API call
  is made, then the token is refreshed first and the call succeeds with no visible prompt.
- AC2.1.2 — Given a refresh occurs, when the new access token is issued, then the refresh token is
  rotated and the previous refresh token can no longer be used.
- AC2.1.3 — Given a session older than 30 days measured from first sign-in, when it tries to refresh,
  then the refresh is refused and I am sent to sign in (BRD OQ-001 resolved: 1 h access, 7 d refresh,
  30 d absolute cap from first sign-in).
- AC2.1.4 — Given two tabs sharing the session, when both refresh near-simultaneously, then neither
  tab ends up with a token the provider has already rotated away, and I am not signed out.
- AC2.1.5 — Given the refresh endpoint is slow, when the request exceeds its timeout, then the call
  that needed it fails as recoverable rather than leaving a half-updated session.

### US2.2 A dead refresh token sends me back to sign in

**As P1 Maya, I want a clear return to sign-in when my session can no longer be renewed, so that I am
never left on a screen that silently fails.**

- **Priority:** Must Have (BRD FR-AUTH-07)
- **Requirements:** FR2.3
- **INVEST:** small; testable as a state transition.

**Acceptance criteria**

- AC2.2.1 — Given an expired or revoked refresh token, when a refresh is attempted, then the session
  is cleared and I am redirected to sign-in.
- AC2.2.2 — Given I am redirected, when the sign-in page loads, then a plain message explains that I
  need to sign in again and no previous page content is left visible.
- AC2.2.3 — Given several parallel requests all discover the dead token, when they fail, then the
  user sees exactly one redirect and one message, not a burst of them.

### US2.3 My tokens never touch browser storage

**As P1 Maya, I want my session tokens kept out of anything a script or a later visitor could read,
so that leaving a device unattended does not expose my holdings.**

- **Priority:** Must Have (BRD FR-AUTH-21)
- **Requirements:** FR2.4, FR2.5
- **INVEST:** testable mechanically; small; high value.

**Acceptance criteria**

- AC2.3.1 — Given I have signed in, when any code writes to `localStorage`, `sessionStorage`, or
  IndexedDB, then no access token, refresh token, or provider JWT is present in the write.
- AC2.3.2 — Given a redirect callback is in progress, when the verifier is persisted, then the only
  persisted value is the PKCE code verifier, and it is removed when the callback completes.
- AC2.3.3 — Given the app is reloaded, when it starts, then it holds no session from storage and
  behaves as signed out until the provider session is re-established.
- AC2.3.4 — Given a test scans the built bundle and the runtime storage, when a token-shaped string
  is written anywhere persistent, then the test fails.

### US2.4 Signing out ends the session everywhere it exists

**As P1 Maya, I want sign-out to end both my app session and my provider session, so that a shared
computer cannot resurrect my account with a reload.**

- **Priority:** Must Have (BRD FR-AUTH-08)
- **Requirements:** FR4.1
- **INVEST:** small; testable; independent.

**Acceptance criteria**

- AC2.4.1 — Given I am signed in, when I sign out, then the in-memory session is cleared and the
  provider session is terminated.
- AC2.4.2 — Given I signed out, when I reload the page, then I am signed out and no API call is made
  with a stale token.
- AC2.4.3 — Given the provider sign-out call fails, when the app session is cleared anyway, then I am
  still signed out locally and the failure is surfaced rather than swallowed.
- AC2.4.4 — Given I sign out, when the audit trail is read, then a sign-out event names the actor,
  the action, the target session, the time, and the outcome.

---

## Area 3 — MFA and the limited session

### US3.1 Enrol a TOTP authenticator

**As P1 Maya, I want to add an authenticator app to my account, so that a stolen password is not
enough to reach my portfolio.**

- **Priority:** Must Have (BRD FR-AUTH-12)
- **Requirements:** FR5.1, FR5.4
- **INVEST:** deliverable on its own; valuable; testable with a fixed clock and a known secret.

**Acceptance criteria**

- AC3.1.1 — Given I start enrolment, when the provider issues a secret, then I see a QR code and a
  manual key, and the secret is shown once and never again.
- AC3.1.2 — Given I enter a correct six-digit code, when I confirm, then the factor is verified and
  my session is raised to the multi-factor level.
- AC3.1.3 — Given I enter a wrong or expired code, when I confirm, then enrolment is refused, the
  factor is not enabled, and I can retry without restarting from the secret.
- AC3.1.4 — Given enrolment succeeds, when it completes, then I receive one-time recovery codes,
  shown once, and an audit event records the enrolment without the secret or any code.
- AC3.1.5 — Given I have enrolled, when I view my security settings, then I can see that a factor is
  active and when it was added.

### US3.2 A user who needs MFA but has not enrolled is held in a limited session

**As P1 Maya, I want a clear, single-purpose screen when my account requires a second factor I have
not set up, so that I know exactly what to do and my portfolio stays inaccessible until I do it.**

- **Priority:** Must Have (BRD FR-AUTH-12, BR-AUTH-09)
- **Requirements:** FR5.2
- **INVEST:** one policy expressed end to end; testable at three layers.

**Acceptance criteria**

- AC3.2.1 — Given I hold an exchange credential and my session is single-factor, when I sign in, then
  I am placed in a limited session that can reach only the MFA and settings screens.
- AC3.2.2 — Given I am in a limited session, when I request any other SPA route, then I am redirected
  to the MFA screen rather than shown the page.
- AC3.2.3 — Given I am in a limited session, when I call a portfolio endpoint through the gateway,
  then the request is refused with `403` and a problem body that names the missing factor.
- AC3.2.4 — Given I complete MFA enrolment from the limited session, when the factor is verified, then
  the limited restriction lifts on the next request without a new sign-in.
- AC3.2.5 — Given a user with no exchange credential, when they sign in without MFA, then they are
  not placed in a limited session.

### US3.3 A sensitive change asks me to prove the second factor again

**As P1 Maya, I want to confirm my second factor before changing my password, MFA method, or recovery
codes, so that an unattended open session cannot be turned into a permanent takeover.**

- **Priority:** Must Have (BRD BR-AUTH-09)
- **Requirements:** FR5.3, FR4.6, FR5.4
- **INVEST:** one guard on three operations; testable per operation.

**Acceptance criteria**

- AC3.3.1 — Given I request a password, MFA-method, or recovery-code change, when the request starts,
  then I must supply a current second-factor code before the change is accepted.
- AC3.3.2 — Given I supply a wrong or expired code, when I submit, then the change is refused and
  nothing is altered.
- AC3.3.3 — Given the change succeeds, when it completes, then every other session for my account is
  revoked and I remain signed in only on the session that made the change.
- AC3.3.4 — Given the change succeeds, when the audit trail is read, then the event names the change
  and the revocation it caused, and contains no code and no secret.

---

## Area 4 — Sign-up gates and email verification

### US4.1 I accept the terms, the risk disclosure, and my age before my account exists

**As P1 Maya, I want to read and accept the terms, acknowledge the risk, and confirm I am old enough
before anything is created, so that I start from a clear, recorded consent.**

- **Priority:** Must Have (BRD FR-AUTH-19, BR-AUTH-12)
- **Requirements:** FR6.1, FR6.3, FR6.4
- **INVEST:** one gate chain; testable at the browser and the API.

**Acceptance criteria**

- AC4.1.1 — Given I open sign-up, when the form renders, then terms acceptance, risk acknowledgement,
  and age confirmation are separate controls with no value pre-selected.
- AC4.1.2 — Given I leave any of the three unanswered, when I submit, then submission is blocked and
  each unanswered control is named.
- AC4.1.3 — Given I answer all three, when I submit, then my actual choices — not fixed values — are
  sent to the signup gate and a gate authorisation is returned.
- AC4.1.4 — Given a client submits a sign-up without a valid gate authorisation, when the request
  reaches the service, then no account is created and the refusal names the missing gate.
- AC4.1.5 — Given a gate authorisation has already been used for one sign-up, when it is presented
  again, then the second sign-up is refused and no second account is created.

### US4.2 A restricted jurisdiction cannot create an account

**As P1 Maya in a supported country, I want the product to check where I am before creating my
account, so that the service I get is one it is allowed to offer me.**

- **Priority:** Must Have (BRD FR-AUTH-20)
- **Requirements:** FR6.2, FR6.3
- **INVEST:** independent of the terms gate (different data, different refusal); testable.

**Acceptance criteria**

- AC4.2.1 — Given my detected jurisdiction is not on the launch list, when I attempt sign-up, then no
  account is created and I am told plainly that the service is not available in my region.
- AC4.2.2 — Given my jurisdiction is supported, when I sign up, then the check passes and my
  jurisdiction is recorded with the account.
- AC4.2.3 — Given the jurisdiction cannot be determined, when I sign up, then the outcome is one
  defined behaviour — refusal, with a message that does not imply I did something wrong.
- AC4.2.4 — Given I bypass the SPA and call the service directly, when I omit or forge a jurisdiction,
  then the trusted-side check still runs and refuses as it would have in the browser.

### US4.3 I verify my email before I can add an exchange credential

**As P1 Maya, I want to prove I own my email address before connecting an exchange, so that a typo in
my address cannot become a permanent lockout from my own data.**

- **Priority:** Must Have (BRD FR-AUTH-17)
- **Requirements:** FR6.6, FR6.7
- **INVEST:** independent of the gates; testable with a mocked verification state.

**Acceptance criteria**

- AC4.3.1 — Given my email is unverified, when I try to add an exchange credential, then the action is
  refused and I am offered a way to verify.
- AC4.3.2 — Given I request a verification email, when I have not exceeded the rate limit, then one is
  sent and the response does not reveal whether an account exists for that address.
- AC4.3.3 — Given I have exceeded the rate limit, when I request another, then the request is refused
  with a retry hint and no email is sent.
- AC4.3.4 — Given a client claims the email is verified, when the service has no verification record,
  then the credential is still refused: the state is read from the trusted side, not the client.
- AC4.3.5 — Given my email becomes verified, when I return to add the credential, then the action is
  allowed without a new sign-in.

---

## Area 5 — Devices and sessions

### US5.1 I can see every device that is signed in

**As P2 Maya, suspicious of my account, I want to see all my active sessions with when each was last
used, so that I can tell whether someone else is in my account.**

- **Priority:** Must Have (BRD FR-AUTH-15)
- **Requirements:** FR4.2, FR4.7
- **INVEST:** read-only first, then the actions follow; testable; small.

**Acceptance criteria**

- AC5.1.1 — Given I have sessions on several devices, when I open the sessions screen, then each row
  shows a device label, a last-seen time, and whether it is the session I am using now.
- AC5.1.2 — Given I have exactly one session, when I open the screen, then the wording is singular and
  the current session is marked.
- AC5.1.3 — Given I have no other sessions, when I open the screen, then the empty state says so and
  offers nothing destructive.
- AC5.1.4 — Given another user's sessions exist, when I request my list, then only my own rows are
  returned; a request for another user's list is refused without revealing whether that user exists.
- AC5.1.5 — Given the list is requested through the SPA, when the browser calls it, then the path is
  declared in the gateway route table and served by an owning handler (interview Q4).

### US5.2 I can revoke one device

**As P2 Maya, I want to end a specific session I do not recognise, so that whoever is using it loses
access at once.**

- **Priority:** Must Have (BRD FR-AUTH-15)
- **Requirements:** FR4.3, FR4.5, FR3.8
- **INVEST:** independent of revoke-all; testable at the use case and over HTTP.

**Acceptance criteria**

- AC5.2.1 — Given I choose a session that is not the current one, when I revoke it, then it is revoked
  at the provider, marked revoked locally, and an audit event records the action.
- AC5.2.2 — Given I choose the session I am using, when I revoke it, then I must confirm explicitly
  first; cancel leaves it active.
- AC5.2.3 — Given I confirm revoking my current session, when it completes, then I am signed out and
  returned to sign-in.
- AC5.2.4 — Given the provider refuses the revocation, when it fails, then the local row is left
  unchanged, the failure is surfaced as recoverable, and no audit event claims success.
- AC5.2.5 — Given a session was already revoked, when it is revoked again, then the result is success
  without calling the provider a second time.
- AC5.2.6 — Given a revocation succeeds, when the revoked device makes its next request with its
  still-unexpired token, then that request is refused while the SPA holds that token — and the
  documented residual window, up to the access token's hour, is stated in the screen's help text
  rather than left implicit (interview Q5, requirements FR3.8).

### US5.3 I can end every other session at once

**As P2 Maya, certain my account is compromised, I want one action that ends all other sessions, so
that I do not have to inspect a list while I am worried.**

- **Priority:** Must Have (BRD FR-AUTH-15)
- **Requirements:** FR4.4, FR4.5
- **INVEST:** independent; testable; small.

**Acceptance criteria**

- AC5.3.1 — Given I have several sessions, when I choose "sign out everywhere else", then every session
  except the current one is revoked and the confirmation names how many were ended.
- AC5.3.2 — Given the bulk action partially fails, when it completes, then I am told which sessions
  remain and can retry, and the result is never reported as complete.
- AC5.3.3 — Given the action completes, when the audit trail is read, then each ended session has its
  own record rather than one aggregate claim.

### US5.4 Changing a credential ends my other sessions

**As P4 Dana, I want a credential change to end the user's other sessions automatically, so that a
stolen session cannot outlive the fix.**

- **Priority:** Must Have (BRD FR-AUTH-22)
- **Requirements:** FR4.6
- **INVEST:** one rule, testable; depends on US3.3 for the step-up.

**Acceptance criteria**

- AC5.4.1 — Given a user changes their password, MFA method, or passkey, when the change succeeds, then
  every other session for that user is revoked.
- AC5.4.2 — Given the revocations partially fail, when the change has succeeded, then the remaining
  sessions are reported and retried rather than silently left live.
- AC5.4.3 — Given the change was made from one session, when it completes, then that session stays
  signed in and the others do not.

---

## Area 6 — Login alerts

### US6.1 I am told about a sign-in from somewhere new

**As P2 Maya, I want an alert when my account is used from a device I have not used before, so that I
find out from the product rather than from my portfolio balance.**

- **Priority:** Should Have (BRD FR-AUTH-16)
- **Requirements:** FR7.1, FR7.2, FR7.3
- **INVEST:** independently deliverable; testable with a known device history.

**Acceptance criteria**

- AC6.1.1 — Given I sign in from a device I have not used before, when the sign-in succeeds, then an
  in-app notification is created for me.
- AC6.1.2 — Given I sign in again from the same device, when the sign-in succeeds, then no new alert is
  created for that device.
- AC6.1.3 — Given an alert exists, when I read it, then it names the device label and the time, contains
  no token, no credential, and no full device fingerprint, and offers a link to review my sessions.
- AC6.1.4 — Given the alert cannot be created, when the sign-in itself succeeded, then the sign-in still
  succeeds and the alert failure is surfaced rather than rolling back the session.
- AC6.1.5 — Given I have no unread alerts, when I open the notification area, then the empty state says
  so rather than showing a placeholder count.

---

## Area 7 — Account recovery

### US7.1 I can start recovery when I have lost both my email and my device

**As P2 Maya, locked out of both my email account and my device, I want a route back that does not
depend on either, so that I can recover my account at all.**

- **Priority:** Must Have (BRD FR-AUTH-18, OQ-005 resolved)
- **Requirements:** FR8.1, FR8.4
- **INVEST:** independent of the support console work; testable at the service boundary.

**Acceptance criteria**

- AC7.1.1 — Given I cannot use my email or device, when I request recovery, then a recovery case is
  opened for a support agent and I am told the next step without being asked for a secret.
- AC7.1.2 — Given a recovery case is open, when it is created, then it records who requested it, when,
  and the reason, and requires an authorised support agent to continue.
- AC7.1.3 — Given identity re-proof fails, when the case is closed, then the account is unchanged, no
  session is issued, and the message to me names no failed check.
- AC7.1.4 — Given a support agent is not authorised, when they attempt to continue a case, then the
  action is refused and the refusal is audited.

### US7.2 A completed recovery gives me a session and ends the old ones

**As P2 Maya, once support has re-proved who I am, I want a working session and every old session
gone, so that whoever else had access is out.**

- **Priority:** Must Have (BRD FR-AUTH-18)
- **Requirements:** FR8.2, FR8.3
- **INVEST:** depends on US7.1; testable end to end with a fake provider.

**Acceptance criteria**

- AC7.2.1 — Given a recovery case has been re-proved, when it completes, then I receive a way to set a
  new session credential and no new session exists before that step.
- AC7.2.2 — Given recovery completes, when it finishes, then every pre-existing session for my account
  is revoked.
- AC7.2.3 — Given the recovery flow runs, when the application handles it, then no password, provider
  secret, or factor seed is ever seen by the application.
- AC7.2.4 — Given recovery completes, when the audit trail is read, then the whole path — request,
  re-proof, completion, and every revocation — is reconstructible from records with no secret in them.

### US7.3 Recovery never tells an attacker whether it worked

**As P3 Sam, the support agent, I want the recovery flow to give an outsider no signal about its
progress, so that the path cannot be used to confirm an account exists or to probe a user's identity.**

- **Priority:** Must Have (BRD FR-AUTH-18, FR-AUTH-11)
- **Requirements:** FR8.4, FR3.7
- **INVEST:** one property across several endpoints; testable as response assertions.

**Acceptance criteria**

- AC7.3.1 — Given I request recovery for an address that has no account, when I submit, then the
  response is indistinguishable from one for an address that does have an account.
- AC7.3.2 — Given a recovery attempt fails, when the response is returned, then it names no internal
  check, no support action, and no account detail.
- AC7.3.3 — Given repeated failed attempts, when the rate limit is reached, then the refusal is the same
  shape as any other refusal, and the attempt is still recorded for the audit trail.

---

## Area 8 — Reachability through the edge

### US8.1 The auth paths the app calls are declared at the gateway

**As P1 Maya, I want every sign-in and session action I take to work through the product's single entry
point, so that the feature is actually usable rather than silently refused.**

- **Priority:** Must Have (team practice; requirements FR9.1–FR9.3)
- **Requirements:** FR9.1, FR9.2, FR9.3
- **INVEST:** one coherent change to the route table plus handlers; testable per route.

**Acceptance criteria**

- AC8.1.1 — Given the SPA calls the signup gate, when the request arrives at the gateway, then the route
  is declared, forwarded to Identity, and answered — not refused as undeclared.
- AC8.1.2 — Given the SPA calls the device-session list or revoke, when the request arrives, then each is
  declared and forwarded to Identity with the user's token unchanged.
- AC8.1.3 — Given the SPA calls the MFA enrolment or recovery path, when the request arrives, then either
  an owning handler exists and the route is declared, or the SPA call is removed — no third state.
- AC8.1.4 — Given the auth routes are declared, when their rate budgets are read, then each is stricter
  than an ordinary read and the value is stated.
- AC8.1.5 — Given a route is not declared, when any caller requests it, then it is refused, and a test
  asserts that the set of SPA-called paths is a subset of the declared table.

---

## Area 9 — Token trust and outage behaviour

### US9.1 Every service decides for itself that my token is good

**As P4 Dana, I want each service to validate the token itself, so that a compromised edge cannot be
the only check between an attacker and a user's data.**

- **Priority:** Must Have (BRD FR-AUTH-04, FR-AUTH-09)
- **Requirements:** FR3.1, FR3.2, FR3.5, FR3.6
- **INVEST:** one validation contract applied everywhere; testable per service and per claim.

**Acceptance criteria**

- AC9.1.1 — Given a token signed by a key not in the configured JWKS, when a protected request arrives,
  then it is refused with `401` and no data is returned.
- AC9.1.2 — Given a token with the wrong issuer or audience, when it arrives, then it is refused,
  including when the signature itself is valid.
- AC9.1.3 — Given an expired token, when it arrives, then it is refused; given a token within the
  documented clock-skew allowance, then it is accepted, and that allowance has one stated value applied
  at the gateway and every service.
- AC9.1.4 — Given a protected route, when the request carries no bearer token, then it is refused, and
  the only routes that accept an unauthenticated request are health, readiness, metrics, public docs,
  and the auth callbacks.
- AC9.1.5 — Given the JWT signing keys change, when the cache refreshes, then new tokens validate without
  a restart, and keys staler than the stated maximum cause a refusal rather than an unverified accept.
- AC9.1.6 — Given a refusal, when the problem body is read, then it names no expected issuer, audience,
  or key material.

### US9.2 A disabled user is refused even with a valid token

**As P4 Dana, I want disabling an account to take effect on that user's next request, so that turning
an account off actually turns it off.**

- **Priority:** Must Have (BRD BR-AUTH-05)
- **Requirements:** FR3.3, FR3.4
- **INVEST:** independent; testable; small.

**Acceptance criteria**

- AC9.2.1 — Given I disable a user, when their unexpired token is used on the next request, then access
  is denied.
- AC9.2.2 — Given a user has never been seen before, when they make their first authenticated request,
  then an internal user is created for them exactly once, even if two requests arrive together.
- AC9.2.3 — Given a disabled user, when their history is read, then their prior records are intact and
  the status change is itself recorded.

### US9.3 A provider outage degrades auth and nothing else

**As P1 Maya, mid-session during an identity-provider outage, I want the product to keep working until
my token runs out and then tell me plainly that sign-in is unavailable, so that an outage elsewhere
does not look like a broken product.**

- **Priority:** Must Have (BRD FR-AUTH-11, OQ-003 resolved)
- **Requirements:** FR3.7, FR10.1, FR10.2, FR10.3
- **INVEST:** one behaviour across three entry points; testable with the provider forced to fail.

**Acceptance criteria**

- AC9.3.1 — Given the provider is unavailable and I am already signed in, when I use the product, then my
  requests succeed until my access token expires, with no fallback authentication path added.
- AC9.3.2 — Given the provider is unavailable, when I try to sign in, sign up, refresh, or answer an MFA
  challenge, then I get a plain "sign-in is unavailable" message that names no upstream detail.
- AC9.3.3 — Given the provider is failing, when my client retries, then it backs off rather than retrying
  in a loop that amplifies the outage.
- AC9.3.4 — Given the provider recovers, when I try again, then it succeeds with no operator action and
  no sign-out beyond what expiry already required.
- AC9.3.5 — Given the outage affects Identity's token validator, when a protected request arrives, then
  the refusal is a service-unavailable problem whose detail reveals nothing about the upstream.

---

## Deferred — recorded so the decision is visible

### US10.1 Sign in with Google or Apple

**As P1 Maya, I want to sign in with an account I already have, so that I do not create another
password.**

- **Priority:** Won't Have (this intent). BRD FR-AUTH-13 is a *Should*; interview Q1 deferred it to a
  follow-up intent so this intent's surface stays to the two methods it can finish properly.
- **Requirements:** none covered in this intent; the BRD row is recorded as deferred in
  `traceability.json`.
- **Note:** the sign-in page must not show a provider control that is not wired (requirements assumption
  A4).

### US10.2 Sign in with a passkey

**As P1 Maya, I want to sign in with a passkey on my device, so that there is no shared secret to steal.**

- **Priority:** Won't Have (this intent). BRD FR-AUTH-14 is a *Should*; deferred by interview Q1, with
  the consequence that TOTP stays the only second factor and passkeys are not offered as a first factor.
- **Requirements:** none covered in this intent; recorded as deferred.
- **Note:** FR-AUTH-22 names a passkey change as a revocation trigger. Until passkeys exist, that clause
  is satisfied for password and MFA changes only (see US5.4).

---

## Story map

Rows are priority; columns are the journey.

| | Sign in | Stay in | Secure | Sign up | Manage | Recover |
|---|---|---|---|---|---|---|
| **Must** | US1.1 US1.2 US1.3 | US2.1 US2.2 US2.3 US2.4 | US3.1 US3.2 US3.3 US9.1 US9.2 US9.3 | US4.1 US4.2 US4.3 | US5.1 US5.2 US5.3 US5.4 US8.1 | US7.1 US7.2 US7.3 |
| **Should** | | | US6.1 | | | |
| **Won't (here)** | US10.1 US10.2 | | | | | |

<!-- Text fallback: Must-have stories cover sign-in (US1.1–US1.3), staying signed in (US2.1–US2.4), security (US3.1–US3.3, US9.1–US9.3), sign-up (US4.1–US4.3), managing sessions (US5.1–US5.4, US8.1) and recovery (US7.1–US7.3). One Should-have story covers login alerts (US6.1). Two Won't-have stories record the deferred OAuth and passkey methods. -->

## Dependencies

- **US8.1 before the browser journeys for US4.1, US4.2, US5.1–US5.3, US3.1** — the SPA cannot reach
  those paths through the gateway until the routes are declared. The stories are written to be
  independently testable at the service layer, but their end-to-end criteria need US8.1.
- **US3.1 before US3.2** — the limited session lifts only when enrolment can complete.
- **US2.3 before US2.4** — sign-out assumes tokens live only in memory.
- **US7.1 before US7.2 and US7.3** — the recovery path is defined by its entry.
- **US9.1 before everything with a protected route**, because every other criterion assumes the
  token was validated.

## Coverage notes

- Every BRD functional requirement is covered except FR-AUTH-13 and FR-AUTH-14, deferred by
  interview Q1 and recorded as Won't Have.
- Every BRD business rule maps to at least one criterion; BR-AUTH-06 (HTTP-only secure cookies) is
  satisfied structurally by FR2.4/US2.3 — no script-readable token cookie is issued at all.
- The accepted revocation window (requirements FR3.8, interview Q5) is an explicit criterion in
  US5.2.6 so it is tested rather than assumed.
