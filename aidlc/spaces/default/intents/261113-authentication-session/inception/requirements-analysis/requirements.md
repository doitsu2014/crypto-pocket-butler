# Requirements — Authentication & Session Management

**Intent:** start implementing `docs/brds/BRD-003-authentication-and-session-management.md`
**Source of truth for the requirement set:** BRD-003 v1.1, re-shaped by this intent's interview
(Q1–Q8, recorded in `requirements-analysis-questions.md`). Every requirement below traces to a
BRD id; nothing is invented. Where the interview closed a BRD open question, the answer is cited
and the BRD's open question is marked resolved.

**Context read:** the code knowledge base for `crypto-pocket-butler`
(`business-overview.md`, `architecture.md`, `code-structure.md`), and the team practices affirmed
at this intent's Practices Discovery (`team-practices.md`). The knowledge base records that the
SPA already uses Supabase Auth with PKCE and an in-memory token store, the gateway validates JWTs
against cached JWKS, and Identity already exposes device-session list and revoke — so most of this
work finishes and proves an existing path rather than starting one.

---

## Intent analysis

A person must be able to sign in, stay signed in safely, see and control every device that is
signed in, satisfy the compliance gates that precede account creation, and recover access without
exposing secrets. The API must trust a request only on a token it has validated itself.

Three goals sit behind that:

1. **Close the gap between what the SPA calls and what the edge declares.** Today the SPA calls
   `/signup-gate`, `/mfa/enrol` and `/mfa/recovery`, and Identity serves the signup gate and the
   device-session routes, but none of those paths is in the gateway's deny-by-default table. They
   are unreachable through the edge. Under the team's affirmed practice, a slice is complete only
   when it is reachable through declared gateway routes.
2. **Make MFA real for everyone and mandatory for the exchange-credential case.**
3. **Settle the security policy the BRD left open** — token lifetimes, the revocation window, the
   outage posture, the recovery path, and the launch-market identity question — so construction
   does not have to guess.

---

## Functional requirements

### FR1 — Sign-in and session establishment

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR1.1 | The SPA shall sign a user in with email address and password through Supabase Auth, and shall never send a password to any service of its own. | FR-AUTH-03, BR-AUTH-01 | Must |
| FR1.2 | The SPA shall sign a user in without a password through a Supabase Auth magic-link email whose callback completes the session in the SPA. | FR-AUTH-03 (in-scope list), BR-AUTH-01 | Must |
| FR1.3 | Every OAuth-style redirect flow the SPA uses shall use the PKCE flow; the implicit flow shall not be configured or reachable. | FR-AUTH-02 | Must |
| FR1.4 | A successful sign-in shall establish the session and land the user on the dashboard. | FR-AUTH-03, AC-AUTH-02 | Must |
| FR1.5 | An unauthenticated visitor requesting a protected page shall be redirected to the sign-in page, and returned to the requested page after signing in. | FR-AUTH-01, AC-AUTH-01 | Must |

### FR2 — Token lifecycle and silent refresh

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR2.1 | Production access tokens shall expire after 1 hour; refresh tokens shall expire after 7 days with rotation on use; and no session shall exceed an absolute cap of 30 days without a fresh sign-in. | BRD OQ-001, **resolved by interview Q3**; BR-AUTH-03 | Must |
| FR2.2 | An expired access token shall refresh transparently while a valid refresh token exists, with no re-sign-in and no user-visible interruption. | FR-AUTH-06, AC-AUTH-03 | Must |
| FR2.3 | An expired, revoked, or invalid refresh token shall end the session and redirect the user to sign-in. | FR-AUTH-07, AC-AUTH-04 | Must |
| FR2.4 | Access and refresh tokens shall be held in browser memory only and never written to `localStorage`, `sessionStorage`, IndexedDB, cookies readable by script, or a URL. | FR-AUTH-21, BR-AUTH-10 (NFR-SEC-10) | Must |
| FR2.5 | The PKCE code verifier may be persisted in `sessionStorage` for the duration of a redirect callback and shall be removed once the callback completes; no other value may use that exception. | FR-AUTH-02, FR-AUTH-21 | Must |

### FR3 — API trust, provisioning, and revocation effect

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR3.1 | The gateway and every service shall validate the token's signature against the configured Supabase JWKS, its issuer, its audience, and its expiry on every protected request, and shall trust no other issuer or audience. | FR-AUTH-04, BR-AUTH-08 | Must |
| FR3.2 | Every API route except health, readiness, metrics, public documentation, and the auth callbacks shall require a valid bearer token. | BR-AUTH-02 | Must |
| FR3.3 | The token subject shall map to an internal user, created lazily at the first authenticated request, idempotently. | FR-AUTH-05, BR-AUTH-04 | Must |
| FR3.4 | A disabled internal user shall be denied access even with a valid provider token. | BR-AUTH-05, AC-AUTH-07 | Must |
| FR3.5 | A missing, malformed, expired, or wrongly-scoped token shall be refused with `401` and a problem body that names no expected audience, issuer, or key material. | FR-AUTH-09, AC-AUTH-05 | Must |
| FR3.6 | The configured clock-skew allowance shall be documented and applied consistently; validation shall not fail on an otherwise valid token within that allowance. | BRD-003 Notes | Must |
| FR3.7 | When Supabase Auth is unavailable, a sign-in attempt shall fail with a clear, non-revealing message that names no upstream detail, endpoint, or credential. | FR-AUTH-11 | Must |
| FR3.8 | A session revocation shall take effect for the affected session on its next request where that request depends on the SPA's held token, by the SPA discarding the token on revocation and by Identity refusing the principal; the residual window until the current access token expires is accepted and documented as a known limit. | BR-AUTH-10, **amended by interview Q4** | Must |

### FR4 — Sign-out and device/session management

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR4.1 | Sign-out shall destroy the application session in the browser **and** terminate the identity-provider session, in that order, so a reload cannot resurrect either. | FR-AUTH-08, AC-AUTH-06 | Must |
| FR4.2 | The user shall be able to list their active devices and sessions, each showing a device label, last-seen time, and whether it is the current session. | FR-AUTH-15 | Must |
| FR4.3 | The user shall be able to revoke an individual device session, and shall be required to confirm explicitly when the target is the session making the request. | FR-AUTH-15, AC-AUTH-09 | Must |
| FR4.4 | The user shall be able to revoke all sessions at once. | FR-AUTH-15 | Must |
| FR4.5 | Revocation shall call the identity provider first, then mark the local session revoked, then record an audit event; a provider failure shall leave the local row unchanged and surface a recoverable error. | FR-AUTH-15, BR-AUTH-10 | Must |
| FR4.6 | A password, MFA, or passkey change shall revoke every other session for that user. | FR-AUTH-22 | Must |
| FR4.7 | A user shall be able to see and revoke only their own sessions; an administrator may act on another user's sessions only through the admin path, and a refusal shall not disclose whether the target exists. | FR-AUTH-15, BRD-012 NFR-SEC-07 | Must |

### FR5 — Multi-factor authentication

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR5.1 | Every user shall be able to enrol a TOTP authenticator as a second factor and shall receive one-time recovery codes at enrolment. | FR-AUTH-12 | Must |
| FR5.2 | A user with an exchange credential shall be required to hold a verified second factor; while their session is single-factor, the SPA shall hold them in a limited session that reaches only the MFA and settings screens and the API shall refuse portfolio access. | FR-AUTH-12, BR-AUTH-09, AC-AUTH-08 | Must |
| FR5.3 | MFA verification shall be required again for a sensitive change (password, MFA method, recovery codes) even inside an authenticated session. | BR-AUTH-09 | Must |
| FR5.4 | MFA enrolment, verification, and recovery-code regeneration shall each record an audit event with actor, action, target, timestamp, context, and outcome, and shall never record a code or a shared secret. | BRD-012 NFR-SEC-13 | Must |

### FR6 — Sign-up gates and email verification

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR6.1 | Before an account is created, the user shall accept the terms, acknowledge the risk disclosure, and confirm they meet the age requirement. | FR-AUTH-19, BR-AUTH-12 | Must |
| FR6.2 | The sign-up flow shall detect the user's jurisdiction and shall deny sign-up where the jurisdiction is not on the launch list, with a refusal that names the reason and no workaround. | FR-AUTH-20, AC-AUTH-12 | Must |
| FR6.3 | The gates in FR6.1 and FR6.2 shall be enforced on the trusted side before provisioning, not only by the browser; a client that skips the gate shall not get an account. | BR-AUTH-12, AC-AUTH-12 | Must |
| FR6.4 | The sign-up form shall collect jurisdiction, terms acceptance, risk acknowledgement, and age confirmation as real user choices and send them to the signup gate; fixed or default values shall not be submitted on the user's behalf. | FR-AUTH-19, FR-AUTH-20 | Must |
| FR6.5 | The signup-gate submission shall be single-use and bound to the sign-up attempt it authorised, so a captured gate token cannot authorise a second account. | FR-AUTH-19 (integrity), BR-AUTH-12 | Must |
| FR6.6 | A user's email ownership shall be verified before an exchange credential can be added, and the verification state shall be re-read from the trusted side rather than trusted from the client. | FR-AUTH-17, AC-AUTH-11 | Must |
| FR6.7 | A user shall be able to request a fresh verification email, subject to a rate limit, and the response shall not reveal whether the address has an account. | FR-AUTH-17, BRD-012 NFR-SEC-12 | Must |

### FR7 — Login alerts

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR7.1 | A sign-in from a device the user has not previously used, or from an unusual location, shall create an in-app notification for that user. | FR-AUTH-16, AC-AUTH-10 | Must |
| FR7.2 | The alert shall carry no token, no credential, and no full device fingerprint, and shall tell the user how to review and revoke the session. | BR-AUTH-11 | Must |
| FR7.3 | A repeated sign-in from the same known device shall not raise a new alert for each sign-in. | FR-AUTH-16 (implied by "new device") | Should |

### FR8 — Account recovery

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR8.1 | A user who has lost access to both their email account and their device shall be able to recover through a support-assisted path that records a reason, requires an authorised support agent, and is audited end to end. | FR-AUTH-18, BRD OQ-005 **resolved by interview Q6** | Must |
| FR8.2 | Recovery shall re-prove identity without the application ever seeing a password, a provider secret, or a factor seed, and shall issue a new session only after the support path completes. | FR-AUTH-18, BR-AUTH-01 | Must |
| FR8.3 | Recovery shall revoke every existing session for that user. | FR-AUTH-18 with FR-AUTH-22 | Must |
| FR8.4 | A recovery attempt that fails identity re-proof shall leave the account untouched and produce no signal to the requester about which check failed. | FR-AUTH-18, FR-AUTH-11 | Must |

### FR9 — Gateway reachability

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR9.1 | Every auth and session path the SPA calls shall be declared in the gateway's deny-by-default route table with an owning service handler behind it, and shall be unreachable by any other route. | Team practice (Practices Discovery Q2); BRD-003 scope | Must |
| FR9.2 | Auth routes declared at the gateway shall carry a stricter per-route rate budget than ordinary reads. | BRD-012 NFR-SEC-12 | Must |
| FR9.3 | A path that the SPA calls but no service serves shall be treated as a defect: either the route and handler are added, or the SPA call is removed. | Team practice; `api-documentation.md` gap | Must |

### FR10 — Outage behaviour

| ID | Requirement | BRD source | Priority |
|---|---|---|---|
| FR10.1 | During a Supabase Auth outage, a user who is already signed in shall continue working until their current access token expires; no fallback authentication path shall exist. | BRD OQ-003 **resolved by interview Q5** | Must |
| FR10.2 | During the same outage, a new sign-in, sign-up, refresh, or MFA challenge shall fail with a non-revealing service-unavailable message and shall not retry in a loop that amplifies load. | FR-AUTH-11, BRD OQ-003 | Must |
| FR10.3 | Recovery from the outage shall be automatic: once the provider answers again, the next attempt shall succeed with no operator action. | FR-AUTH-11 | Must |

---

## Non-functional requirements

| ID | Requirement | Target | BRD source |
|---|---|---|---|
| NFR1 | Session and device-list reads shall answer within a stated latency budget at the expected load. | p95 ≤ 300 ms and p99 ≤ 800 ms for `GET` session list at 50 requests/second per replica | BRD-012 (NFR-AVL/PERF family) |
| NFR2 | Sign-in, refresh, and revocation shall remain correct when two tabs or two devices act at once; the last completed revocation wins and no tab silently resurrects a revoked session. | No lost or resurrected session in a concurrent-revocation test | FR-AUTH-10, FR-AUTH-15 |
| NFR3 | The service shall meet the platform monthly availability target, and auth failures shall degrade only auth. | 99.9% availability over a 30-day rolling window; no more than 0.1% of auth requests failing outside an outage | BRD-012 NFR-AVL-01 |
| NFR4 | Every authentication screen shall be operable by keyboard and screen reader and shall not convey status by colour alone. | WCAG 2.2 AA on sign-in, sign-up, MFA, sessions, and recovery screens | BRD-018 / design system |
| NFR5 | Security-relevant auth events shall be recorded in the append-only audit trail. | Sign-in, sign-out, failed sign-in, MFA enrol/verify/reset, session revocation, recovery, and jurisdiction refusal each produce one record with actor or system, action, target, timestamp, context, and outcome | BRD-012 NFR-SEC-13 |
| NFR6 | No credential, token, factor seed, recovery code, or full device fingerprint shall appear in a log, an error, a URL, an analytics event, or a support screen. | Zero occurrences; verified by a dedicated test and a log review | BRD-012 NFR-SEC-03, BR-AUTH-07 |
| NFR7 | Every new authentication surface shall carry at least one health signal and one error-rate signal. | `/healthz`, `/readyz`, `/metrics` on the serving component; an auth error-rate series | Project practice (operation phase guardrail) |
| NFR8 | Authentication and session code shall meet the team's testing posture. | 80% line coverage per Rust crate, measured with the workspace or paired `cargo llvm-cov` run with the command quoted; a green `yarn e2e` for any change touching `web/src/auth/`, `web/src/features/auth/`, or the auth journeys | `team-practices.md`, Testing Posture |
| NFR9 | Session and device records shall be retained only as long as needed to show recent history and to satisfy the audit requirement. | Session rows kept for 90 days after expiry, then purged; audit rows follow the audit retention policy | BRD-012 retention |
| NFR10 | The authentication surface shall work on the supported browsers and shall degrade with a clear message elsewhere. | Current and previous major versions of Chrome, Firefox, Safari, and Edge; an unsupported-browser message rather than a broken screen | BRD-018 |

---

## Constraints

1. **Supabase Auth owns identity.** Password storage, password reset, lockout, password policy,
   email verification, MFA challenges, and provider-side revocation belong to Supabase Auth. The
   application owns role projection, MFA *policy*, device visibility, and the jurisdiction gate
   (BRD-003 Notes).
2. **The SPA calls only the gateway**, with Supabase Auth the single permitted direct browser
   exception. Any other direct service call is a defect (project mandated rules).
3. **The gateway's route table is deny-by-default.** Adding capability means adding a declaration
   (project mandated rule, affirmed at this intent's Practices Discovery).
4. **Stack is fixed**: Rust with Axum, tower, sqlx, `rust_decimal`, tokio; React 18 + TypeScript 5
   with Vite and TanStack Query; Yarn 4.18.1 only; no message queues; synchronous internal HTTP
   with retries and idempotency keys; one schema and one role per service.
5. **Secrets live in Supabase Vault** and reach services through `*_FILE` variables; a credential is
   never in source, an env file, a log, or an error.
6. **Production is the only environment**; there is no staging tier.
7. **Schema changes ship as versioned sqlx up/down pairs**, via the service's migrate one-shot, and
   a rollback applies the matching down migration — and this intent carries the automatic down step
   as an explicit deliverable (Practices Discovery Q7).

---

## Assumptions

| # | Assumption | Rationale | Owner |
|---|---|---|---|
| A1 | Supabase Auth supports TOTP enrolment and AAL2 verification in the version this project self-hosts, and exposes the session list needed for device management through its Admin API or through Identity's own device rows. | Identity already has a device-session table and an `AuthAdmin` port; the design stage must confirm the Admin API surface before relying on it. | Architect, at Domain Design |
| A2 | The existing `sessionStorage` PKCE-verifier exception is sufficient for magic-link callbacks as well as OAuth callbacks. | Both are redirect flows; the verifier is needed across the navigation and is not a token. | Developer, at Functional Design |
| A3 | The launch jurisdiction list is already modelled (the jurisdiction gate exists and refuses `422`), so FR6.2 needs wiring, not new policy. | `business-overview.md` and the identity code show a jurisdiction gate and a `C8` contract. | Product owner |
| A4 | Deferred sign-in methods (Google/Apple OAuth, passkeys) leave no dead UI behind: the sign-in page will not show a provider button that does nothing. | Interview Q1 deferred them; a disabled-looking control is worse than an absent one. | Designer |

---

## Out of scope

- **Google and Apple OAuth sign-in** (FR-AUTH-13) and **passkeys/WebAuthn** (FR-AUTH-14) — deferred by
  interview Q1 to a follow-up intent.
- **Phone OTP** in every launch market — excluded by interview Q8 (BRD OQ-004 closed as "no").
- **In-product password storage or reset** — BRD-003 out-of-scope line; Supabase Auth owns it.
- **Enterprise SSO/SCIM** — post-MVP per BRD-003.
- **Native mobile authentication** — post-MVP per BRD-003.
- **Email delivery of login alerts** — interview Q7 chose in-app only for this intent.
- **Identity-provider-initiated single logout** (OQ-002) — remains open; this intent covers
  application-initiated sign-out only.
- **Blue/green or canary rollout** — no such path is adopted.

---

## Open questions

| ID | Question | Why it is still open | Owner |
|---|---|---|---|
| Q1 | Identity-provider-initiated single logout (BRD OQ-002): does a provider-side sign-out elsewhere end the SPA session? | Not decided at the interview; it depends on whether Supabase Auth emits such a signal the SPA can observe. | Security |
| Q2 | Exactly which Supabase Admin API calls back the device list, given the earlier design note that the Admin API exposes no session list. | A design-stage fact, not a product decision; the device-session table may be the only source. | Architect, Domain Design |
| Q3 | Whether the 30-day absolute session cap is measured from first sign-in or from the last refresh. | Interview Q3 set the cap but not its anchor; it must be one rule. | Security, at Functional Design |
| Q4 | Who approves a support-assisted recovery under FR8.1 — the existing support-grant second approver, or a dedicated role? | BRD-002's support-grant approval exists; recovery may reuse it. | Compliance |
| Q5 | Whether an "unusual location" for FR7.1 means a new country, a new city, or an implausible travel speed. | Interview Q7 chose the channel, not the detection rule; the rule needs a measurable definition. | Security |

---

## Traceability summary

| BRD requirement | Covered by |
|---|---|
| FR-AUTH-01 | FR1.5 |
| FR-AUTH-02 | FR1.3, FR2.5 |
| FR-AUTH-03 | FR1.1, FR1.2, FR1.4 |
| FR-AUTH-04 | FR3.1 |
| FR-AUTH-05 | FR3.3 |
| FR-AUTH-06 | FR2.2 |
| FR-AUTH-07 | FR2.3 |
| FR-AUTH-08 | FR4.1 |
| FR-AUTH-09 | FR3.5 |
| FR-AUTH-10 | NFR2 |
| FR-AUTH-11 | FR3.7, FR10.2, FR10.3 |
| FR-AUTH-12 | FR5.1, FR5.2 |
| FR-AUTH-13 | Out of scope (deferred, Q1) |
| FR-AUTH-14 | Out of scope (deferred, Q1) |
| FR-AUTH-15 | FR4.2, FR4.3, FR4.4, FR4.7 |
| FR-AUTH-16 | FR7.1, FR7.2, FR7.3 |
| FR-AUTH-17 | FR6.6, FR6.7 |
| FR-AUTH-18 | FR8.1, FR8.2, FR8.4 |
| FR-AUTH-19 | FR6.1, FR6.4, FR6.5 |
| FR-AUTH-20 | FR6.2, FR6.3 |
| FR-AUTH-21 | FR2.4, FR2.5 |
| FR-AUTH-22 | FR4.6, FR8.3 |
| BR-AUTH-01 | FR1.1, FR1.2, FR8.2, constraint 1 |
| BR-AUTH-02 | FR3.2 |
| BR-AUTH-03 | FR2.1 |
| BR-AUTH-04 | FR3.3 |
| BR-AUTH-05 | FR3.4 |
| BR-AUTH-06 | FR2.4 (no script-readable token cookie is issued) |
| BR-AUTH-07 | NFR6, FR7.2 |
| BR-AUTH-08 | FR3.1 |
| BR-AUTH-09 | FR5.2, FR5.3 |
| BR-AUTH-10 | FR3.8, FR4.5 (with the documented residual window) |
| BR-AUTH-11 | FR7.2 |
| BR-AUTH-12 | FR6.1, FR6.3 |
| AC-AUTH-01 … AC-AUTH-12 | Each maps to its FR above; acceptance criteria are written per requirement in the Functional Design stage |
| BRD OQ-001 | Resolved: FR2.1 (Q3) |
| BRD OQ-002 | Still open: see Open questions Q1 |
| BRD OQ-003 | Resolved: FR10.1–FR10.3 (Q5) |
| BRD OQ-004 | Resolved: out of scope (Q8) |
| BRD OQ-005 | Resolved: FR8.1–FR8.4 (Q6) |
