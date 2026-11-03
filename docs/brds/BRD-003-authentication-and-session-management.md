# BRD-003 — Authentication & Session Management

Draft v1.0 · Product/Security · 2026-11-01. How users sign in, how sessions live, and how the API trusts requests. Identity is delegated to **Supabase Auth** — the application never touches passwords.

**In scope:** Supabase Auth SSO (email/password, magic link, MFA), PKCE for OAuth redirects, in-memory session tokens with silent refresh, JWT validation at the API, lazy first-login provisioning, edge-case handling.
**Out of scope:** password storage/reset in-product, social login, native mobile auth, SSO/SCIM — post-MVP.

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
| FR-AUTH-10 | Multiple concurrent sessions/devices per user. | Should |
| FR-AUTH-11 | Clear, non-revealing error when Supabase Auth is unavailable. | Should |
| FR-AUTH-12 | MFA (TOTP/WebAuthn) via Supabase Auth. | Won't (MVP) |
| FR-AUTH-13 | Additional OAuth providers (Google, GitHub). | Won't (MVP) |

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

## Notes

- Use the configured Supabase URL/JWKS/publishable key, never hard-coded endpoints; document clock-skew allowance.
- Auth failures log correlation IDs, never token contents.
- Supabase Auth owns lockout, password policy, verification, MFA, revocation; no server-side session store in the MVP.

## Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | Production access/refresh token lifetimes? | Security |
| OQ-002 | Identity-provider-initiated single logout? | Security |
| OQ-003 | Fallback during sustained provider outage? | Operations |
| OQ-004 | Service account for scheduled jobs, or no user identity? | Architecture |
