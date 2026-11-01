# BRD-003 — Authentication & Session Management

| Field | Value |
|-------|-------|
| Document ID | BRD-003 |
| Version | 1.0 |
| Status | Draft — requires security sign-off |
| Owner | Product / Security |
| Last updated | 2026-11-01 |
| Related documents | BRD-002, BRD-012 |

## 1. Purpose

Define how users authenticate, how sessions are established and refreshed, how the API trusts requests, and how sign-out works. The product uses federated identity so that the application never stores user passwords.

## 2. Scope

### In scope

- Single sign-on through an OpenID Connect (OIDC) identity provider (Keycloak in the current deployment).
- Authorization Code flow with PKCE for the web client.
- Session cookies, access-token refresh, and sign-out.
- JWT validation and claim requirements at the API.
- First-login provisioning of an internal user record.
- Failure and edge-case handling (expired tokens, revoked sessions, provider outage).

### Out of scope

- Password storage, password reset, and credential recovery inside the product.
- Social login (Google, GitHub) — post-MVP.
- Native mobile application authentication — post-MVP.
- Organisation-level SSO/SCIM — post-MVP.

## 3. Actors

| Actor | Interaction |
|-------|-------------|
| Anonymous visitor | Initiates sign-in; has no access to portfolio data |
| Portfolio Owner / Admin | Authenticates and uses the application |
| Identity provider | Authenticates credentials, issues tokens, ends sessions |
| Web client | Maintains session cookie and attaches bearer tokens to API calls |
| API | Validates JWTs and enforces authorisation |

## 4. Functional requirements

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-AUTH-01 | An unauthenticated visitor attempting to reach a protected page must be redirected to sign-in. | Must |
| FR-AUTH-02 | Sign-in must use OIDC Authorization Code flow with PKCE; implicit flow must not be used. | Must |
| FR-AUTH-03 | After successful authentication, the web client must establish a session and redirect the user to the dashboard. | Must |
| FR-AUTH-04 | The API must validate the access token's signature, issuer, audience, and expiry on every protected request. | Must |
| FR-AUTH-05 | The API must map the token subject to an internal user, creating the user on first successful request. | Must |
| FR-AUTH-06 | The web client must transparently refresh an expired access token using the refresh token, without requiring the user to sign in again. | Must |
| FR-AUTH-07 | If the refresh token is expired, revoked, or invalid, the user must be redirected to sign-in. | Must |
| FR-AUTH-08 | Sign-out must destroy the application session and terminate the identity-provider session. | Must |
| FR-AUTH-09 | The system must reject requests with missing, malformed, expired, or incorrectly scoped tokens. | Must |
| FR-AUTH-10 | The system should support multiple concurrent sessions/devices for the same user. | Should |
| FR-AUTH-11 | The system should display a clear, non-revealing error when the identity provider is unavailable. | Should |
| FR-AUTH-12 | The post-MVP system should support multi-factor authentication (TOTP/WebAuthn) through the identity provider. | Won't (MVP) |
| FR-AUTH-13 | The post-MVP system should support additional OAuth2 identity providers (Google, GitHub). | Won't (MVP) |

## 5. Business rules

| ID | Rule |
|----|------|
| BR-AUTH-01 | Passwords are never stored, processed, or logged by the application. |
| BR-AUTH-02 | Every API route except health, public documentation, and sign-in callbacks requires a valid bearer token. |
| BR-AUTH-03 | Access tokens are short-lived; refresh tokens govern session longevity. |
| BR-AUTH-04 | The internal user is created lazily at first authenticated request, not at identity-provider registration. |
| BR-AUTH-05 | A disabled internal user must be denied even if their identity-provider token is valid. |
| BR-AUTH-06 | Session cookies must be HTTP-only and secure in production. |
| BR-AUTH-07 | Tokens must never be written to application logs or exposed to browser JavaScript beyond what the client framework requires. |
| BR-AUTH-08 | The API trusts only tokens issued by the configured issuer and intended for the configured audience. |

## 6. Data requirements

| ID | Data element | Description | Sensitivity |
|----|--------------|-------------|-------------|
| DR-AUTH-01 | Access token | Short-lived bearer credential | Secret |
| DR-AUTH-02 | Refresh token | Long-lived credential used to renew access | Secret |
| DR-AUTH-03 | Session cookie | Client-side session reference | Secret |
| DR-AUTH-04 | Issuer and audience configuration | Trust parameters for token validation | Internal |
| DR-AUTH-05 | Subject claim | Stable identity-provider user identifier | Internal |

## 7. Acceptance criteria

| ID | Given / When / Then |
|----|---------------------|
| AC-AUTH-01 | Given an anonymous visitor, when they request a protected page, then they are redirected to sign-in. |
| AC-AUTH-02 | Given valid credentials at the identity provider, when the callback completes, then the user reaches the dashboard and an internal user record exists. |
| AC-AUTH-03 | Given an expired access token and a valid refresh token, when the next API call is made, then it succeeds without user interaction. |
| AC-AUTH-04 | Given an expired refresh token, when the next request is made, then the user is redirected to sign-in. |
| AC-AUTH-05 | Given a token with the wrong audience or issuer, when it is presented to the API, then the request is rejected with 401. |
| AC-AUTH-06 | Given a signed-in user, when they sign out, then the session is destroyed and the identity-provider session is terminated. |
| AC-AUTH-07 | Given a valid token for a disabled internal user, when any protected API is called, then access is denied. |

## 8. Security and non-functional considerations

- OIDC discovery metadata should be used rather than hard-coded endpoints where possible.
- Clock skew allowance for token validation must be small and documented.
- Authentication failures must be logged with correlation identifiers but without token contents.
- Session lifetime must align with the identity provider's policy; the product must not extend it silently.
- The identity provider must be reachable over TLS in production.

## 9. Assumptions, dependencies, constraints

- A Keycloak realm and OIDC client are provisioned per environment.
- The web client and API are served over HTTPS in production.
- The identity provider is responsible for account lockout, password policy, and MFA.
- The API validates tokens on every request; no server-side session store is required for MVP.

## 10. Open questions

| ID | Question | Owner |
|----|----------|-------|
| OQ-001 | What are the required access- and refresh-token lifetimes for production? | Security |
| OQ-002 | Should the product support identity-provider-initiated single logout? | Security |
| OQ-003 | What is the fallback behaviour during a sustained identity-provider outage? | Operations |
| OQ-004 | Is a dedicated service account required for scheduled jobs, or do they run without user identity? | Architecture |
