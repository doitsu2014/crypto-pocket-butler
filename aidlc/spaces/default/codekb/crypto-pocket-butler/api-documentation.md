# API Documentation — Crypto Pocket Butler

## API Surface Summary

| Surface | Technology | Where | Auth |
| --- | --- | --- | --- |
| External edge | HTTP/JSON via Axum fallback proxy | `services/gateway/http/src/routes.rs` | Supabase JWT (RS256, JWKS) |
| Service user APIs | HTTP/JSON (Axum) | each `services/<svc>/http/src/routes.rs` | Forwarded Supabase JWT, re-validated (135 route declarations across the 12 routers, including probes) |
| Internal service APIs | HTTP/JSON (Axum) | each `services/<svc>/http/src/routes.rs` | Hashed service token from a Docker secret |
| Operational | `/healthz`, `/readyz`, `/metrics` | every service | None (private network) |

No GraphQL, gRPC, WebSocket or message-queue contract exists. The only streaming path in the documentation is an SSE progress surface, which is not present in the code.

## External Edge — API Gateway

Router: `services/gateway/http/src/routes.rs`. Health, readiness and metrics are local; **every other request** goes through `proxy` → `AuthorizeAndRoute::execute`.

| Method | Path | Declared in route table | Admin | Upstream |
| --- | --- | --- | --- | --- |
| GET | `/healthz` | local | — | — |
| GET | `/readyz` | local | — | — |
| GET | `/metrics` | local | — | — |
| GET | `/v1/assets/resolve` | yes | no | asset-registry |
| GET | `/v1/assets/{assetId}/price` | yes | no | asset-registry |
| POST | `/v1/admin/assets` | yes | yes | asset-registry |
| POST | `/v1/admin/token-contracts` | yes | yes | asset-registry |
| PUT | `/v1/admin/price-sources` | yes | yes | asset-registry |
| PATCH | `/v1/admin/price-sources/{name}/enabled` | yes | yes | asset-registry |
| POST | `/v1/admin/observations/{id}/resolve` | yes | yes | asset-registry |
| POST | `/v1/admin/observations/{id}/ignore` | yes | yes | asset-registry |
| any | anything else | **no declared route** | — | 404 `NotFound` |

The route table is `RouteTable::default_registry_routes()` (`services/gateway/domain/src/route.rs:68-90`). There is currently **no way to configure additional routes at runtime**: `services/gateway/src/config.rs` exposes only `ASSET_REGISTRY_URL` and gateway settings.

### Edge request behaviour

- Missing/invalid/expired token → **401** with a `gateway.denied` audit event.
- Undeclared path → **404** and a `gateway.denied` audit event.
- Admin route with non-admin role → **403** and a `gateway.denied` audit event.
- `x-correlation-id`: honoured when 1–128 ASCII `[A-Za-z0-9-_.:]`, generated otherwise, echoed on the response.
- `idempotency-key`: preserved when supplied; generated for non-idempotent methods (POST, PATCH) when absent.
- Retries: idempotent methods (GET, HEAD, OPTIONS, PUT, DELETE) get up to 3 attempts; non-idempotent methods get 1.
- Hop-by-hop headers and `host`/`content-length`/`x-correlation-id` are stripped before forwarding.
- Rate limits: fixed 60-second window, in-memory, keyed on forwarded client IP plus token presence; authenticated budget default 120/min, anonymous 30/min (`services/gateway/http/src/rate_limit.rs`).
- Body limit: 1 MiB.

## Service APIs

Auth classes below: **public** (no auth), **JWT** (user token, re-validated in the service), **admin** (JWT with `app_metadata.role = "admin"`), **service** (per-caller service token), **JWT-or-service** (either).

### Identity Service (`services/identity/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| POST | `/v1/signup-gate` | public | Pre-sign-up jurisdiction/age gate token |
| GET | `/v1/principals/{authUserId}` | JWT | Resolve/provision principal (WF1) |
| POST | `/v1/consent` | JWT | Append a consent record |
| GET | `/v1/consent/{userId}` | JWT | Latest consent per purpose (path subject must equal token subject) |
| PATCH | `/v1/users/{userId}/preferences` | JWT | Locale, base currency, timezone |
| GET | `/v1/users/{userId}/sessions` | JWT | Device session view |
| DELETE | `/v1/users/{userId}/sessions/{sessionId}` | JWT | Revoke a device session |
| POST | `/v1/exports` | JWT | Request a data export |
| GET | `/v1/exports/{exportId}` | JWT | Export status/download |
| POST | `/v1/privacy/deletions` | JWT | Request account deletion |
| GET | `/v1/privacy/deletions/{deletionId}` | JWT | Deletion status |
| POST | `/v1/support-grants` | JWT (support role) | Request a support grant |
| POST | `/v1/support-grants/{grantId}/approve` | JWT (admin, not requester) | Approve a grant |
| DELETE | `/v1/support-grants/{grantId}` | JWT | Revoke a grant |
| GET | `/v1/users/{userId}/contact` | service — **notification only** | C3 verified contact at delivery time |
| GET | `/v1/internal/users/due` | service — **reporting only** | Users due for a period report |
| POST | `/v1/internal/run/audit-privacy-sweep` | service — **job-orchestration only** | C11 run endpoint |
| GET | `/v1/internal/support-grants/{grantId}/check` | service — **gateway only** | C2/C3 support-grant check on the metadata path |

### Account Service (`services/account/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| POST | `/v1/accounts/exchange` | JWT | Add a read-only exchange account |
| POST | `/v1/accounts/wallet` | JWT | Add a watch-only wallet address |
| POST | `/v1/accounts/walletconnect` | JWT | Add a WalletConnect account |
| POST | `/v1/accounts/{accountId}/resync` | JWT | Trigger a sync |
| POST | `/v1/accounts/{accountId}/imports` | JWT | CSV import (11 MiB cap) |
| GET | `/v1/imports/{batchId}` | JWT | Import status |
| GET | `/v1/users/{userId}/accounts` | JWT | List accounts |
| GET | `/v1/users/{userId}/holdings` | service | C5 holdings read |
| GET | `/v1/accounts/{accountId}/transactions` | service | Transaction feed for Ledger |
| GET | `/v1/internal/privacy/export` | service | Privacy export contribution |
| POST | `/v1/internal/privacy/anonymise` | service | Privacy anonymise contribution |
| POST | `/v1/internal/run/{jobName}` | service — job-orchestration | C11 run endpoint |

### Asset Registry Service (`services/asset-registry/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| GET | `/v1/assets/resolve` | JWT | Resolve canonical asset identity |
| GET | `/v1/assets/{asset_id}/price` | JWT | Resolve a stored price |
| POST | `/v1/admin/assets` | admin | Create an asset |
| POST | `/v1/admin/token-contracts` | admin | Register a token contract |
| PUT | `/v1/admin/price-sources` | admin | Upsert a price source |
| PATCH | `/v1/admin/price-sources/{name}/enabled` | admin | Enable/disable a source |
| POST | `/v1/admin/observations/{id}/resolve` | admin | Resolve a quarantine observation |
| POST | `/v1/admin/observations/{id}/ignore` | admin | Ignore a quarantine observation |

### Market Data Service (`services/market-data/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| GET | `/v1/prices` | service | Point-in-time prices |
| GET | `/v1/fx` | service | FX rates |
| POST | `/v1/internal/run/{jobName}` | service — job-orchestration | C11 run endpoint |

### Portfolio Service (`services/portfolio/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| GET, POST | `/v1/portfolios` | JWT | List / create portfolios |
| GET | `/v1/portfolios/{portfolioId}` | JWT | Read one portfolio (owner-scoped) |
| PUT | `/v1/portfolios/{portfolioId}/accounts/{accountId}` | JWT | Assign an account |
| PUT | `/v1/portfolios/{portfolioId}/targets` | JWT | Set allocation targets |
| POST | `/v1/portfolios/{portfolioId}/order-intents` | JWT | Create an order intent |
| POST | `/v1/portfolios/{portfolioId}/order-intents/{intentId}/cancel` | JWT | Cancel an intent |
| POST | `/v1/portfolios/{portfolioId}/trades` | JWT | Record a trade |
| GET | `/v1/portfolios/{portfolioId}/snapshots` | JWT or service | C1/C8 snapshot list |
| GET | `/v1/portfolios/{portfolioId}/snapshots/{snapshotId}` | JWT or service | C1/C8 snapshot read |
| GET | `/v1/users/{userId}/portfolios` | service | Internal portfolio lookup |
| GET | `/v1/portfolios/{portfolioId}/valuation` | service | Internal valuation |
| GET | `/v1/internal/privacy/export` | service | Privacy export contribution |
| POST | `/v1/internal/privacy/anonymise` | service | Privacy anonymise contribution |
| GET | `/v1/internal/smoke` | service | Internal smoke check |

### Ledger Service (`services/ledger/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| GET | `/v1/users/{userId}/transactions` | JWT | Owner transaction list |
| POST | `/v1/transactions/{transactionId}/corrections` | JWT | Submit a correction |
| GET | `/v1/users/{userId}/pnl` | JWT or service (Reporting, `pnl.read`) | Cost basis and P&L |
| POST | `/v1/users/{userId}/trades` | JWT or allowed service | Record a trade |
| GET | `/v1/internal/privacy/export` | service | Privacy export contribution |
| POST | `/v1/internal/privacy/anonymise` | service | Privacy anonymise contribution |
| GET | `/v1/internal/smoke` | service | Internal smoke check |
| POST | `/v1/internal/run/ledger-reconcile` | service — job-orchestration | C11 run endpoint |
| POST | `/v1/internal/run/ledger-ingest` | service — job-orchestration | C11 run endpoint |

### Reporting Service (`services/reporting/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| GET | `/v1/users/{userId}/briefs` | JWT | Brief list |
| GET | `/v1/users/{userId}/briefs/{briefId}` | JWT | Brief detail |
| GET | `/v1/internal/privacy/export` | service | Privacy export contribution |
| POST | `/v1/internal/privacy/anonymise` | service | Privacy anonymise contribution |
| GET | `/v1/internal/smoke` | service | Internal smoke check |

### Notification Service (`services/notification/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| GET | `/v1/users/{userId}/notifications` | JWT | Notification feed |
| GET, PUT | `/v1/users/{userId}/notifications/preferences` | JWT | Alert preferences |
| POST | `/v1/alerts` | service (allow-list) | Queue an alert |
| POST | `/v1/internal/privacy/anonymise` | service | Privacy anonymise contribution |
| GET | `/v1/internal/smoke` | service | Internal smoke check |
| POST | `/v1/internal/run/notification-deliver` | service — job-orchestration | C11 delivery run endpoint |
| POST | `/v1/internal/run/notification-retention` | service — job-orchestration | C11 retention run endpoint (worker router) |

### Analytics Service (`services/analytics/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| POST | `/v1/events` | JWT | Consent-gated event ingestion |
| GET | `/v1/flags/{key}` | JWT | Feature-flag evaluation |
| GET | `/v1/analytics/aggregates` | admin | Aggregates |
| GET, POST | `/v1/admin/flags` | admin | List / create flags |
| PATCH, DELETE | `/v1/admin/flags/{key}` | admin | Update / delete a flag |
| GET | `/v1/internal/smoke` | service | Internal smoke check |
| POST | `/v1/internal/privacy/erase` | service — identity | Privacy erase |
| POST | `/v1/internal/run/analytics-aggregate` | service — job-orchestration | C11 run endpoint |
| POST | `/v1/internal/run/analytics-retention` | service — job-orchestration | C11 run endpoint |

### Audit Service (`services/audit/http/src/routes.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes and metrics |
| POST | `/v1/events` | service (rate-limited) | Append an audit event |
| GET | `/v1/events` | admin (rate-limited) | Query the audit trail |
| POST | `/v1/exports` | admin (rate-limited) | Create an audit export |
| POST | `/v1/support-sessions` | service (support) | Start a support session |
| POST | `/v1/support-sessions/{id}/approve` | service (support flow) | Approve a support session |
| POST | `/v1/support-sessions/{id}/revoke` | admin | Revoke a support session |
| GET | `/v1/support-sessions/{id}/check` | service | Check a support session |
| POST | `/v1/privacy/deletions` | service | Record a privacy deletion |
| POST | `/v1/runs` | service | Report a retention run |

### Job Orchestration Service (`services/job-orchestration/http/src/lib.rs`)

| Method | Path | Auth | Purpose |
| --- | --- | --- | --- |
| GET | `/healthz`, `/readyz`, `/metrics` | public | Probes, metrics, heartbeat age |
| GET | `/v1/jobs` | JWT (operator role enforced in `OperatorService`) | Job catalogue and latest attempt |
| GET | `/v1/jobs/{job}/runs` | JWT (operator) | Bounded run page |
| POST | `/v1/jobs/{job}/run` | JWT (operator) | Trigger a run |
| POST | `/v1/jobs/{job}/backfill` | JWT (operator) | Bounded backfill |
| GET | `/v1/internal/smoke` | service | Internal smoke check |

### Gateway internal paths

The gateway has no domain API of its own beyond the edge proxy. It writes to Audit over an internal client (`services/gateway/infrastructure/src/audit.rs`).

## Contracts and Conventions

### Authentication

- `Authorization: Bearer <supabase-access-token>` on every user path.
- RS256 only; signature, `iss`, `aud` and `exp` all checked against JWKS (`services/<svc>/infrastructure/src/auth.rs`).
- Internal paths carry a service token from a Docker secret; each route names the one service allowed to call it. Unknown tokens are 401; a valid token held by the wrong service is 403.
- The gateway forwards the user JWT unchanged; services re-validate it as defence in depth.

### Errors

RFC 7807 problem+json on every service (`ProblemDetails`):

```json
{
  "type": "https://crypto-pocket-butler/problem/validation-failed",
  "title": "Validation failed",
  "status": 400,
  "detail": "one or more fields failed validation",
  "instance": "<correlation-id>",
  "errors": [{ "field": "email", "reason": "is required" }]
}
```

Status mapping is consistent across services: 400 validation, 401 unauthenticated, 403 forbidden, 404 not-found-for-this-caller, 409 conflict, 413 body too large, 429 rate limited, 500 internal, 503 dependency unavailable, 504 upstream timeout at the gateway.

### Headers

| Header | Direction | Meaning |
| --- | --- | --- |
| `Authorization` | request | Supabase JWT or service token |
| `x-correlation-id` | request/response | Trace id; validated and echoed |
| `idempotency-key` | request | Preserved or generated for non-idempotent methods |
| `x-device-label`, `x-geo-location` | request (Identity) | Coarse session view; never a full IP |
| `X-Support-Reason` | request (forwarded) | Support access reason |

### Pagination and limits

- Audit queries, run pages and brief lists take bounded `limit`/time-window parameters; defaults and maxima are per service (e.g. job runs `DEFAULT_PAGE`/`MAX_PAGE`).
- Body limits are per router: 1 MiB on gateway and identity, 11 MiB on account (CSV), 256 KiB on portfolio.
- Rate limits exist at the gateway edge and on the Audit append/query/export paths.

### Versioning

- All public paths are versioned in the path (`/v1/...`); there is no header versioning.
- No OpenAPI/AsyncAPI document exists in the repository in either service or frontend package; the route tables above are the authoritative contract.

## Internal Contracts (C-numbered in code)

| Contract | Producer → consumer | Shape |
| --- | --- | --- |
| C1 | SPA → each service (via gateway) | User JWT surface |
| C2/C3 | Gateway → Identity support check; Notification → Identity contact | Support-grant check; verified contact |
| C5 | Portfolio → Account | `GET /v1/users/{userId}/holdings` |
| C7 | Reporting → Ledger | `GET /v1/users/{userId}/pnl` (`pnl.read`) |
| C8 | Reporting → Portfolio | Snapshot reads (JWT-or-service) |
| C10 | Reporting → Audit | `POST /v1/events` |
| C11 | Job Orchestration → every service | `POST /v1/internal/run/<job>` with a per-window idempotency key |
