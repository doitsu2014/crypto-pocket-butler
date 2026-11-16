# Requirements Analysis — Questions

These questions settle what BRD-003 leaves open, plus one apparent conflict between the BRD's own
rules and the way the system validates tokens today. Everything else in the BRD is clear enough to
turn into requirements directly.

Answer each with a letter (A–D), or **X** plus your own words if none fits.

## Q1. [product manager] Which sign-in methods does this intent deliver?

BRD-003 marks email/password, magic link, OAuth and passkeys as in scope, but the SPA today
implements email/password only. Google/Apple OAuth (FR-AUTH-13) and passkeys (FR-AUTH-14) are both
marked *Should*, not *Must*. Delivering all four in one intent is roughly double the surface.

- A. Email/password + magic link now; OAuth and passkeys deferred to a follow-up intent
- B. Everything in the BRD now — email/password, magic link, Google/Apple OAuth and passkeys
- C. Email/password only now; every other method deferred
- D. Email/password + magic link + OAuth now, passkeys deferred
- X. Other (please specify)

[Answer]: A — email/password and magic link in this intent; Google/Apple OAuth and passkeys deferred to a follow-up intent. **Mode:** guided

## Q2. [product manager] What does MFA cover in this intent?

MFA is available to all users and required for users with exchange credentials (FR-AUTH-12,
BR-AUTH-09). Today the SPA has an MFA settings page and a "limited session" redirect, but no
enrolment flow wired to a provider challenge.

- A. TOTP enrolment and verification for all users; AAL2 enforced for exchange-credential users
- B. TOTP only for exchange-credential users (the required case), no opt-in for others
- C. TOTP and WebAuthn/passkey second factor both available
- D. Defer MFA to the follow-up intent that adds exchange credentials
- X. Other (please specify)

[Answer]: A — TOTP enrolment and verification for all users; AAL2 enforced for exchange-credential users. **Mode:** guided

## Q3. [security engineer] What are the production token lifetimes? (BRD OQ-001)

The BRD leaves this open, and it drives both the silent-refresh requirement (FR-AUTH-06) and the
revocation window. Supabase Auth's defaults are 1 hour for access tokens and no refresh expiry.

- A. Access 1 hour, refresh 7 days with rotation on use, absolute session cap 30 days
- B. Access 15 minutes, refresh 7 days with rotation, absolute cap 30 days
- C. Access 1 hour, refresh session-scoped (until browser close), absolute cap 7 days
- D. Keep Supabase defaults and document them as-is
- X. Other (please specify)

[Answer]: A — access 1 hour, refresh 7 days with rotation on use, absolute session cap 30 days. **Mode:** guided

## Q4. [security engineer] How does a revoked session take effect on the next request?

BR-AUTH-10 says revocation takes effect on the next request, not at token expiry. But FR-AUTH-04
describes stateless JWT validation, and today the gateway and Identity validate the token alone —
a revoked session's unexpired JWT would still pass. These cannot both be true without a check
somewhere.

- A. Services keep stateless JWT validation; the SPA holds no token after revocation and Identity's
  session list is the user-facing control; the gap to token expiry is accepted and documented
- B. The gateway adds a short-TTL (≤60s) revoked-session check against Identity for user routes
- C. Identity writes revocations to a shared deny-list every service checks
- D. Shorten access-token lifetime enough that the window is negligible, no check added
- X. Other (please specify)

[Answer]: A — stateless JWT validation stays; the SPA drops its token on revocation, Identity's session list is the user-facing control, and the gap to token expiry is accepted and documented. **Mode:** guided

## Q5. [security engineer] What happens during a sustained Supabase Auth outage? (BRD OQ-003)

FR-AUTH-11 requires a non-revealing error when the provider is unavailable. The open question is
what users already signed in can still do.

- A. Signed-in users keep working until their access token expires; new sign-ins fail with a plain
  "sign-in unavailable" message; no fallback auth path
- B. Services cache validated tokens for a bounded grace window so signed-in users survive short
  outages
- C. Everything refuses immediately with a service-unavailable problem
- D. A break-glass local credential path is added
- X. Other (please specify)

[Answer]: A — signed-in users keep working until their access token expires; new sign-ins fail with a plain "sign-in unavailable" message; no fallback auth path. **Mode:** guided

## Q6. [compliance specialist] What recovery paths are acceptable when both email and device are lost? (BRD OQ-005)

FR-AUTH-18 requires recovery without exposing secrets, but not how identity is re-proved.

- A. Support-assisted recovery through Identity's support-grant flow with a recorded reason and audit
- B. Recovery codes issued at MFA enrolment, plus support assistance as the last resort
- C. Verified backup email address only
- D. No self-service path; recovery is support-only with identity documents off-system
- X. Other (please specify)

[Answer]: A — support-assisted recovery through Identity's support-grant flow with a recorded reason and audit. **Mode:** guided

## Q7. [product manager] Which login-alert channel, and is it in this intent?

FR-AUTH-16 (new device or unusual-location sign-in) is a *Should*, and BR-AUTH-11 forbids tokens
and full device fingerprints in the alert. There is a Notification service, and no email sender is
wired for auth events today.

- A. In-app notification only, in this intent
- B. In-app plus email through the Notification service, in this intent
- C. Defer alerts to a follow-up intent
- D. In-app only, and only for sign-ins from a device the user has not seen before
- X. Other (please specify)

[Answer]: A — in-app notification only, delivered in this intent. **Mode:** guided

## Q8. [product manager] Is phone OTP needed in any launch market? (BRD OQ-004)

The BRD lists phone-only identity as out of scope for launch but leaves the per-market question
open. It affects the sign-up gate and the recovery story.

- A. No — email is the only identity at launch in every market
- B. Not at launch, but the jurisdiction model must allow adding it later without a schema change
- C. Yes, in at least one launch market (say which)
- D. Undecided; record as an open question carried to delivery planning
- X. Other (please specify)

[Answer]: A — no phone OTP; email is the only identity at launch in every market. **Mode:** guided
