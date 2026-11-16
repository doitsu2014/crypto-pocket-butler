## Developer Code Scan Results

### Scan Coverage
- **Analyzed deeply**:
  - `Cargo.toml` — root workspace membership, shared dependency versions and toolchain.
  - `package.json`, `web/package.json` — Yarn scripts and frontend dependencies.
  - `web/src/auth/session.ts` — browser session state, Supabase operations and token exposure.
  - `web/src/auth/storage.ts` — memory-only auth storage and PKCE verifier exception.
  - `web/src/auth/client.ts` — Supabase client configuration, PKCE flow and auto-refresh.
  - `web/src/auth/session.test.ts` — session store test coverage patterns.
  - `web/src/features/auth/SignInPage.tsx` — sign-in/sign-up UI flow and handling.
  - `web/src/api/client.ts` — gateway API client, bearer-token forwarding, errors and signup gate routes.
  - `services/gateway/domain/src/route.rs` — deny-by-default gateway route declarations, identity routes and per-route budgets.
  - `services/gateway/infrastructure/src/auth.rs` — Supabase JWT/JWKS validation, issuer/audience/expiry checks and key cache behavior.
  - `services/identity/Cargo.toml` — Identity package composition and executable targets.
  - `services/identity/http/src/routes.rs` — public, user-authenticated and service-token-protected Identity routes.
  - `services/identity/http/src/middleware.rs` — bearer/JWT checks, trusted request metadata, correlation and security headers.
  - `services/identity/application/src/sessions.rs` — device-session list/revoke use cases and auditing.
  - `services/identity/tests/http.rs` — HTTP-level auth, session revocation, owner checks, signup gate and error contracts.
  - `services/identity/tests/authorisation_adapters.rs` — Supabase role adapter and audit adapter tests.
  - `services/identity/migrations/0001_init.up.sql` through `0008_support_mine_index.up.sql` and matching down migrations (file inventory; migration SQL not individually read).
  - `docs/brds/BRD-003-authentication-and-session-management.md` — active intent requirements, rules, acceptance criteria and unresolved questions.
  - `.gitignore` — ignored build, dependency, generated and AI-DLC content.
  - `aidlc/spaces/default/knowledge/aidlc-shared/{domain-glossary,problem-details,service-map,stack-facts}.md` — project-auth and architecture context.
  - `aidlc/spaces/default/knowledge/aidlc-developer-agent/{rust-conventions,sqlx-migrations,test-layout}.md` — implementation conventions and test practices.
- **Skimmed only**:
  - `services/` — package/Cargo manifest, HTTP route, test and migration file inventories; additional services and gateway composition not individually read.
  - `web/src/` and `support-console/src/` — test file inventory; most UI, API and auth implementations not individually read.
  - `deployment/`, `infra/`, `supabase/`, `scripts/`, `docs/`, `design-system/` — directory/file inventories and supplied project architecture context; most implementation files not individually read.
  - Repository root — build/config file inventory, including absence of root CI workflow paths in the inspected glob results.
- **Left out**:
  - `target/`, `**/target/`, `node_modules/`, `web/dist/`, `support-console/dist/`, `web/tsconfig.tsbuildinfo`, `support-console/tsconfig.tsbuildinfo`, `support-console/.yarn/install-state.gz`, `web/e2e/.artifacts/`, `web/e2e/screenshots/`, `web/e2e/.playwright/` — ignored build, dependency or generated artifacts per `.gitignore`.
  - `docs/architecture/workspace.json`, `docs/architecture/.structurizr/` — generated Structurizr output ignored by `.gitignore`.
  - `.claude/` and `aidlc/` — AI-DLC installation/workspace, excluded from project-code analysis except the explicitly supplied developer template, scan guide, stage context and active state needed for this handoff.
  - `deployment/localhost/.gitignore`, `docs/architecture/.gitignore`, `supabase/.gitignore` — noted but not opened; any directories they exclude were not inspected.

### Packages Found
- `crypto-pocket-butler` — multi-package Cargo workspace — Rust 2021 — backend services, API Gateway and worker/migration executables.
- `identity` — service package plus `identity-domain`, `identity-application`, `identity-infrastructure`, `identity-http` — Rust — user registry, authorization, sessions, preferences/consent and lifecycle APIs, workers and migrations.
- `gateway` — service package plus four architecture crates — Rust — public API routing, rate limits, JWT validation and upstream forwarding.
- `account`, `analytics`, `asset-registry`, `audit`, `job-orchestration`, `ledger`, `market-data`, `notification`, `portfolio`, `reporting` — service packages, generally with domain/application/infrastructure/http crates — Rust — independent service/API and worker responsibilities (see service map in supplied workspace knowledge).
- `crypto-pocket-butler-web` — Yarn package — TypeScript/React — primary SPA, sign-in and user flows.
- `support-console` — Yarn package — TypeScript/React — separate support/admin frontend.

### Build System
- **Type**: Cargo workspace for Rust; Yarn 4.18.1 packages for the root, `web/` and `support-console/`; Dockerfiles and Compose/Swarm deployment definitions.
- **Config Files**: `Cargo.toml`; per-service and per-crate `Cargo.toml` files; root, `web/` and `support-console/` `package.json`; associated Yarn lockfiles; `rustfmt.toml`; `clippy.toml`; `web/vite.config.ts` and `support-console/vite.config.ts`; Dockerfiles; `deployment/localhost/docker-compose.yml`; `infra/swarm-stack.yml`.
- **Build Dependencies**: root Cargo workspace includes all service packages/crates (`Cargo.toml:7-70`); each service composition package depends on its own four crates (Identity example: `services/identity/Cargo.toml:26-30`); frontends are separate Yarn packages and not Cargo workspace members; services own their schemas and migrations.

### APIs Discovered
- REST — `services/identity/http/src/routes.rs:38-255` — signup gate, principal resolution, user sessions (`GET /v1/users/{userId}/sessions`, `DELETE /v1/users/{userId}/sessions/{sessionId}`), export, privacy, support grants, preferences/consent, internal user registry, authorization, lifecycle and jurisdiction endpoints. Routes are separated into public, user-JWT and service-token protected groups.
- REST — `services/gateway/domain/src/route.rs:75-182` — explicit deny-by-default public `/v1` allowlist forwarding to Asset Registry and Identity. Includes principal/user lookup, admin user operations, preferences/consent, support grants, account/profile/deletion and export routes; explicit rate limits are declared for selected Identity routes.
- REST — `web/src/api/client.ts:325-391, 463-487, 501-546` — SPA `GatewayClient` sends bearer token and correlation ID, and exposes signup gate, profile, MFA enrollment/recovery and consent operations. Browser API calls target the gateway; Supabase Auth is used directly for authentication.
- External auth API — `@supabase/supabase-js` configured in `web/src/auth/client.ts:32-44` for PKCE, auto-refresh and custom storage; sign-in/password, sign-up and sign-out operations in `web/src/auth/session.ts:106-140`.
- REST — additional service routes are present under `services/*/http/src/routes.rs`; exact endpoint counts were not derived for services outside Identity and Gateway.

### Frameworks & Libraries
- Rust 2021 — edition and MSRV 1.85 (`Cargo.toml:72-77`); Axum 0.8, tower 0.5, sqlx 0.8, Tokio 1, reqwest 0.12, jsonwebtoken 9, serde, rust_decimal (`Cargo.toml:79-125`).
- React 18.3.1, TypeScript 5.7, Vite 6, TanStack Query 5, React Router 6, Recharts 3, Supabase JS 2, Tailwind CSS v4 (`web/package.json:40-80`; project facts also identify stack).
- Vitest 3, Testing Library, jest-axe, MSW 2, Playwright, ESLint 9, Prettier 3 (`web/package.json:50-80`; test setup shared by project packages per `stack-facts.md:19-27`).
- Yarn 4.18.1 — pinned via `packageManager` in root and web manifests (`package.json:1-5`, `web/package.json:1-6`).
- Supabase Postgres and pgsodium; Docker Swarm for production and Docker Compose for local development (supplied `stack-facts.md:40-47`).

### Test Coverage
- **Test Directories**: `services/identity/tests/`, other service `services/*/tests/` and crate-level test trees, `web/src/` colocated `*.test.ts(x)`, `support-console/src/` colocated tests.
- **Test Frameworks**: Rust `#[test]`/`#[tokio::test]`, Axum router tests via `tower::ServiceExt::oneshot`, port fakes and adapter integration tests; Vitest with Testing Library and jest-axe; MSW for support-console gateway mocks; Playwright package scripts exist for web journeys.
- **Coverage Config**: No coverage config was confirmed in the inspected application manifests. Project knowledge states `cargo llvm-cov` is the Rust coverage tool and 80% per-crate is the affirmed target; no TypeScript coverage threshold is set. No test/build commands were run.
- **Relevant examples**: `web/src/auth/session.test.ts:22-77` covers session loading, provider sign-out, unconfigured auth and signup gate metadata; `services/identity/tests/http.rs:115-149` rejects missing/invalid JWTs and checks correlation; `services/identity/tests/http.rs:289-329` checks own-session listing and revocation confirmation; `services/identity/tests/authorisation_adapters.rs:124-197, 264-372` checks provider role and audit adapter behavior.

### Code Quality Indicators
- **Linting**: Rust `rustfmt.toml`, `clippy.toml`; frontend ESLint flat config and Prettier configuration are declared in package/workspace materials. Canonical commands in project memory: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `yarn lint`, `yarn format:check`, `bash scripts/check-boundaries.sh`.
- **CI/CD**: No `.github/workflows/*` files were found by the workflow-path inventory; no CI was confirmed in the scanned project knowledge. Deployment configuration exists at `infra/swarm-stack.yml` and `deployment/localhost/docker-compose.yml`; no pipeline implementation was inspected.
- **Documentation**: BRDs are under `docs/brds/`, architecture/development documentation under `docs/architecture/` and `docs/development/`. Rust modules include purpose/BR comments, e.g. `services/gateway/infrastructure/src/auth.rs:1-5` and `services/identity/application/src/sessions.rs:1`.
- **Architecture/quality controls**: Service-level four-crate Clean Architecture and `scripts/check-boundaries.sh` are documented. Gateway and Identity route/middleware boundaries explicitly separate user JWTs from service tokens. Identity tests cover ownership, role denial, malformed claims, provider outage and RFC 7807 responses.

### Technical Debt Signals
- The BRD explicitly leaves authentication policy decisions open: production token lifetimes, identity-provider-initiated logout, sustained provider-outage fallback, phone OTP launch need, and recovery when both email and device are lost (`docs/brds/BRD-003-authentication-and-session-management.md:76-85`). These directly affect session lifecycle and recovery behavior.
- The current browser storage adapter intentionally persists the PKCE code verifier in `sessionStorage`, while access/refresh tokens remain memory-only (`web/src/auth/storage.ts:1-7, 21-48`). Maintain the exception narrowly and verify provider/browser callback behavior against FR-AUTH-02 and FR-AUTH-21.
- `SignInPage` currently implements email/password only and sets signup jurisdiction/age values to fixed values (`web/src/features/auth/SignInPage.tsx:50-60, 73-81`); compare this existing surface with requirements for OAuth, MFA/passkeys, explicit age/terms/jurisdiction and account recovery before treating it as complete.
- Identity auth middleware maps invalid token errors to 401, but formats upstream-unavailable detail into a 503 problem (`services/identity/http/src/middleware.rs:200-268`); ensure client-visible errors remain non-revealing as required by FR-AUTH-11 and BR-AUTH-11.
- Several service-local problem/DTO structures are intentionally repeated rather than shared (workspace guidance in `problem-details.md:6-9`); changes to auth error contracts should preserve consistent behavior across boundaries.
- The existing service/package forest has broad route surface and mostly per-service tests, but this scan did not inspect every service or run checks. Exact cross-service route parity, full coverage, and dependency health remain unverified.

## Handoff Summary
- **Intent-relevant finding**: The SPA already uses Supabase Auth with PKCE and an in-memory token store; only the PKCE verifier uses `sessionStorage` (`web/src/auth/client.ts:32-44`, `web/src/auth/storage.ts:1-7, 21-48`). `GatewayClient` reads the current access token and sends it as a bearer token (`web/src/api/client.ts:325-355`), while Gateway validates Supabase JWT signature, issuer, audience and expiry through cached JWKS (`services/gateway/infrastructure/src/auth.rs:1-5, 235-284`). Identity provides JWT-protected device session list/revoke routes (`services/identity/http/src/routes.rs:46-74`) and its use case calls Supabase Auth revocation before persisting the revoked session and audit event (`services/identity/application/src/sessions.rs:67-131`). The BRD requirements and acceptance criteria are at `docs/brds/BRD-003-authentication-and-session-management.md:8-85`.
- **Risks / follow-up**: Decide the five BRD open questions before finalizing session/recovery behavior (`docs/brds/BRD-003-authentication-and-session-management.md:76-85`). Confirm signup age/jurisdiction/terms values are supplied from real user choices rather than current hard-coded values (`web/src/features/auth/SignInPage.tsx:73-81`). The scan was read-only and selective; unrelated service implementations and actual runtime/test outcomes were not verified.

## Snapshot
- **Repo**: `crypto-pocket-butler`
- **Snapshot paths**: `./`
- **Snapshot head commit**: `112b3541355aa1c38d9370d49142a2d3fb9e5866`
- **Store status**: STALE; this brief reflects a full-rescan selection.
- **Constraint**: No tests, builds or programs were run.
