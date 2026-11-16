# Discovered Rules — Crypto Pocket Butler

Hard constraints stated by the project's own documents or affirmed by the human at a practices
interview. Each rule is traceable to the source named after it. Product requirements that are not
engineering constraints remain in the BRDs and are not duplicated here.

Carried forward from `aidlc/spaces/default/memory/project.md` (affirmed 2026-11-05 and
2026-11-10) after re-checking each against the tree at commit `112b354`, plus the five rules the
person affirmed at this run's interview. Duplicates that the promotion tool had appended twice
were collapsed here to one line each. Rules marked **[corrected]** were contradicted by the
repository and are rewritten to match it. Rules marked **[intention]** are stated intent, not
current behaviour.

> **Open action for the rules file's owner (interview Q9):** `.claude/rules/development.md` still
> says "Yarn ≥ 6" (no such Yarn release exists; the pin is 4.18.1) and "Docker Swarm — every
> service and the observability stack" (the observability stack is a target, not deployed). Both
> corrections are made in memory at promotion; the two sentences in that file are left to its
> owner and are not edited by this stage.

**Correction evidence:** the reverse-engineering knowledge base
`aidlc/spaces/default/codekb/crypto-pocket-butler/` (`code-structure.md`, `technology-stack.md`,
`dependencies.md`, `code-quality-assessment.md`, `architecture.md`, `business-overview.md`),
re-verified directly in `infra/swarm-stack.yml`, `infra/prometheus/support-rules.yml`, the root
`Cargo.toml`, `services/*/migrations/`, `services/gateway/domain/src/route.rs`, the three
`package.json` files and `aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md`.
Per-claim detail is in `evidence.md`.

## Mandated

### Stack and boundaries

- ALWAYS use Rust with Axum, tower, sqlx, rust_decimal, and tokio for backend services and the API Gateway. (`.claude/rules/development.md` — stack fixed, do not substitute)
- ALWAYS keep Rust dependencies pointing inward: `http`/`infrastructure` → `application` → `domain`; `domain` imports no sibling crate or framework; `application` never imports `infrastructure` or `http`; nothing imports `http`. (`.claude/rules/development.md`, `docs/development/README.md`)
- ALWAYS express cross-boundary dependencies as port traits defined inward and implemented outward. (`.claude/rules/development.md`)
- ALWAYS map DTOs to/from domain types at the `http` boundary. (`.claude/rules/development.md`)
- ALWAYS use `rust_decimal` for quantities and values; never binary floating point. (`.claude/rules/development.md`, `docs/architecture/README.md` — decimal arithmetic kept on purpose)
- ALWAYS use React 18 + TypeScript 5 with Vite, TanStack Query, `@supabase/supabase-js`, Recharts, and Tailwind CSS v4 mapped to Parchment Ledger tokens for frontend work. (`.claude/rules/development.md`)
- ALWAYS route SPA calls through the API Gateway; the only direct browser-external calls are Supabase Auth for sign-in and WalletConnect for read-only sessions. (`.claude/rules/development.md`, `docs/architecture/README.md` §6)
- ALWAYS declare every gateway route the SPA calls in the deny-by-default route table, with an owning service handler behind it — and ALWAYS treat reachability through those declared routes as the completion test for a slice. (Interview Q2 of this run; `services/gateway/domain/src/route.rs` header "Anything not declared is denied (BR4.2)", `services/identity/http/src/routes.rs`, `web/src/api/client.ts`)
- ALWAYS compute valuation, drift, allocation, and other business math in the backend. (`.claude/rules/development.md`)
- ALWAYS use Parchment Ledger design tokens; no raw colour, spacing, or type values. (`.claude/rules/development.md`, `design-system/DESIGN.md`)

### Identity, data ownership and secrets

- ALWAYS give each service one Supabase Postgres schema and one Postgres role, and read another service's data only through that service's HTTP API. (`.claude/rules/development.md`, `docs/architecture/README.md` §7)
- ALWAYS forward the user's Supabase JWT unchanged from the gateway, and ALWAYS validate it in every service against Supabase JWKS (signature, issuer, audience, expiry). (`.claude/rules/development.md`, BRD-012 NFR-SEC-09)
- ALWAYS use synchronous internal HTTP with bounded retries and idempotency keys for service-to-service calls. (`.claude/rules/development.md`, `docs/architecture/README.md` §6)
- ALWAYS store secrets in Supabase Vault (pgsodium) or project secrets. (`.claude/rules/development.md`, BRD-012 NFR-SEC-15)
- ALWAYS enforce ownership checks on every read and write of user-owned resources. (BRD-012 NFR-SEC-07)
- ALWAYS keep exchange integrations read-only and verify key permissions before use and periodically. (BRD-012 NFR-SEC-05/06)
- ALWAYS require MFA for users who have exchange credentials. (BRD-012 NFR-SEC-11)
- ALWAYS keep browser session tokens in memory only. (BRD-012 NFR-SEC-10)
- ALWAYS rate-limit public endpoints per user/IP/region at the gateway, with stricter limits for auth and manual triggers. (BRD-012 NFR-SEC-12)
- ALWAYS use TLS for all production traffic. (BRD-012 NFR-SEC-01)
- ALWAYS record security-sensitive events with actor (or system), action, target, timestamp, context, and outcome. (BRD-012 audit logging, NFR-SEC-13)
- ALWAYS keep the audit schema append-only through an insert-only Postgres role. (`docs/architecture/README.md` §7)
- ALWAYS scan dependencies and container images for vulnerabilities before release. **[intention]** — required by BRD-012 NFR-SEC-14 and named in unit design records; no scanner configuration or pipeline exists in the tree. (BRD-012 NFR-SEC-14)

### Deployment

- ALWAYS deploy every backend service and the static frontend artefact as replicated Docker Swarm services. **[corrected]** — the observability stack is *not* deployed: `infra/swarm-stack.yml` contains no Prometheus, Loki, Tempo or Grafana service, and no scrape configuration exists anywhere in the repository. The self-hosted Loki/Prometheus/Tempo/Grafana stack remains the target; structured logs exist today, metrics and traces are not collected. The earlier wording ("every service and the observability stack") named something that does not exist — the same claim is still in `docs/development/README.md`'s stack table and is part of the open action above. (`.claude/rules/development.md`, `docs/development/README.md`, `infra/swarm-stack.yml` — 31 services, none of them the observability stack; `infra/prometheus/support-rules.yml` header: "There is no Prometheus scrape configuration in this repository"; Interview Q6 of this run)
- ALWAYS deploy with the Docker Swarm rolling update configured in `infra/swarm-stack.yml` — `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, `order: start-first` for the APIs, gateway, `web`, `audit-retention`, `job-orchestration` and `identity-worker`, and `order: stop-first` for the seven background workers — with the service health check gating the update. (Interview Q1 of the second discovery, `infra/swarm-stack.yml` — re-verified in this run: 22 update policies, 22 health checks, 22 `stop_grace_period` entries)
- ALWAYS tag release images with the commit SHA, and ALWAYS release continuously from `main` with no semantic-versioning ceremony until a first production release exists. (Interview Q8 of this run; `infra/swarm-stack.yml` `${REGISTRY:-ghcr.io/crypto-pocket-butler}/<service>:${TAG:-latest}`; `git tag` empty)
- ALWAYS ship schema changes as versioned sqlx up/down migration pairs, and ALWAYS run the service's migrate one-shot before the API and worker are updated — then apply the matching down migration on rollback as an automatic step of the release job. **[intention]** — 19 matched pairs exist in eleven services, 9 one-shot `*-migrate` services ship, and no job exists to order them or apply a `.down.sql`; building the automatic down step is an explicit deliverable of this intent. (Interview Q7 of this run; `services/*/src/bin/migrate.rs` header; `infra/swarm-stack.yml`)
- ALWAYS give every new service or component at least one health metric and one error rate metric. (`memory/phases/operation.md`)
- ALWAYS include rollback steps in every deployment procedure, and a smoke test or health check for production deployments. (`memory/phases/operation.md`)
- ALWAYS quantify SLOs with a percentage and a time window, and set alert thresholds below the SLO breach. (`memory/phases/operation.md`)
- ALWAYS include escalation paths and contacts in runbooks, and write a post-incident review for every P1/P2 incident. (`memory/phases/operation.md`)

### Way of working, review and traceability

- ALWAYS require a human review before merge. (`memory/org.md`)
- ALWAYS link every pull request to its requirement or unit id. (`memory/team.md`)
- ALWAYS keep a change scoped to one unit of work. (`memory/project.md`)
- ALWAYS merge unit work back within days; an intent's own AI-DLC branch may live as long as the intent. (Interview Q1 of this run; `memory/team.md` "short-lived branches"; `112b354` on `feat/authentication-session-management`)
- ALWAYS explain why before what in commit messages. (`memory/project.md`)
- ALWAYS write acceptance criteria in Given/When/Then form. (`memory/phases/inception.md`)
- ALWAYS document at least two alternatives in an architecture decision, and include Context, Decision, Consequences, and Alternatives Rejected in every ADR. (`memory/phases/inception.md`)
- ALWAYS update the C4 model and `docs/architecture/README.md` together when a container, schema, external system, or relationship changes. (`docs/architecture/README.md` §13)
- ALWAYS validate the Structurizr model before review. (`docs/architecture/README.md` §12)
- ALWAYS define the retention and legal-hold policy, launch jurisdictions, privacy notices, consumer-subscription terms, market-data licences, and incident-response plan before launch. (BRD-012 — "Must define before launch")

### Testing and quality

- ALWAYS write a failing test before the implementation for all production code (TDD), and land the test in the same change as the code it covers. (Interview Q3 of the first discovery)
- ALWAYS cover the happy path and at least two error or edge cases in every test file. (`memory/phases/construction.md`)
- ALWAYS keep line coverage at or above 80% per Rust crate. (Interview Q4 of the first discovery, re-affirmed at Q4 of this run)
- ALWAYS measure Rust line coverage with `cargo llvm-cov`, using a reading that instruments the package carrying the tests — the workspace run, or the paired `-p <crate> -p <service>` run — after the disposable database is up so the `#[ignore]` suites contribute. **[corrected]** — a bare `cargo llvm-cov -p <crate>` measures almost nothing here, because most suites live in the service package rather than the library crate; a review round was already spent on a number produced that way. (`aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md` "The measurement trap"; Interview Q4 of this run)
- ALWAYS quote the `cargo llvm-cov` command beside a coverage figure, so every quoted number carries the invocation that produced it. (Interview Q4 of this run)
- ALWAYS run `yarn e2e` for any change touching the auth/sign-in surface, and treat a green run as a gate on that change. (Interview Q3 of this run; `web/package.json`, `web/e2e/README.md` — the suite found a first-render crash all 312 unit tests passed over)
- ALWAYS provision a disposable Postgres database per gate run and run the `#[ignore]` adapter and migration suites against it as a blocking gate, with `TEST_DATABASE_URL` pointing at that database. **[intention]** — no committed provisioning script exists; the runbook records the manual `docker run supabase/postgres:15.8.1.060` shape. (Interview Q5 of the second discovery, `coverage-runbook.md`)
- ALWAYS handle errors at integration boundaries and surface or log them, distinguishing recoverable from fatal errors. (`memory/phases/construction.md`)
- ALWAYS validate and sanitize inputs at system boundaries. (`memory/phases/construction.md`)
- ALWAYS run `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` for Rust, and ESLint plus Prettier through Yarn for TypeScript. (Interview Q8 of the first discovery)
- ALWAYS run locally the same build, lint, and test commands that CI runs. (`memory/team.md`)
- ALWAYS keep the canonical build, lint and test command list in `aidlc/spaces/default/memory/project.md` as the single authoritative list, including `yarn e2e` and `yarn size` for `web/`. (Interview Q7 of the second discovery, Interview Q9 of this run)

### Tooling

- ALWAYS use Yarn 4.18.1, pinned through corepack (`packageManager: yarn@4.18.1` at the root and in `web/` and `support-console/`), for every JavaScript/TypeScript command. (`.claude/rules/development.md`, `package.json`, `web/package.json`, `support-console/package.json` — supersedes the earlier "Yarn ≥ 6" rule, which no Yarn release satisfied; re-verified in this run at all three levels)

## Forbidden

### Stack and architecture

- NEVER use a backend language other than Rust or an ORM other than sqlx. (`.claude/rules/development.md`)
- NEVER let `domain` import a sibling crate or framework — no Axum, sqlx, reqwest, or serde transport types. (`.claude/rules/development.md`, `docs/development/README.md`)
- NEVER let `application` import `infrastructure` or `http`, and NEVER let any crate import `http`. (`.claude/rules/development.md`)
- NEVER leak DTO types into `application` or `domain`. (`.claude/rules/development.md`)
- NEVER use npm, npx, pnpm, or their lockfiles. (`.claude/rules/development.md`)
- NEVER compute business math in the frontend. (`.claude/rules/development.md`)

### Data and communication

- NEVER grant cross-schema access, and NEVER let a service read another service's tables. (`.claude/rules/development.md`, `docs/architecture/README.md` §7)
- NEVER use Row Level Security in services. (`.claude/rules/development.md`)
- NEVER use message queues. (`.claude/rules/development.md` — "No message queues"; `docs/architecture/README.md` §6)
- NEVER let the SPA call a service directly instead of the gateway, except Supabase Auth and WalletConnect. (`.claude/rules/development.md`)
- NEVER let an SPA call exist without a declared gateway route and an owning service handler behind it. (Interview Q2 of this run; `services/gateway/domain/src/route.rs` — deny-by-default)

### Security and privacy

- NEVER put secrets in source, env files, logs, errors, analytics, or support screens. (`.claude/rules/development.md`, BRD-012 NFR-SEC-03/15)
- NEVER log credentials or tokens. (`.claude/rules/development.md`)
- NEVER request or store wallet private keys or seed phrases. (BRD-012 NFR-SEC-04)
- NEVER enable withdrawals or trading permissions on exchange integrations. (BRD-012 NFR-SEC-05)
- NEVER store browser tokens in `localStorage`. (BRD-012 NFR-SEC-10)
- NEVER include credentials, tokens, or full addresses in audit records. (BRD-012 audit logging)
- NEVER hardcode credentials, API keys, or secrets. (`memory/phases/construction.md`)
- NEVER remove or bypass existing security controls without explicit approval and documented rationale. (`memory/phases/operation.md`)

### Design system

- NEVER use raw colour, spacing, or type values instead of Parchment Ledger tokens. (`.claude/rules/development.md`, `design-system/DESIGN.md`)
- NEVER render a missing financial value as `0` or `$0.00`; use an em dash. (`design-system/DESIGN.md`)
- NEVER let colour alone carry meaning; pair every status colour with a label and a shape. (`design-system/DESIGN.md`)

### Testing and release safety

- NEVER swallow an error silently. (`memory/phases/construction.md`)
- NEVER generate tests that always pass regardless of implementation. (`memory/phases/construction.md`)
- NEVER merge production code whose test was written after the implementation. (Interview Q3 of the first discovery)
- NEVER merge a change touching the auth/sign-in surface with a failing `yarn e2e`. (Interview Q3 of this run)
- NEVER quote a coverage figure without the `cargo llvm-cov` command that produced it. (Interview Q4 of this run)
- NEVER create a staging environment or gate a release on staging; production is the only environment. (Interview Q5 and Q9 of the first discovery — supersedes ideation scope document launch gates C11–C12)
- NEVER ship a schema change without a matching down migration. (Interview Q6 of the second discovery, re-affirmed at Q7 of this run)
- NEVER update the API or worker before the service's migrate one-shot has run against the migrator DSN. (Interview Q7 of this run; `services/*/src/bin/migrate.rs` header; `infra/swarm-stack.yml` `*-migrate` services)
- NEVER let a Swarm service update proceed without its health check passing. (Interview Q1 of the second discovery)

### Repository hygiene

- NEVER commit generated Structurizr artefacts (`docs/architecture/workspace.json`, `docs/architecture/.structurizr/`). (`.gitignore`, `docs/architecture/README.md` §3)

### Requirements discipline

- NEVER carry forward an unresolved contradiction between requirements; surface and resolve it explicitly. (`memory/phases/inception.md`)
- NEVER introduce a new requirement in inception without documenting its origin. (`memory/phases/inception.md`)
