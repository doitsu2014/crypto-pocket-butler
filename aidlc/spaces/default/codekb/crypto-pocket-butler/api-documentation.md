# API Documentation

Surfaces below are only those read in source during the contributing scans. Endpoint counts for other services were not derived. No server was started.

## SPA to Supabase Auth

Configured in `web/src/auth/client.ts`: `flowType: "pkce"`, `autoRefreshToken: true`, `detectSessionInUrl: true`, `persistSession: true` with the memory adapter.

Operations read in `web/src/auth/session.ts`:

- `signInWithPassword({ email, password })`
- `signUp({ email, password, options.data.gate_token })` when a gate token is present
- `signOut()`
- `getSession()` and `onAuthStateChange` to fill the in-memory store

The anon key and URL come from `VITE_SUPABASE_URL` and `VITE_SUPABASE_ANON_KEY`. Empty values leave the client `null`, and sign-in then throws "Supabase Auth is not configured".

## SPA to the gateway

`GatewayClient.request` (`web/src/api/client.ts`):

- Sends `Accept: application/json`, `x-correlation-id`, and `Authorization: Bearer` when a token exists and `auth` is not false.
- JSON body unless the body is `FormData`.
- Non-OK responses become `ApiError` from the problem payload.
- 204 returns undefined.

The scan also notes signup-gate, profile, MFA enrollment/recovery, and consent methods on this client. Their method bodies were not re-read in this synthesis. `SignInPage` does call `createGate` then `signUp`.

## Gateway public allowlist

`RouteTable::default_registry_routes` declares GET/POST/PUT/PATCH/DELETE patterns to `asset-registry` or `identity`. Undeclared paths are not in the table. Selected Identity routes set per-route request budgets.

Asset routes present include `/v1/assets/resolve`, `/v1/assets/{assetId}/price`, and admin asset, token-contract, price-source, and observation routes.

Identity routes present include principals, users, admin user lookup/role/disable/restore, `/v1/me/*` preferences, consent, account status, profile, deletion-request, support-grant routes, privacy deletions, and export routes.

Not in that table: `POST /v1/signup-gate`, `GET /v1/users/{userId}/sessions`, `DELETE /v1/users/{userId}/sessions/{sessionId}`.

## Identity HTTP

From `services/identity/http/src/routes.rs`.

Public, no JWT: `GET /healthz`, `GET /readyz`, `GET /metrics`, `POST /v1/signup-gate`.

User JWT (`require_jwt`), among others:

- `GET /v1/principals/{authUserId}`
- `GET /v1/users/{userId}/sessions`
- `DELETE /v1/users/{userId}/sessions/{sessionId}`
- Consent, preferences, exports, privacy deletions, support grants
- `GET /v1/users/{userId}`, `POST /v1/admin/users/lookup`
- Admin role, disable, restore
- `/v1/me/preferences`, consent, account-status, profile, deletion-request

Service-token groups include internal user registry read/write, contact, due users, privacy sweep, jurisdiction check, `/internal/authorize`, `/internal/admin/role`, internal consent, and internal account lifecycle. The file comments say `/internal/authorize` and `/internal/admin/role` are not on the gateway `/v1` table.

## Session use-case contract

`SessionService` (`services/identity/application/src/sessions.rs`):

- `list`: owner or admin, then repository list, then audit `identity.sessions.read`.
- `revoke`: owner or admin; unknown session or another user's session is hidden as not found; already revoked is idempotent success; current session without `confirm_current` conflicts; provider revoke, then persist, then audit `identity.session.revoked`.

Problem-body shape follows the read `problem-details.md` knowledge file (RFC 7807, no rejected values). This run did not re-read the Identity problem implementation.

## Reporting Domain

The focused scan covered a Rust domain library and found no HTTP endpoints. It exports domain types and functions through `services/reporting/domain/src/lib.rs`, including brief, delivery, audit, identifier, error, composition, and service-authorization APIs. The manifest declares `serde` and `serde_json`, but no analyzed domain source uses them; both dependencies are forbidden in domain crates by `scripts/check-boundaries.sh`.
