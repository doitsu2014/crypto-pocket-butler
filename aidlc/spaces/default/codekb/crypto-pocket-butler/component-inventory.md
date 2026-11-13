# Component Inventory — Crypto Pocket Butler

One row per component, with its location, responsibility, dependencies and owned data. API detail lives in `api-documentation.md`; versions live in `technology-stack.md`.

### API Gateway (BFF)

- **Location**: `services/gateway`
- **Crates**: `gateway-domain`, `gateway-application`, `gateway-infrastructure`, `gateway-http`, `gateway` (binary: `src/main.rs`)
- **Responsibility**: single edge for the SPA and support console — JWT validation against cached JWKS, deny-by-default route table, admin gate, correlation ids, idempotency keys, bounded upstream retries, fixed-window rate limits, security headers, denial audit events.
- **Depends on**: Supabase JWKS (HTTPS), upstream service base URL (`ASSET_REGISTRY_URL`), Audit service.
- **Owns**: no schema.
- **Current limit**: route table declares only Asset Registry paths (`services/gateway/domain/src/route.rs:68-90`); other services are unreachable through `/v1`.
- **Tests**: `tests/{application,domain,http,jwt,smoke,upstream}.rs`, `tests/common/{fakes,jwt}.rs`.

### Identity Service

- **Location**: `services/identity`
- **Crates**: `identity-domain`, `identity-application`, `identity-infrastructure`, `identity-http`, `identity` (binaries: `src/bin/api.rs`, `src/bin/worker.rs`)
- **Responsibility**: internal users keyed by hashed Supabase subject; roles from `app_metadata.role`; status; locale/base currency/timezone; consent history; device sessions; support-access grants; export and deletion requests; jurisdiction rules; crypto-shredding key references; verified-contact read for Notification; due-user list for Reporting.
- **Depends on**: Supabase Auth admin API, JWKS, Vault (pgsodium), Storage (exports), Account/Ledger/Portfolio (privacy contributions), Audit.
- **Owns**: `identity` schema — `users`, `user_personal`, `shredding_keys`, `consent_records`, `support_access_grants`, `device_sessions`, `export_requests`, `deletion_requests`, `jurisdiction_rules`, plus migrations `0001_init`, `0002_contact`.
- **Tests**: `tests/{application,domain,http,repositories,worker}.rs`, `tests/common/{fakes,mod}.rs`.

### Account Service

- **Location**: `services/account`
- **Crates**: `account-domain`, `account-application`, `account-infrastructure`, `account-http`, `account` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: read-only exchange accounts, watch-only wallets, WalletConnect and CSV imports; verified credentials; holdings sync; spam/dust filtering; privacy export/anonymise contributions; holdings and transaction feeds for Portfolio/Ledger.
- **Depends on**: Asset Registry, Market Data, Notification, Audit, Supabase Vault, exchange/RPC providers.
- **Owns**: `account` schema (accounts, credentials, holdings, imports, sync runs, wallets, privacy).
- **Tests**: crate-level `tests/` plus `infrastructure/tests/{providers,repositories}.rs`.

### Asset Registry Service

- **Location**: `services/asset-registry`
- **Crates**: `asset-registry-domain`, `asset-registry-application`, `asset-registry-infrastructure`, `asset-registry-http`, `asset-registry` (binary: `src/main.rs`)
- **Responsibility**: canonical asset identity, symbols, chains, token contracts, price-source registry, price resolution, observation quarantine for unmapped data.
- **Depends on**: Postgres (`registry` schema), Audit.
- **Owns**: `registry` schema.
- **Tests**: service-level `tests/{application,domain,http,repositories,smoke}.rs`, `tests/common/{fakes,mod}.rs`.

### Market Data Service

- **Location**: `services/market-data`
- **Crates**: `market-data-domain`, `market-data-application`, `market-data-infrastructure`, `market-data-http`, `market-data` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: point-in-time prices, FX, benchmarks; deterministic source priority, stale fallback, freshness, tiered request budget, licensing attribution.
- **Depends on**: Asset Registry, external price/FX providers, Vault, Audit.
- **Owns**: `market` schema (observations, fx, rate buckets, runs, tiers).
- **Tests**: crate-level `tests/` plus `infrastructure/tests/{migrations,repositories}.rs`.

### Portfolio Service

- **Location**: `services/portfolio`
- **Crates**: `portfolio-domain`, `portfolio-application`, `portfolio-infrastructure`, `portfolio-http`, `portfolio` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: portfolios and membership, targets and guardrails, base-currency valuation, drift, immutable snapshots, order intents, trades, performance and attribution; unpriced/unmapped assets labelled and excluded.
- **Depends on**: Account (holdings), Ledger (P&L), Market Data (prices/FX), Identity (preferences/role), Notification, Audit.
- **Owns**: `portfolio` schema (portfolios, membership, targets, guardrails, overlays, intents, trades, snapshots, runs, privacy).
- **Tests**: crate-level `tests/`, `http/tests/{dto,routes}.rs`, `infrastructure/tests/{adapters,repositories}.rs`.

### Ledger Service

- **Location**: `services/ledger`
- **Crates**: `ledger-domain`, `ledger-application`, `ledger-infrastructure`, `ledger-http`, `ledger` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: append-only transactions, imports, dedupe, transfer pairing, FIFO/average cost basis, net invested, realized/unrealized P&L, reconciliation, corrections, privacy anonymise.
- **Depends on**: Account (transaction feed), Identity, Report/Audit, Postgres.
- **Owns**: `ledger` schema (transactions, corrections, lots, transfer pairs, snapshots, anonymisations, lease).
- **Tests**: crate-level `tests/`, `application/tests/{application,fakes}.rs`, `domain/tests/domain.rs`, `http/tests/http.rs`.

### Reporting Service

- **Location**: `services/reporting`
- **Crates**: `reporting-domain`, `reporting-application`, `reporting-infrastructure`, `reporting-http`, `reporting` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: reporting preferences, daily/weekly brief composition from Portfolio/Ledger/Identity, delivery, idempotency per owner/portfolio/period, privacy contributions.
- **Depends on**: Portfolio, Ledger, Identity, Notification, Audit, Storage, Postgres.
- **Owns**: `reporting` schema (briefs, deliveries, runs, privacy).
- **Tests**: `application/tests/generation.rs` plus crate-level suites.

### Notification Service

- **Location**: `services/notification`
- **Crates**: `notification-domain`, `notification-application`, `notification-infrastructure`, `notification-http`, `notification` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: alert catalogue, per-user/per-channel preferences, quiet hours, digests, dedupe, delivery log, bounded retries, email and web-push dispatch, privacy anonymise.
- **Depends on**: Identity (verified contact), email/push providers, Vault, Audit, Postgres.
- **Owns**: `notification` schema (alerts, attempts, preferences, control, privacy); migrations `0001_init`, `0002_alert_channel`.
- **Tests**: crate-level `tests/`, `http/tests/http.rs`, `infrastructure/tests/repositories.rs`.

### Analytics Service

- **Location**: `services/analytics`
- **Crates**: `analytics-domain`, `analytics-application`, `analytics-infrastructure`, `analytics-http`, `analytics` (binaries: `api`, `worker`, `migrate`)
- **Responsibility**: consent-aware event ingestion, activation/retention/conversion aggregates, feature flags and experiment exposure, retention and privacy erase.
- **Depends on**: Identity (consent), Audit, Postgres.
- **Owns**: `analytics` schema (events, aggregates, flags, consents, pause, privacy).
- **Tests**: crate-level `tests/`, `http/tests/http.rs`, `infrastructure/tests/repositories.rs`.

### Job Orchestration Service

- **Location**: `services/job-orchestration`
- **Crates**: `job-orchestration-domain`, `job-orchestration-application`, `job-orchestration-infrastructure`, `job-orchestration-http`, `job-orchestration` (binary: `src/bin/api.rs`; migrations via `Dockerfile.migrate`)
- **Responsibility**: self-ticking scheduler elected by a single lease row (30 s TTL), fixed job catalogue, run records, bounded retries, missed-window recovery (24 h lookback), operator backfills, heartbeat and failure metrics.
- **Depends on**: every service's C11 run endpoint (service token), Audit, Postgres.
- **Owns**: `job_orchestration` schema (runs, lease, heartbeat, job definitions).
- **Tests**: `application/tests/behaviour.rs`.
- **Operational contract**: `infra/prometheus/job-orchestration-rules.yml` alerts on heartbeat age > 180 s (including `absent()`).

### Audit Service

- **Location**: `services/audit`
- **Crates**: `audit-domain`, `audit-application`, `audit-infrastructure`, `audit-http`, `audit` (binaries: `api`, `retention`)
- **Responsibility**: append-only audit store enforced by an insert-only Postgres role and insert-shape triggers; admin queries; support-access records; privacy exports to Storage; erasure by crypto-shredding; partition retention through a separate privileged role.
- **Depends on**: Postgres (`audit` schema), Storage, Vault.
- **Owns**: `audit` schema (events, append idempotency, support access records, exports, privacy deletions, retention runs).
- **Tests**: `tests/{application,domain,http,repositories,smoke}.rs`.

### React Web Client (SPA)

- **Location**: `web/`
- **Responsibility**: authenticated user journeys — sign-in/verify-email, onboarding, connect accounts, dashboard, portfolios, targets, order intents, trades, history, briefs, notifications, settings/MFA, privacy, activity, help; PWA shell; en/vi locales.
- **Depends on**: API Gateway via `VITE_GATEWAY_URL`, Supabase Auth directly (allowed exception), WalletConnect directly (read-only), Parchment Ledger tokens.
- **Owns**: no data; session tokens in memory only.
- **Structure**: `src/api/` (single gateway client), `src/auth/`, `src/app/` (router/shell), `src/features/`, `src/components/`, `src/i18n/`, `src/pwa/`, `src/test/`.
- **Tests**: 30+ `*.test.ts(x)` files, Vitest + Testing Library + jest-axe.

### Support Console

- **Location**: `support-console/`
- **Responsibility**: least-privilege support UI — grant request/approval/banner, account metadata search and panel, ticket queue, incident console (admin-only section), help panel, status footer; role- and grant-aware UI.
- **Depends on**: API Gateway (support paths), Supabase Auth directly.
- **Owns**: no schema.
- **Structure**: `src/{api,auth,components,domain,hooks,pwa,screens}/`.
- **Tests**: component and domain tests with MSW; 20+ `*.test.ts(x)` files.

### Shared Four-Crate Architecture

- **Location**: every `services/<service>/` package
- **Responsibility**: enforces the dependency rule inside each service — `domain` (entities/rules), `application` (use cases/ports), `infrastructure` (adapters), `http` (Axum/DTOs), plus a composition-root package.
- **Depends on**: nothing shared between services; no shared crate exists.
- **Enforcement**: `scripts/check-boundaries.sh` via `cargo metadata`.
- **Note**: the four crates are structurally parallel per service, not a single shared library.

### Local Docker Compose Stack

- **Location**: `deployment/localhost/`
- **Responsibility**: run the whole system on one machine — self-hosted Supabase (Postgres 15 + pgsodium, GoTrue, PostgREST, Storage, Studio, Mailpit, Kong), every Rust service and worker, schema provisioning, service-token seeding, SPA and support console behind nginx.
- **Depends on**: Docker Compose v2, Node.js for key generation.
- **Owns**: local `.env`, generated `secrets/`, `supabase/kong.generated.yml` (all local-only).
- **Documented gaps**: gateway routes only registry paths; Mailpit is not a compatible email-provider API; no market-data provider configured; Account needs reachable RPC endpoints.

### Swarm Production Stack

- **Location**: `infra/swarm-stack.yml` (1526 lines), `infra/sql/*`, `infra/prometheus/*`
- **Responsibility**: production topology — digest-pinned images per service role (`api`, `worker`, `migrate`, `retention`), replicas, health checks, Docker secrets from Vault, edge routing, schema bootstrap SQL, Prometheus alert rules.
- **Depends on**: Docker Swarm cluster, Supabase Postgres/Auth/Storage/Vault, CDN/WAF.
- **Owns**: deployment configuration only.
- **Note**: production is the only environment (no staging).

### Observability Stack

- **Location**: modelled in `docs/architecture/workspace.dsl`; alert rules in `infra/prometheus/job-orchestration-rules.yml`; `/metrics` on every service
- **Responsibility**: logs (Loki), metrics (Prometheus), traces (Tempo), dashboards and alerts (Grafana/Alertmanager); heartbeat dead-man rule.
- **Depends on**: every service's `/metrics` and trace export.
- **Owns**: no application data.

### Design System

- **Location**: `design-system/`
- **Responsibility**: Parchment Ledger design tokens, colour/typography/spacing rules, component guidance, reference HTML pages, logo assets.
- **Depends on**: nothing.
- **Owns**: static assets and documentation.
- **Rule**: no raw colour, spacing or type values in UI code.

### Documentation Set

- **Location**: `docs/`
- **Responsibility**: 21 BRDs (BRD-001…BRD-021), C4 Structurizr model and README, development standards, project-management notes.
- **Depends on**: nothing at runtime.
- **Owns**: requirements and architecture records.
- **Note**: the C4 model is design intent; it lists containers (notably Billing) that have no code.

### Build and Boundary Tooling

- **Location**: `scripts/check-boundaries.sh`, `tools/preview-architecture.sh`, `Cargo.toml`, `clippy.toml`, `rustfmt.toml`, per-package `package.json`/`vite`/`vitest`/`eslint` configs
- **Responsibility**: workspace membership, lint/format configuration, mechanical crate-boundary enforcement, Structurizr preview helper.
- **Depends on**: `cargo`, `python3`, `docker` (preview only).
- **Owns**: no runtime data.
- **Gap**: no CI workflow consumes these.

### Billing Service (planned, not implemented)

- **Location**: none — declared only in `docs/architecture/workspace.dsl:54` and `docs/architecture/README.md` §5
- **Responsibility (declared)**: plans, subscriptions, entitlements, quotas, invoices, dunning, refunds, merchant-of-record integration; server-side entitlement enforcement.
- **Depends on (declared)**: merchant of record, Postgres `billing` schema.
- **Owns (declared)**: `billing` schema — **no migration, schema file, crate or service exists**.
- **Consequence**: BRD-016 is unimplemented; no entitlement or quota check exists in any service or at the gateway.
