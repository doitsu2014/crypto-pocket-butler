# Team Memory

Rules specific to this team. More specific than organisation memory; a narrower
rule here overrides a broader rule only when it does not contradict it.

## Team Practices

- Trunk-based development with short-lived branches.
- Every pull request links to its requirement or unit id.
- Reviews focus on correctness and security before style.

## Tooling

- The project's own build, lint, and test commands are authoritative.
- CI runs the same commands developers run locally.

## On-call

- Alerts must be actionable; delete alerts nobody acts on.

## Way of Working

We work trunk-based on `main` with short-lived branches, and we merge back within days rather
than weeks. Every pull request is small, stays scoped to one unit of work, and links to the
requirement or unit id it serves — a `BR-`/`FR-`/`NFR-` id or a `U-###` unit. A human reviews
every change before merge, and the review looks for correctness and security before style.
Commit messages explain why the change is needed before describing what changed. We confirmed
this way of working unchanged at the practices interview: no maximum branch age, PR size cap,
branch-name pattern, commit-message convention, or CODEOWNERS rule is added on top of it.
Recent history matches the practice — squash-merged pull requests land directly on `main`, and
the live side branches are short-lived `chore/*` and `copilot/*` branches. *(Sources:
`memory/team.md`, `memory/org.md`, `memory/project.md`, `git log`, `git branch -a`; interview
Q1.)*

## Walking Skeleton

We build the thinnest end-to-end slice first, before feature breadth. Our first slice is the
Asset Registry path: the SPA shell calls the API Gateway; the gateway validates and forwards a
Supabase JWT to the Asset Registry service; that service reads and writes its own Supabase
Postgres schema through sqlx; and the whole path runs on the Docker Swarm with health and
readiness endpoints and with logs, metrics, and traces visible in the self-hosted observability
stack. We start with Asset Registry because it has no exchange or RPC credential dependency and
exercises the full Clean Architecture, schema, and role path. We prove that slice end to end
first, then grow the remaining services behind the same pattern. *(Sources: `memory/org.md` —
greenfield work starts with the thinnest end-to-end slice; `docs/architecture/README.md` §§5–6;
interview Q2. The target is the production Swarm, since production is our only environment —
see Deployment.)*

## Testing Posture

- **Methodology**: tdd
- **Methodology evidence**: The team affirmed test-first development for all production code at
  the practices interview. The organisation baseline still applies underneath it:
  requirement-driven tests with a happy-path floor per component (`memory/org.md`), acceptance
  criteria in Given/When/Then form (`memory/phases/inception.md`), business rules tested in
  `application` use cases against port fakes, and one integration test per adapter against a
  real Postgres schema (`docs/development/README.md`). TDD governs when each of those tests is
  written, not what they cover. No project document mandated test-first before this decision,
  so it is recorded here as the team's affirmed choice. *(Interview Q3.)*
- **Ordering**: We write a failing test first for every piece of production code, then the
  implementation that makes it pass, and we land the test in the same change as the code it
  covers.
- **Coverage**: 80% line coverage per Rust crate, enforced in CI. Every test file still covers
  the happy path and at least two error or edge cases (`memory/phases/construction.md`), and a
  test that always passes regardless of the implementation is not acceptable. The coverage tool
  and the TypeScript coverage threshold are not chosen yet (unresolved). *(Interview Q4.)*
- **Test types**: unit tests in `application` against port fakes; one integration test per
  adapter against a real Postgres schema. The API and cross-service contract-testing approach
  still needs agreement (unresolved).
- **Quality gates**: a change merges only when the same build, lint, and test commands CI runs
  pass locally and in CI (`memory/team.md`). Crate-boundary enforcement is mechanical and fails
  the build; dependency and container-image vulnerability scanning runs before release
  (`docs/development/README.md`, BRD-012 NFR-SEC-14). The CI platform and the full ordered gate
  list are not configured yet (unresolved).

## Deployment

We deploy every backend service, the SPA, the support console, and the self-hosted observability
stack as replicated Docker Swarm services, and production is our only environment: there is no
staging tier and no promotion step between environments. A release rolls out as a Swarm rolling
update using the Swarm defaults, with each service's health check gating the update. Schema
changes ship as versioned sqlx up/down migration pairs, and rolling a release back applies the
matching down migration as an automatic step. Images are digest-pinned and vulnerability-scanned
before release, the edge uses CDN/WAF, and a deployment is not complete until its health or
smoke check passes. Every deployment procedure carries a rollback path. Secrets reach services
from Supabase Vault (pgsodium), never from env files, source, or logs. *(Sources:
`.claude/rules/development.md`, `docs/architecture/README.md` §§5–7 and §10, BRD-012; interview
Q5–Q7, Q9.)*

- **Environments**: production only, on the Docker Swarm cluster. The ideation scope document's
  staging environment and its C11–C12 staging launch gate are dropped by this decision and no
  longer apply; the scope document itself still needs that change recorded (unresolved).
  *(Interview Q5, Q9.)*
- **Release cadence**: not stated (unresolved).
- **Deployment strategy**: rolling updates with Swarm defaults, health-checked, for every
  component. No blue-green or canary path is adopted. *(Interview Q6.)*
- **Database migrations**: versioned sqlx up/down pairs, with the automatic down step on
  rollback. Who runs migrations in the pipeline, how the down step is triggered, and migration
  naming and ordering are not settled (unresolved). *(Interview Q7.)*
- **Reliability targets**: 99.9% monthly availability for paid plans; RPO ≤24h and RTO ≤4h;
  SEV-1 communication within 15 minutes (BRD-012 NFR-AVL-01/04/05/06).
- **Progressive delivery**: feature flags with owner, expiry, and rollout percentage are a
  product capability (BRD-010 FR-ADM-18, BRD-021 FR-ANL-10); whether they gate deployments is
  unresolved.
- **Observability**: every service emits logs, metrics, and traces to the self-hosted
  Loki/Prometheus/Tempo/Grafana stack; nothing logs credentials or tokens
  (`docs/architecture/README.md` §§5 and 12).
- **Rollback triggers**: a failed post-update health check is the one stated trigger. The wider
  set of production rollback triggers and the approval path for a rollback are not stated
  (unresolved).

## Code Style

We keep the codebase boring, explicit, and boundary-respecting.

**Rust.** Every service is a Cargo workspace of four crates — `domain`, `application`,
`infrastructure`, `http` — and dependencies point inward only. `domain` holds entities, value
types, and business rules and imports no framework; `application` orchestrates use cases behind
port traits; `infrastructure` implements those ports (sqlx repositories, Vault, provider
clients, OTLP); `http` holds Axum routers, middleware, and DTOs. DTOs map to/from domain types
at the `http` boundary and never leak inward. Cross-boundary needs are expressed as ports, not
concrete imports. Quantities and values use decimal arithmetic (`rust_decimal`), never binary
floating point. Errors are handled at integration boundaries and surfaced or logged — never
swallowed. *(Sources: `.claude/rules/development.md`, `docs/development/README.md`,
`docs/architecture/README.md`.)*

**TypeScript/React.** The SPA is React 18 + TypeScript 5 on Vite, with TanStack Query for
server state, Tailwind CSS v4 mapped to the Parchment Ledger tokens, and `@supabase/supabase-js`
for auth. Components render; one API-client module owns all gateway calls; business math
(valuation, drift, allocation) is computed only in the backend. Use design tokens, never raw
colour, spacing, or type values; financial figures use tabular mono, and a missing value is an
em dash rather than `0` or `$0.00`. *(Sources: `.claude/rules/development.md`,
`design-system/DESIGN.md`.)*

**Tooling.** Yarn ≥ 6 is the only package manager, pinned through corepack. Rust is formatted and
linted with `rustfmt` and `clippy`, run in CI and locally as `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings`. TypeScript is linted and formatted with
ESLint and Prettier through Yarn. The exact Yarn script names, the Rust and TypeScript test
commands and runners, naming conventions beyond the framework defaults, and any pre-commit hook
are not chosen yet (unresolved). *(Sources: `.claude/rules/development.md`,
`docs/development/README.md`; interview Q8.)*

**Repository hygiene.** Generated Structurizr artefacts (`docs/architecture/workspace.json`,
`docs/architecture/.structurizr/`) are build outputs and stay out of git. Commit messages
explain why before what, and a change stays scoped to one unit of work. *(Sources:
`.gitignore`, `docs/architecture/README.md` §3, `memory/project.md`.)*
