# Developer Code Scan — Crypto Pocket Butler

Full rescan of the project root (`./`) for intent `brd-002-users-roles`.

## Developer Code Scan Results

### Scan Coverage

- **Analyzed deeply**:
  - `Cargo.toml` — root Cargo workspace (12 member service packages, 4 crates each)
  - `Cargo.lock` — resolved versions for axum, tokio, sqlx, tower, reqwest, jsonwebtoken, rust_decimal
  - `package.json`, `web/package.json`, `support-console/package.json` — Yarn 4.18.1 workspace and dependency versions
  - `clippy.toml`, `rustfmt.toml`, `.gitignore`, `.dockerignore` — tooling and ignore rules
  - `services/identity/domain/src/user.rs`, `claims.rs` — user lifecycle, roles, AAL, next-step rules
  - `services/identity/application/src/principal.rs`, `support.rs`, `common.rs` — provisioning, refresh, support grants, error surface
  - `services/identity/http/src/routes.rs`, `middleware.rs` — 21 route declarations, JWT and per-caller service-token middleware
  - `services/identity/infrastructure/src/auth.rs`, `service_token.rs` — JWKS validator, constant-time token registry
  - `services/identity/migrations/0001_init.up.sql`, `infra/sql/identity-schema.sql` — tables, roles, constraints
  - `services/identity/src/config.rs`, `tests/common/mod.rs`, `tests/repositories.rs` — config and test harness
  - `services/gateway/domain/src/{route,principal,method}.rs`, `services/gateway/application/src/authorize.rs` — deny-by-default route table and request path
  - `services/gateway/http/src/{routes,handlers,rate_limit}.rs`, `services/gateway/infrastructure/src/{auth,upstream}.rs`, `services/gateway/src/config.rs`
  - `services/account/http/src/{routes,middleware}.rs`, `services/portfolio/http/src/routes.rs`, `services/portfolio/application/src/common.rs`
  - `services/job-orchestration/domain/src/lib.rs`, `application/src/scheduler.rs`, `http/src/lib.rs`, `src/bin/api.rs`
  - `services/audit/migrations/0001_init.up.sql`, `infra/sql/audit-schema.sql` — insert-only audit roles and trigger guards
  - `web/src/{api/client.ts,auth/session.ts,app/router.tsx}`, `web/vitest.config.ts`, `web/eslint.config.js`, `web/index.html`
  - `support-console/src/screens/{SupportApp,SupportShell}.tsx`, `support-console/package.json`, `support-console/index.html`
  - `docs/architecture/README.md`, `docs/architecture/workspace.dsl` (container declarations), `docs/development/README.md`, `docs/brds/BRD-002-users-roles-and-permissions.md`, `docs/project-management/README.md`
  - `deployment/localhost/README.md`, `deployment/localhost/docker-compose.yml` (service list)
  - `infra/swarm-stack.yml` (header and service list), `infra/prometheus/job-orchestration-rules.yml`
  - `scripts/check-boundaries.sh`, `design-system/DESIGN.md`
  - Route tables of all 12 services, and the full file tree of every `services/*` package
- **Skimmed only**:
  - `services/analytics/`, `services/asset-registry/`, `services/market-data/`, `services/ledger/`, `services/notification/`, `services/reporting/` — route tables, crate layout, representative domain/application/ports files; not every handler or repository
  - `web/src/features/**`, `web/src/components/**`, `web/src/i18n/**` — implementation bodies beyond the router, API client, types and auth surface
  - `support-console/src/components/**`, `support-console/src/domain/**` — component bodies beyond the shell and grant flow
  - `infra/swarm-stack.yml` beyond the build header and service list; `deployment/localhost/scripts/**`
  - `docs/brds/**` other than BRD-002; `docs/architecture/workspace.dsl` beyond container declarations
  - `design-system/*.html`, `design-system/tokens.css`, `supabase/config.toml`, `.env.example`
- **Left out** (build output, dependencies, caches, tool installs):
  - `target/` (Rust build output)
  - `web/node_modules/`, `support-console/node_modules/`, `web/dist/`, `support-console/dist/`, `.yarn/`
  - `docs/architecture/workspace.json`, `docs/architecture/.structurizr/` (generated Structurizr artefacts)
  - `.git/`, `.DS_Store`, `tsconfig.tsbuildinfo`
  - `.claude/` and `aidlc/` (AI-DLC's own install and workspace, not project code)

### Packages Found

- `crypto-pocket-butler` (Cargo workspace) — virtual manifest — Rust — members: `services/{account,analytics,asset-registry,audit,gateway,identity,job-orchestration,ledger,market-data,notification,portfolio,reporting}` plus each service's `domain`, `application`, `infrastructure`, `http` crates
- `crypto-pocket-butler-web` — frontend application — TypeScript/React — SPA for onboarding, dashboard, accounts, portfolios, valuation, P&L, snapshots, orders, briefs, notifications, settings, privacy
- `crypto-pocket-butler-support-console` — frontend application — TypeScript/React — least-privilege support UI for grants, account metadata, tickets, incidents
- `design-system` — static assets/documentation — HTML/CSS/Markdown — Parchment Ledger tokens and reference pages
- `deployment/localhost` — Docker Compose stack — YAML/SQL/Shell — self-hosted Supabase + every service for local development
- `infra` — production deployment — YAML/SQL — Docker Swarm stack, schema bootstraps, Prometheus rules
- `scripts`, `tools` — shell tooling — Bash — crate-boundary gate and Structurizr preview

### Build System

- **Type**: Cargo workspace (Rust, edition 2021, `rust-version = "1.85"`, `resolver = "2"`) + Yarn 4.18.1 workspaces per frontend app
- **Config Files**: `Cargo.toml`, `Cargo.lock`, `clippy.toml`, `rustfmt.toml`, `package.json`, `web/{package.json,tsconfig.json,vite.config.ts,vitest.config.ts,eslint.config.js,.prettierrc.json,.yarnrc.yml}`, `support-console/{...same...}`, `scripts/check-boundaries.sh`
- **Build Dependencies**: every service `http`/`infrastructure` crate → its own `application` → its own `domain`; the service package composes all four. No cross-service crate dependencies (services talk over HTTP).

### APIs Discovered

- HTTP/JSON (Axum) — 12 service routers plus the gateway edge router — 135 `.route(...)` declarations across the routers (including `/healthz`, `/readyz`, `/metrics` on every service)
- Gateway edge — `services/gateway/http/src/routes.rs` — 3 local routes plus a fallback proxy governed by an 8-entry deny-by-default route table in `services/gateway/domain/src/route.rs`
- Internal service contracts (C-prefixed in code comments): C1 user JWT surface, C2/C3 support-check and verified-contact reads, C5/C8 holdings and snapshot reads, C10 audit append, C11 run endpoints
- RFC 7807 problem+json error envelope on every service (`ProblemDetails` in each `http/src/problem.rs`)
- Prometheus text metrics at `/metrics` on every service; `/healthz` and `/readyz` probes

### Frameworks & Libraries

Rust (resolved in `Cargo.lock`): axum 0.8.9, tokio 1.54.1, sqlx 0.8.6 (postgres, chrono, rust_decimal, uuid, json, migrate, macros), tower 0.5.3, tower-http 0.6, reqwest 0.12.28 (rustls), jsonwebtoken 9.3.1, rust_decimal 1.43.0, serde 1.0.229, chrono 0.4.45, uuid 1.29.0, tracing 0.1.44, thiserror 2, anyhow 1, async-trait 0.1, sha2 0.10, base64 0.22, rsa 0.9 (test-only), rand 0.8 (test-only).
TypeScript/React: react 18.3.1, react-dom 18.3.1, typescript ^5.7.2, vite ^6.0.7, @tanstack/react-query ^5.62.11, @supabase/supabase-js ^2.47.10, react-router-dom ^6.30.1, recharts ^3.10.1, tailwindcss ^4.0.0, @tailwindcss/vite ^4.0.0, vitest ^3.0.0, @testing-library/react ^16.1.0, jest-axe ^10.0.0, msw ^2.7.0 (support console), @walletconnect/ethereum-provider ^2.26.0, vite-plugin-pwa ^2.0.0, size-limit ^14.3.0.

### Test Coverage

- **Test Directories**: `services/<svc>/{domain,application,http,infrastructure}/tests/*.rs`, service-level `services/<svc>/tests/*.rs` (asset-registry, audit, gateway, identity); `web/src/**/*.test.{ts,tsx}`, `support-console/src/**/*.test.{ts,tsx}`
- **Test Frameworks**: Rust built-in `#[test]`/`#[tokio::test]` with hand-written port fakes (`tests/fakes.rs`, `tests/common/fakes.rs`); Vitest + Testing Library + jest-axe + MSW
- **Coverage Config**: absent — no tarpaulin/llvm-cov/nextest config, no coverage script, no CI job

### Code Quality Indicators

- **Linting**: `clippy.toml` (msrv 1.85, `too-many-arguments-threshold = 8`) and `rustfmt.toml` (edition 2021, max_width 100); ESLint 9 flat config + Prettier 3 in both frontend packages; `scripts/check-boundaries.sh` parses `cargo metadata` and fails on outward crate dependencies
- **CI/CD**: **absent** — there is no `.github/` directory and no GitLab/Jenkins/CircleCI/Azure/Buildkite configuration anywhere in the repository. The documented gates (`cargo fmt`, `cargo clippy -D warnings`, tests, boundary check, 80% coverage, image scanning) are described in `docs/development/README.md` but not wired to a pipeline file.
- **Documentation**: 21 BRDs (BRD-001…BRD-021), a C4 Structurizr model (`workspace.dsl` + `docs/architecture/README.md`), a development-standards document, and a design system. Rust public items carry `///` doc comments with `# Errors`/`# Panics` sections; modules carry `//!` headers that cite BR ids. Documentation is unusually strong relative to code volume.

### Technical Debt Signals

- `services/billing/` **does not exist**, although the C4 model lists a Billing Service (`docs/architecture/workspace.dsl:54`) and `docs/architecture/README.md` §5 assigns it the `billing` schema. No entitlement/quota code exists in any service (`grep -ri 'entitlement\|quota' services --include='*.rs' | grep -v tests` → 0 hits). BRD-016 is unimplemented.
- The gateway route table (`services/gateway/domain/src/route.rs:68-90`) declares **only Asset Registry paths**. Identity, Account, Portfolio and every other service are reachable only on their own published ports, not through `/v1`. The local stack README records this as a known gap (`deployment/localhost/README.md` §"Known gaps").
- The gateway does **not** resolve principals via Identity and does **not** check entitlements, despite §6 of the architecture README describing both. `services/gateway/application/src/authorize.rs` validates the JWT, checks `requires_admin`, and forwards; there is no Identity client in `services/gateway/infrastructure/`.
- Gateway rate limiting is in-memory and keyed on the forwarded client IP plus bearer-token presence (`services/gateway/http/src/rate_limit.rs:1-6`); the documented per-user/per-region dimensions are not implemented (the comment says the edge/CDN supplies the regional dimension).
- Service package layout is inconsistent: `asset-registry` and `gateway` use `src/main.rs`; `audit` uses `src/bin/{api,retention}.rs`; `identity` uses `src/bin/{api,worker}.rs`; `job-orchestration` uses only `src/bin/api.rs`; the other seven use `src/bin/{api,migrate,worker}.rs`. There is no single documented runnable-unit convention.
- Yarn version vs. documented rule: `package.json` pins `yarn@4.18.1`, while project memory says "Yarn ≥ 6". Nothing in the repository resolves that mismatch.
- Test integration suites that need Postgres are `#[ignore]`d by default (`services/identity/tests/common/mod.rs:1-4`); nothing runs them.
- 0 `TODO`/`FIXME`/`HACK`/`XXX` markers in Rust or TypeScript source — debt is structural (missing service, missing routing, missing CI), not annotated.
- Every `*.up.sql` has a matching `*.down.sql` (13/13), so the migration-pair rule is met.

## Handoff Summary

- **Intent-relevant finding**: The identity surface for BRD-002 is implemented and tested directly on the Identity service — users are provisioned just-in-time from `VerifiedClaims` (`services/identity/application/src/principal.rs:61-86`), roles come only from `app_metadata.role` with unknown values falling back to `user` (`services/identity/domain/src/claims.rs:19-31`), status is `pending|active|deleted` (`services/identity/domain/src/user.rs:14-51`), support grants are request/approve/revoke/check with a 60-minute window and distinct-approver constraint (`services/identity/application/src/support.rs:39-130`, `services/identity/migrations/0001_init.up.sql:107-133`), and each user path enforces the path subject against the validated token (`services/identity/http/src/handlers.rs:90`). The gap is at the edge: the gateway route table does not yet route `/v1/principals`, `/v1/consent`, `/v1/users/...` or `/v1/support-grants` (`services/gateway/domain/src/route.rs:68-90`), so BRD-002 behaviour is reachable only by calling Identity's published port directly. Note also that BRD-002's `disabled` status and `pending_deletion` status (FR-USR-20) are not represented — the stored enum is `pending|active|deleted` only.
- **Risks / follow-up**:
  - BRD-016 (billing/entitlements/quotas) has no service, no schema, and no enforcement code; the architecture documents it as present. Any downstream stage that assumes entitlement gating will over-read the implemented system.
  - The gateway is the single documented entry point but currently forwards only Asset Registry routes; adding identity routes is a pre-condition for SPA sign-in/settings/support journeys through `/v1`.
  - No CI exists, so the documented quality gates (fmt, clippy `-D warnings`, tests, boundary check, 80% coverage, image scanning) are unenforced by automation.
  - Support Console gating is client-side role gating (`support` vs `admin` sections); server-side support access depends on the Identity grant-check path and Audit records.
  - 21 BRDs describe a much larger system than the 11 implemented services; treat BRD coverage claims in the architecture README as design intent, not as implemented status.
