# BRD-003 — Authentication & Session Management

Draft v1.1 · Product/Security · 2026-11-04. How users sign in, how sessions live, and how the API trusts requests. Identity is delegated to **Supabase Auth** — the application never touches passwords. v1.1 makes MFA, passkeys, device management, recovery, and global sign-up requirements explicit.

**In scope:** Supabase Auth SSO (email/password, magic link, OAuth, MFA, passkeys) · PKCE for OAuth redirects · in-memory session tokens with silent refresh · JWT validation at the API · lazy first-login provisioning · device and session management · login alerts · account recovery · email verification · age/terms/jurisdiction gates.
**Out of scope:** password storage/reset in-product · SSO/SCIM for enterprises · phone-only identity at launch (Could) · native mobile auth (post-MVP).

## Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-AUTH-01 | Unauthenticated visitors hitting a protected page are redirected to sign-in. | Must |
| FR-AUTH-02 | OAuth redirects use PKCE; implicit flow is forbidden. | Must |
| FR-AUTH-03 | Successful sign-in establishes a session and lands on the dashboard. | Must |
| FR-AUTH-04 | The API validates token signature, issuer, audience, and expiry on every protected request. | Must |
| FR-AUTH-05 | The token subject maps to an internal user, created on first successful request. | Must |
| FR-AUTH-06 | Expired access tokens refresh transparently; no re-sign-in. | Must |
| FR-AUTH-07 | Expired/revoked/invalid refresh token → redirect to sign-in. | Must |
| FR-AUTH-08 | Sign-out destroys the app session **and** the identity-provider session. | Must |
| FR-AUTH-09 | Missing, malformed, expired, or wrongly-scoped tokens are rejected. | Must |
| FR-AUTH-10 | Multiple concurrent sessions/devices per user. | Must |
| FR-AUTH-11 | Clear, non-revealing error when Supabase Auth is unavailable. | Must |
| FR-AUTH-12 | MFA (TOTP/WebAuthn) is available to all users and **required** for users with exchange credentials. | Must |
| FR-AUTH-13 | Google and Apple OAuth sign-in. | Should |
| FR-AUTH-14 | Passkeys/WebAuthn as a first-factor or step-up option. | Should |
| FR-AUTH-15 | List active devices/sessions with last-seen time and revoke individually or all. | Must |
| FR-AUTH-16 | Notify users of new device or unusual-location sign-in. | Should |
| FR-AUTH-17 | Verify email ownership before an account can add an exchange credential. | Must |
| FR-AUTH-18 | Provide account recovery for lost email or device without exposing secrets. | Must |
| FR-AUTH-19 | Require terms, risk disclosure, and age confirmation before the first source. | Must |
| FR-AUTH-20 | Detect jurisdiction and deny or restrict sign-up per BRD-017. | Must |
| FR-AUTH-21 | Bind tokens to the SPA in memory; never persist access or refresh tokens in localStorage. | Must |
| FR-AUTH-22 | Support session revocation after a password, MFA, or passkey change. | Must |

## Business rules

| ID | Rule |
|----|------|
| BR-AUTH-01 | Passwords are never stored, processed, or logged by the application. |
| BR-AUTH-02 | Every API route except health, public docs, and sign-in callbacks requires a valid bearer token. |
| BR-AUTH-03 | Access tokens are short-lived; refresh tokens govern session longevity. |
| BR-AUTH-04 | The internal user is created lazily at first authenticated request. |
| BR-AUTH-05 | A disabled internal user is denied even with a valid provider token. |
| BR-AUTH-06 | Session cookies are HTTP-only and secure in production. |
| BR-AUTH-07 | Tokens are never logged or exposed beyond what the client framework requires. |
| BR-AUTH-08 | Only tokens from the configured issuer and audience are trusted. |
| BR-AUTH-09 | Exchange credentials require MFA, verified email, and a strong session. |
| BR-AUTH-10 | A session revocation takes effect on the next request, not at token expiry. |
| BR-AUTH-11 | Login alerts never contain tokens or full device fingerprints. |
| BR-AUTH-12 | Jurisdiction and age gates run before account creation. |

## Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-AUTH-01 | Anonymous visitor requests a protected page → redirected to sign-in. |
| AC-AUTH-02 | Valid credentials complete the callback → dashboard, internal user exists. |
| AC-AUTH-03 | Expired access token + valid refresh token → next API call succeeds silently. |
| AC-AUTH-04 | Expired refresh token → redirect to sign-in. |
| AC-AUTH-05 | Wrong audience or issuer → 401. |
| AC-AUTH-06 | Sign-out → app session destroyed, provider session terminated. |
| AC-AUTH-07 | Valid token for a disabled user → access denied. |
| AC-AUTH-08 | A user with an exchange credential and no MFA is prompted to enrol before saving it. |
| AC-AUTH-09 | Revoking a device ends its session on the next request. |
| AC-AUTH-10 | A new-device sign-in produces a notification without token contents. |
| AC-AUTH-11 | Email ownership is verified before an exchange credential can be added. |
| AC-AUTH-12 | A restricted jurisdiction cannot create an account. |

## Notes

- Use the configured Supabase URL/JWKS/publishable key, never hard-coded endpoints; document clock-skew allowance.
- Auth failures log correlation IDs, never token contents.
- Supabase Auth owns lockout, password policy, verification, MFA, and revocation; the application owns role projection, MFA policy, device visibility, and jurisdiction gates.
- Enterprise SSO/SCIM is post-MVP and must not block the MVP model.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Production access/refresh token lifetimes? | Security |
| OQ-002 | Identity-provider-initiated single logout? | Security |
| OQ-003 | Fallback during sustained provider outage? | Operations |
| OQ-004 | Is phone OTP required in any launch market? | Product |
| OQ-005 | Which recovery path is acceptable when both email and device are lost? | Security/Support |
