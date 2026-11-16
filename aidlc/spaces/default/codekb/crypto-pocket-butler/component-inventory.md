# Component Inventory

The following components were understood from source in this run and the focused reporting scan. Other workspace members were seen as package names but were not given component entries. Health judgments are scoped to the observed areas, not a system-wide rating.

## SPA Authentication Session

- Responsibility: Supabase client, PKCE, memory token store, sign-in/sign-up/sign-out, and the email/password page.
- Paths: `web/src/auth/client.ts`, `web/src/auth/storage.ts`, `web/src/auth/session.ts`, `web/src/auth/session.test.ts`, `web/src/features/auth/SignInPage.tsx`.
- Depends on: `@supabase/supabase-js`, browser `sessionStorage` for the verifier only, signup gate via the gateway client.
- Health: at-risk. Token storage matches the in-memory rule. The page does not implement OAuth, MFA, or passkeys, and sign-up hard-codes jurisdiction and age.

## SPA Gateway Client

- Responsibility: the module that sends browser API calls to the gateway with the bearer token and a correlation id.
- Paths: `web/src/api/client.ts` (request path read; later methods cited by the scan, not fully re-read).
- Depends on: SPA Authentication Session for the access token; gateway base URL.
- Health: healthy for the request path that was read. Signup-gate routing through the gateway is at-risk because the gateway table that was read does not declare that path.

## Gateway Route Table

- Responsibility: deny-by-default declaration of public `/v1` routes, owning service, admin flag, and optional per-route budget.
- Paths: `services/gateway/domain/src/route.rs`.
- Depends on: gateway domain method and path types. Forwards to `asset-registry` or `identity` by name only; HTTP forwarding code was not read.
- Health: at-risk for this intent. Session list/revoke and the signup gate are absent from `default_registry_routes`.

## Gateway JWT Validator

- Responsibility: validate Supabase JWTs with cached JWKS. Checks signature, issuer, audience, and expiry.
- Paths: `services/gateway/infrastructure/src/auth.rs` (header and key type read; validation function body not fully re-read).
- Depends on: `jsonwebtoken`, gateway application `TokenValidator` port.
- Health: healthy on the stated checks. Runtime failure behavior was not executed.

## Identity HTTP API

- Responsibility: Axum router splitting public, user-JWT, and service-token routes, including device sessions and the signup gate.
- Paths: `services/identity/http/src/routes.rs`, `services/identity/http/src/middleware.rs`, `services/identity/Cargo.toml`.
- Depends on: application use cases and `AppState` (not opened). User routes use `require_jwt`.
- Health: at-risk where 503 copies validator detail into the problem body. Route grouping itself is clear.

## Identity Device Sessions

- Responsibility: list and revoke the caller's device sessions; revoke the provider session before the local row; audit both actions.
- Paths: `services/identity/application/src/sessions.rs`.
- Depends on: ports `DeviceSessionRepository`, `UserRepository`, `AuthAdmin`, `Clock`, `Trail`. Adapters were not read.
- Health: healthy for the use-case rules that were read. Persistence and the Supabase admin adapter were not read, so end-to-end revocation is not verified.

## Reporting Domain

- Responsibility: reporting domain entities, validation, business rules, and composition/authorization APIs.
- Paths: `services/reporting/domain/Cargo.toml`, `services/reporting/domain/src/audit.rs`, `brief.rs`, `compose.rs`, `error.rs`, `ids.rs`, `lib.rs`, and `service.rs`.
- Depends on: source-used `chrono`, `rust_decimal`, `thiserror`, and `uuid`. `serde` and `serde_json` are declared in the manifest but unused in the analyzed source and prohibited by the domain-crate boundary check.
- Health: the focused finding is a manifest-level boundary violation; the scanned domain code itself showed no use of the two prohibited dependencies. No build or boundary command was run.
