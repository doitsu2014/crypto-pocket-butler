# Business Overview

Crypto Pocket Butler is a multi-service portfolio product. This reverse-engineering run did not re-read the whole product. It read the authentication and session path named by BRD-003, plus the workspace manifests and project knowledge files identified in the scan, and the reporting domain focused area.

## Purpose in scope

BRD-003 (`docs/brds/BRD-003-authentication-and-session-management.md`) defines how a person signs in, how a session lives, and how the API trusts a request. Identity is delegated to Supabase Auth. The application does not store or reset passwords (BR-AUTH-01).

In scope for that BRD: email/password, magic link, OAuth, MFA, and passkeys; PKCE; in-memory access and refresh tokens with silent refresh; JWT checks on protected APIs; lazy first-login provisioning; device and session management; login alerts; account recovery; email verification; age, terms, and jurisdiction gates.

Out of scope for that BRD: in-product password storage, enterprise SSO/SCIM, phone-only identity at launch, and native mobile auth.

## What the read code actually does

- The SPA configures one Supabase client with PKCE, auto-refresh, and a custom storage adapter (`web/src/auth/client.ts`). Access and refresh tokens stay in memory. Only the PKCE code verifier is written to `sessionStorage` (`web/src/auth/storage.ts`).
- Sign-in and sign-up on the page that was read are email and password only (`web/src/features/auth/SignInPage.tsx`). Sign-up calls a signup gate, then `signUp`, and currently sends fixed `jurisdiction: "supported"` and `ageConfirmed: true`.
- The SPA's `GatewayClient` attaches the current access token as a bearer token and sends `x-correlation-id` (`web/src/api/client.ts`).
- The gateway's default route table is deny-by-default and forwards declared `/v1` routes to Asset Registry or Identity (`services/gateway/domain/src/route.rs`). JWT checks cover signature, issuer, audience, and expiry, with a cached JWKS (`services/gateway/infrastructure/src/auth.rs`).
- Identity exposes user-JWT routes to list and revoke device sessions (`services/identity/http/src/routes.rs`). Revocation calls Supabase Auth before the local row is marked revoked, and it writes an audit trail (`services/identity/application/src/sessions.rs`).
- Reporting's domain crate contains reporting entities, validation, business rules, and composition/authorization APIs. Its manifest declares `serde` and `serde_json`, but the analyzed domain source does not use either dependency; the boundary script forbids these crates in domain manifests.

## Open product decisions

BRD-003 leaves five questions open: production token lifetimes, identity-provider-initiated logout, sustained provider-outage fallback, whether phone OTP is required in a launch market, and recovery when both email and device are lost. Those decisions are not settled in the files this run read.

## Coverage limit

Other services (Account, Portfolio, Ledger, and the rest of the workspace) appear as Cargo workspace members. Their business behavior was not read in this run. The service-map knowledge file names their roles; that is documentation, not a fresh code reading.
