# Project Memory

Rules specific to this project. The most specific layer; it never contradicts
organisation or team memory.

## Project Context

- **Name**: Crypto Pocket Butler
- **Stack**: Rust 2021 backend and API Gateway (Axum, tower, sqlx, rust_decimal, tokio); React 18 + TypeScript 5 frontends (Vite, TanStack Query, Recharts) on Tailwind CSS v4; Supabase Postgres; Docker Swarm
- **Build**: `cargo build --workspace`; `yarn build` in `web/` and `support-console/`
- **Lint**: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `yarn lint` and `yarn format:check` in `web/` and `support-console/`
- **Test**: `cargo test --workspace` (the `#[ignore]` adapter and migration suites need a disposable Postgres via `TEST_DATABASE_URL` and are a blocking gate); `yarn test` in `web/` and `support-console/`; `bash scripts/check-boundaries.sh`

## Project Practices

- Explain why before what in commit messages.
- Keep changes scoped to one unit of work.

## Known Constraints

- (record project-specific constraints here)

## Mandated

- ALWAYS use Rust with Axum, tower, sqlx, rust_decimal, and tokio for backend services and the API Gateway. (`.claude/rules/development.md` — stack fixed, do not substitute) (affirmed 2026-11-05)

- ALWAYS keep Rust dependencies pointing inward: `http`/`infrastructure` → `application` → `domain`; `domain` imports no sibling crate or framework; `application` never imports `infrastructure` or `http`; nothing imports `http`. (`.claude/rules/development.md`, `docs/development/README.md`) (affirmed 2026-11-05)

- ALWAYS express cross-boundary dependencies as port traits defined inward and implemented outward. (`.claude/rules/development.md`) (affirmed 2026-11-05)

- ALWAYS map DTOs to/from domain types at the `http` boundary. (`.claude/rules/development.md`) (affirmed 2026-11-05)

- ALWAYS use `rust_decimal` for quantities and values; never binary floating point. (`.claude/rules/development.md`, `docs/architecture/README.md` — decimal arithmetic kept on purpose) (affirmed 2026-11-05)

- ALWAYS use React 18 + TypeScript 5 with Vite, TanStack Query, `@supabase/supabase-js`, Recharts, and Tailwind CSS v4 mapped to Parchment Ledger tokens for frontend work. (`.claude/rules/development.md`) (affirmed 2026-11-05)

- ALWAYS give each service one Supabase Postgres schema and one Postgres role, and read another service's data only through that service's HTTP API. (`.claude/rules/development.md`, `docs/architecture/README.md` §7) (affirmed 2026-11-05)

- ALWAYS route SPA calls through the API Gateway; the only direct browser-external calls are Supabase Auth for sign-in and WalletConnect for read-only sessions. (`.claude/rules/development.md`, `docs/architecture/README.md` §6) (affirmed 2026-11-05)

- ALWAYS forward the user's Supabase JWT unchanged from the gateway, and ALWAYS validate it in every service against Supabase JWKS (signature, issuer, audience, expiry). (`.claude/rules/development.md`, BRD-012 NFR-SEC-09) (affirmed 2026-11-05)

- ALWAYS use synchronous internal HTTP with bounded retries and idempotency keys for service-to-service calls. (`.claude/rules/development.md`, `docs/architecture/README.md` §6) (affirmed 2026-11-05)

- ALWAYS store secrets in Supabase Vault (pgsodium) or project secrets. (`.claude/rules/development.md`, BRD-012 NFR-SEC-15) (affirmed 2026-11-05)

- ALWAYS deploy every service and the observability stack as replicated Docker Swarm services. (`.claude/rules/development.md`, `docs/development/README.md`) (affirmed 2026-11-05)

- ALWAYS compute valuation, drift, allocation, and other business math in the backend. (`.claude/rules/development.md`) (affirmed 2026-11-05)

- ALWAYS use Parchment Ledger design tokens; no raw colour, spacing, or type values. (`.claude/rules/development.md`, `design-system/DESIGN.md`) (affirmed 2026-11-05)

- ALWAYS enforce ownership checks on every read and write of user-owned resources. (BRD-012 NFR-SEC-07) (affirmed 2026-11-05)

- ALWAYS keep exchange integrations read-only and verify key permissions before use and periodically. (BRD-012 NFR-SEC-05/06) (affirmed 2026-11-05)

- ALWAYS require MFA for users who have exchange credentials. (BRD-012 NFR-SEC-11) (affirmed 2026-11-05)

- ALWAYS keep browser session tokens in memory only. (BRD-012 NFR-SEC-10) (affirmed 2026-11-05)

- ALWAYS rate-limit public endpoints per user/IP/region at the gateway, with stricter limits for auth and manual triggers. (BRD-012 NFR-SEC-12) (affirmed 2026-11-05)

- ALWAYS use TLS for all production traffic. (BRD-012 NFR-SEC-01) (affirmed 2026-11-05)

- ALWAYS record security-sensitive events with actor (or system), action, target, timestamp, context, and outcome. (BRD-012 audit logging, NFR-SEC-13) (affirmed 2026-11-05)

- ALWAYS keep the audit schema append-only through an insert-only Postgres role. (`docs/architecture/README.md` §7) (affirmed 2026-11-05)

- ALWAYS scan dependencies and container images for vulnerabilities before release. (BRD-012 NFR-SEC-14) (affirmed 2026-11-05)

- ALWAYS give every new service or component at least one health metric and one error rate metric. (`memory/phases/operation.md`) (affirmed 2026-11-05)

- ALWAYS include rollback steps in every deployment procedure, and a smoke test or health check for production deployments. (`memory/phases/operation.md`) (affirmed 2026-11-05)

- ALWAYS ship schema changes as versioned sqlx up/down migration pairs, and apply the matching down migration when a release is rolled back. (Interview Q7) (affirmed 2026-11-05)

- ALWAYS quantify SLOs with a percentage and a time window, and set alert thresholds below the SLO breach. (`memory/phases/operation.md`) (affirmed 2026-11-05)

- ALWAYS include escalation paths and contacts in runbooks, and write a post-incident review for every P1/P2 incident. (`memory/phases/operation.md`) (affirmed 2026-11-05)

- ALWAYS require a human review before merge. (`memory/org.md`) (affirmed 2026-11-05)

- ALWAYS link every pull request to its requirement or unit id. (`memory/team.md`) (affirmed 2026-11-05)

- ALWAYS keep a change scoped to one unit of work. (`memory/project.md`) (affirmed 2026-11-05)

- ALWAYS explain why before what in commit messages. (`memory/project.md`) (affirmed 2026-11-05)

- ALWAYS write acceptance criteria in Given/When/Then form. (`memory/phases/inception.md`) (affirmed 2026-11-05)

- ALWAYS document at least two alternatives in an architecture decision, and include Context, Decision, Consequences, and Alternatives Rejected in every ADR. (`memory/phases/inception.md`) (affirmed 2026-11-05)

- ALWAYS handle errors at integration boundaries and surface or log them, distinguishing recoverable from fatal errors. (`memory/phases/construction.md`) (affirmed 2026-11-05)

- ALWAYS cover the happy path and at least two error or edge cases in every test file. (`memory/phases/construction.md`) (affirmed 2026-11-05)

- ALWAYS write a failing test before the implementation for all production code (TDD), and land the test in the same change as the code it covers. (Interview Q3) (affirmed 2026-11-05)

- ALWAYS keep line coverage at or above 80% per Rust crate. (Interview Q4) (affirmed 2026-11-05)

- ALWAYS run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` for Rust, and ESLint plus Prettier through Yarn for TypeScript. (Interview Q8) (affirmed 2026-11-05)

- ALWAYS validate and sanitize inputs at system boundaries. (`memory/phases/construction.md`) (affirmed 2026-11-05)

- ALWAYS update the C4 model and `docs/architecture/README.md` together when a container, schema, external system, or relationship changes. (`docs/architecture/README.md` §13) (affirmed 2026-11-05)

- ALWAYS validate the Structurizr model before review. (`docs/architecture/README.md` §12) (affirmed 2026-11-05)

- ALWAYS run locally the same build, lint, and test commands that CI runs. (`memory/team.md`) (affirmed 2026-11-05)

- ALWAYS define the retention and legal-hold policy, launch jurisdictions, privacy notices, consumer-subscription terms, market-data licences, and incident-response plan before launch. (BRD-012 — "Must define before launch") (affirmed 2026-11-05)

- ALWAYS use Yarn 4.18.1, pinned through corepack (`packageManager: yarn@4.18.1` at the root and in `web/` and `support-console/`), for every JavaScript/TypeScript command. (`.claude/rules/development.md`, `package.json`, `web/package.json`, `support-console/package.json` — supersedes the earlier version rule, which no Yarn release satisfied) (affirmed 2026-11-10)

- ALWAYS deploy with the Docker Swarm rolling update configured in `infra/swarm-stack.yml` — `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, `order: start-first` for API, edge, SPA, retention, job-orchestration and `identity-worker` services, and `order: stop-first` for the remaining background workers — with the service health check gating the update. (Interview Q1, `infra/swarm-stack.yml` — supersedes the earlier defaults wording, which conflicts with the stack's explicit configuration) (affirmed 2026-11-10)

- ALWAYS ship schema changes as versioned sqlx up/down migration pairs, and ALWAYS apply the matching down migration when a release is rolled back — the release job carries the automatic down step. (Interview Q6) (affirmed 2026-11-10)

- ALWAYS measure Rust line coverage with `cargo llvm-cov`. (Interview Q4) (affirmed 2026-11-10)

- ALWAYS provision a disposable Postgres database per gate run and run the `#[ignore]` adapter and migration suites against it as a blocking gate, with `TEST_DATABASE_URL` pointing at that database. (Interview Q5) (affirmed 2026-11-10)

- ALWAYS keep the canonical build, lint and test command list in `aidlc/spaces/default/memory/project.md` as the single authoritative list. (Interview Q7) (affirmed 2026-11-10)

## Forbidden
- NEVER use a backend language other than Rust or an ORM other than sqlx. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER let `domain` import a sibling crate or framework — no Axum, sqlx, reqwest, or serde transport types. (`.claude/rules/development.md`, `docs/development/README.md`) (affirmed 2026-11-05)
- NEVER let `application` import `infrastructure` or `http`, and NEVER let any crate import `http`. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER leak DTO types into `application` or `domain`. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER use npm, npx, pnpm, or their lockfiles. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER grant cross-schema access, and NEVER let a service read another service's tables. (`.claude/rules/development.md`, `docs/architecture/README.md` §7) (affirmed 2026-11-05)
- NEVER use Row Level Security in services. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER use message queues. (`.claude/rules/development.md` — "No message queues"; `docs/architecture/README.md` §6) (affirmed 2026-11-05)
- NEVER let the SPA call a service directly instead of the gateway, except Supabase Auth and WalletConnect. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER put secrets in source, env files, logs, errors, analytics, or support screens. (`.claude/rules/development.md`, BRD-012 NFR-SEC-03/15) (affirmed 2026-11-05)
- NEVER log credentials or tokens. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER request or store wallet private keys or seed phrases. (BRD-012 NFR-SEC-04) (affirmed 2026-11-05)
- NEVER enable withdrawals or trading permissions on exchange integrations. (BRD-012 NFR-SEC-05) (affirmed 2026-11-05)
- NEVER store browser tokens in localStorage. (BRD-012 NFR-SEC-10) (affirmed 2026-11-05)
- NEVER include credentials, tokens, or full addresses in audit records. (BRD-012 audit logging) (affirmed 2026-11-05)
- NEVER compute business math in the frontend. (`.claude/rules/development.md`) (affirmed 2026-11-05)
- NEVER use raw colour, spacing, or type values instead of Parchment Ledger tokens. (`.claude/rules/development.md`, `design-system/DESIGN.md`) (affirmed 2026-11-05)
- NEVER render a missing financial value as `0` or `$0.00`; use an em dash. (`design-system/DESIGN.md`) (affirmed 2026-11-05)
- NEVER let colour alone carry meaning; pair every status colour with a label and a shape. (`design-system/DESIGN.md`) (affirmed 2026-11-05)
- NEVER swallow an error silently. (`memory/phases/construction.md`) (affirmed 2026-11-05)
- NEVER hardcode credentials, API keys, or secrets. (`memory/phases/construction.md`) (affirmed 2026-11-05)
- NEVER generate tests that always pass regardless of implementation. (`memory/phases/construction.md`) (affirmed 2026-11-05)
- NEVER merge production code whose test was written after the implementation. (Interview Q3) (affirmed 2026-11-05)
- NEVER create a staging environment or gate a release on staging; production is the only environment. (Interview Q5, Q9 — supersedes ideation scope document launch gates C11–C12) (affirmed 2026-11-05)
- NEVER ship a schema change without a matching down migration. (Interview Q7) (affirmed 2026-11-05)
- NEVER let a Swarm service update proceed without its health check passing. (Interview Q6) (affirmed 2026-11-05)
- NEVER remove or bypass existing security controls without explicit approval and documented rationale. (`memory/phases/operation.md`) (affirmed 2026-11-05)
- NEVER commit generated Structurizr artefacts (`docs/architecture/workspace.json`, `docs/architecture/.structurizr/`). (`.gitignore`, `docs/architecture/README.md` §3) (affirmed 2026-11-05)
- NEVER carry forward an unresolved contradiction between requirements; surface and resolve it explicitly. (`memory/phases/inception.md`) (affirmed 2026-11-05)
- NEVER introduce a new requirement in inception without documenting its origin. (`memory/phases/inception.md`) (affirmed 2026-11-05)
- NEVER ship a schema change without a matching down migration. (Interview Q6) (affirmed 2026-11-10)
- NEVER let a Swarm service update proceed without its health check passing. (Interview Q1) (affirmed 2026-11-10)
