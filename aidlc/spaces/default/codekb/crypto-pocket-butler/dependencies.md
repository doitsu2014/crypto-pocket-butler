# Dependencies — Crypto Pocket Butler

Version numbers live in `technology-stack.md`; this document records dependency **relationships**, both inside the workspace and outside it.

## Internal Cross-Package Dependencies

### Within a service (the enforced rule)

```
<svc>-http ─────────┐
                    ├──▶ <svc>-application ──▶ <svc>-domain
<svc>-infrastructure┘
        ▲
        │ (composition root only)
   <svc> package ──▶ all four crates
```

- `domain` has **no** local dependency and no framework dependency.
- `application` depends only on `domain`.
- `infrastructure` depends on `domain` and `application`; it must not import `http`.
- `http` depends on `domain` and `application`; it must not import `infrastructure`.
- The service package is the composition root and may depend on all four.
- Enforcement: `scripts/check-boundaries.sh` parses `cargo metadata --format-version 1 --no-deps` and exits non-zero on any outward edge. Test-only dependencies are excluded from the check.

### Between services

There are **no** Rust crate dependencies between services. Cross-service needs are HTTP contracts (see `api-documentation.md`). No shared `common`/`types` crate exists; concepts such as `Role` and `ProblemDetails` are intentionally duplicated per service.

### Between frontends

`web/` and `support-console/` are independent Yarn packages with separate `package.json`, lockfile and toolchain configuration. Neither imports the other; shared visual language is duplicated from `design-system/tokens.css`.

## Cross-Service Runtime Dependencies

| Consumer | Provider | Contract | Failure behaviour |
| --- | --- | --- | --- |
| API Gateway | Supabase Auth JWKS | HTTPS JWKS | 503 after 10 min stale keys |
| API Gateway | Asset Registry | forwarded HTTP | bounded retry on timeout/unavailable |
| API Gateway | Audit | `POST /v1/events` | logs a warning, does not block the request |
| Every service | Supabase Auth JWKS | HTTPS JWKS | 503 `identity-unavailable` |
| Every service | Audit | internal append | synchronous for security events; best-effort otherwise |
| Identity | Supabase Auth Admin | HTTPS | propagates as 503 |
| Identity | Account, Ledger, Portfolio | privacy export/anonymise | 503 to the caller |
| Identity | Supabase Storage | private bucket | 503 |
| Account | Asset Registry | resolve assets/contracts | 503 / quarantine observation |
| Account | Notification | internal HTTP | bounded retry |
| Portfolio | Account | holdings | 503; valuation refused rather than guessed |
| Portfolio | Ledger | P&L | 503 |
| Portfolio | Market Data | prices/FX | unpriced assets labelled and excluded |
| Portfolio | Identity | role/preferences | 503 |
| Ledger | Account | transaction feed | reconciliation repairs later |
| Reporting | Portfolio, Ledger, Identity | brief inputs | run reported failed; retried next window |
| Reporting | Notification | delivery queue | bounded retry |
| Notification | Identity | C3 verified contact | delivery deferred, never cached |
| Analytics | Identity | consent check | event rejected |
| Job Orchestration | every service | C11 run endpoint | bounded retries; missed windows recovered |
| Support Console | API Gateway | support paths | 401/403 surface to the operator |

All service-to-service calls are synchronous internal HTTP with bounded retries and idempotency keys (POST) or version checks; there is no message queue and no outbox.

## Shared Infrastructure Dependencies

| Dependency | Used by | Notes |
| --- | --- | --- |
| Supabase Postgres (one schema + role per service) | all backend services | no cross-schema grants, no RLS in services |
| Supabase Vault (pgsodium) | Identity, Account, Market Data, Notification, Audit | credentials and per-subject keys; never env files |
| Supabase Storage | Identity (exports), Reporting (PDFs), Audit (exports) | private buckets |
| Docker secrets | every service | connection strings, service tokens, provider keys |
| Prometheus / Alertmanager | Job Orchestration, every `/metrics` endpoint | heartbeat and failure alerts |
| Loki / Tempo / Grafana | every service | logs, traces, dashboards |

## External Dependencies (Rust crates)

Grouped from `Cargo.toml` `[workspace.dependencies]`; resolved versions in `technology-stack.md`.

| Group | Crates |
| --- | --- |
| Runtime | tokio, futures |
| HTTP boundary | axum, tower, tower-http, http-body-util, bytes, tokio-stream |
| Serialization | serde, serde_json |
| Persistence | sqlx (postgres, rusls, migrate) |
| Security | jsonwebtoken, base64, sha2, hex |
| Outbound HTTP | reqwest (rustls) |
| Values and identifiers | rust_decimal, chrono, chrono-tz, uuid |
| Errors | thiserror, anyhow |
| Observability | tracing, tracing-subscriber |
| Test-only | rsa, rand |

Dependency posture: Rustls everywhere (no OpenSSL), no ORM beyond sqlx, no web framework beyond Axum, no message-queue client, no floating-point money library.

## External Dependencies (npm)

Grouped from `web/package.json` and `support-console/package.json`; versions in `technology-stack.md`.

| Group | Packages |
| --- | --- |
| UI runtime | react, react-dom, react-router-dom, react-is |
| Data and auth | @tanstack/react-query, @supabase/supabase-js |
| Charts and wallets | recharts, @walletconnect/ethereum-provider |
| Styling | tailwindcss, @tailwindcss/vite |
| PWA and size | vite-plugin-pwa, workbox-build, workbox-window, size-limit |
| Build | vite, @vitejs/plugin-react, typescript |
| Test | vitest, @testing-library/*, jest-axe, jsdom, msw (support console only) |
| Quality | eslint, typescript-eslint, eslint-plugin-react-hooks, prettier |

## External Systems (runtime)

| System | Provided by | Used for |
| --- | --- | --- |
| Supabase Auth | external platform | JWTs, JWKS, MFA, passkeys, password recovery, admin API |
| Supabase Postgres / Storage / Vault | external platform | data, files, secrets |
| Exchange APIs (OKX, Binance, Coinbase, Kraken, Bybit) | third party | read-only balances, prices, pairs, candles |
| EVM / Solana / Bitcoin / Tron RPC and indexers | third party | on-chain balances and token metadata |
| WalletConnect Relay | third party | read-only wallet sessions |
| Market-data and FX providers | third party | secondary prices, long-tail quotes, historical FX |
| Benchmark index provider | third party | performance comparison |
| Transactional email provider | third party | alert/brief/billing email |
| Web-push provider | third party | browser push |
| Merchant of record | third party | **declared only** — no client code exists |
| Help desk and status page | third party | support and incidents |
| Operational alerting channel | third party | Prometheus/Alertmanager routing |

## Dependency Risks

- **Billing/entitlement dependency is missing**: Portfolio, Gateway and every gated action are documented as depending on Billing; no client or service implements it.
- **Database credentials**: every service requires its `DATABASE_URL` from a secret file; a missing secret fails startup loudly (`config.rs` `secret(...)`), which is correct but means no service starts without Vault materialisation.
- **JWKS availability**: a JWKS outage makes every service fail closed after the 10-minute staleness window; this is deliberate but is a single external dependency for the whole platform.
- **No dependency-scanning automation**: the rule "scan dependencies and container images before release" is documented but no scanner is configured.
- **Yarn version drift**: declared `yarn@4.18.1` vs. project memory "Yarn ≥ 6".
