# Team Practices — Crypto Pocket Butler

## Way of Working

We work trunk-based on `main` with short-lived branches, and we merge back within days rather
than weeks. Every pull request is small, stays scoped to one unit of work, and links to the
requirement or unit id it serves — a `BR-`/`FR-`/`NFR-` id or a `U-###` unit. A human reviews
every change before merge, and the review looks for correctness and security before style.
Commit messages explain why the change is needed before describing what changed.

Recent history matches the practice: the November 2026 commits are single-parent, squash-merged
pull requests landing directly on `main`, almost all with a `(#NNN)` suffix (three are direct
commits without one: `a567470` and `dfab8e1` on 2026-11-01, and `6d83dee`). The branches still
current are `chore/*`, `wip/*` and `docs/*` branches plus descriptive feature branches
(`parchment-ledger-design-system`, `simplify-architecture`), all created within a week of
landing. Commit bodies carry the why first — a `Why:` paragraph, then `What:`; some AI-DLC-era
bodies add a `Units:` line naming the unit ids the change serves (for example commit `14742cb`
closes with "Units: none (developer tooling) — makes U1–U13 runnable locally"). No
pull-request template, CODEOWNERS file, branch-name pattern, commit-message convention,
branch-age cap or PR size cap exists, so the unit link and the human review are team conventions
rather than mechanically enforced checks. Sixteen remote branches last touched between March and
May 2026 (`copilot/*`, `feature/*`, `refactor`, `add-spec-kit`) are still present; they are
stale housekeeping, not a live branch strategy. *(Sources: `memory/team.md`, `memory/org.md`,
`memory/project.md`, `git log`, `git branch -a`, `git for-each-ref`; previous practices
discovery Q1, re-confirmed by the evidence review in this run.)*

## Walking Skeleton

We build the thinnest end-to-end slice first, before feature breadth. Our first slice is the
Asset Registry path: the SPA shell calls the API Gateway; the gateway validates and forwards a
Supabase JWT to the Asset Registry service; that service reads and writes its own Supabase
Postgres schema through sqlx; and the whole path runs on the Docker Swarm with health and
readiness endpoints and with logs, metrics, and traces visible in the self-hosted observability
stack. We start with Asset Registry because it has no exchange or RPC credential dependency and
exercises the full Clean Architecture, schema, and role path. We prove that slice end to end
first, then grow the remaining services behind the same pattern.

The slice has now been built and proven. Unit U1 (`u1-walking-skeleton`) is complete — gateway,
Asset Registry, SPA shell, the Swarm stack, the migrations and the test suite all exist (commit
`6544576`). Commit `14742cb` then added `deployment/localhost`, a Docker Compose stack that runs
the whole system on one machine: self-hosted Supabase (Postgres with pgsodium, Auth, REST,
Storage, Studio, Mailpit, Kong), every Rust service and worker, the API Gateway and Asset
Registry, the schema migrations, and the SPA and support console behind nginx. That stack is
developer convenience only — production remains the Swarm — but it lets the slice be exercised
without production credentials. The remaining gap is that the gateway's deny-by-default route
table still declares only the `/v1/assets` paths, so the SPA cannot reach Identity, Account,
Portfolio and the other services through `/v1` yet; those routes are the next step in growing
the skeleton. *(Sources: `memory/org.md` — greenfield work starts with the thinnest end-to-end
slice; `docs/architecture/README.md` §§5–6; `deployment/localhost/README.md` "Known gaps";
`git log` commits `6544576` and `14742cb`; previous practices discovery Q2, re-confirmed by the
evidence review in this run.)*

## Testing Posture

- **Methodology**: tdd
- **Methodology evidence**: The team affirmed test-first development for all production code at
  the previous practices interview and re-confirmed it in this run's evidence review. The
  organisation baseline still applies underneath it: requirement-driven tests with a happy-path
  floor per component (`memory/org.md`), acceptance criteria in Given/When/Then form
  (`memory/phases/inception.md`), business rules tested in `application` use cases against port
  fakes, and one integration test per adapter against a real Postgres schema
  (`docs/development/README.md`). TDD governs when each of those tests is written, not what they
  cover. The repository cannot prove write-order from history — there are 71 `.rs` files under
  `services/*/tests` and the crate `tests/` trees (suites plus shared fakes and helpers), 50
  frontend `*.test.ts(x)` files, and ten test files whose database-backed cases are marked
  `#[ignore]` — so ordering remains a team commitment rather than an observable.
- **Ordering**: We write a failing test first for every piece of production code, then the
  implementation that makes it pass, and we land the test in the same change as the code it
  covers.
- **Test types and layout**: domain rule tests per service; application use cases against
  hand-written port fakes in `tests/fakes.rs` or `tests/common/fakes.rs`; HTTP boundary tests
  over the Axum routers with status codes and RFC 7807 problem bodies; infrastructure adapter
  tests against a real Postgres schema, which are `#[ignore]`d by default and run only when
  `TEST_DATABASE_URL` is set. The ignored adapter and migration suites are now a blocking gate,
  not a developer opt-in: every run provisions a disposable Postgres database and points
  `TEST_DATABASE_URL` at it, so an adapter regression fails the change. Until the CI platform is
  chosen, that gate is run by the same local command a developer runs today. The frontends use
  Vitest 3 with Testing Library and jest-axe; MSW mocks the gateway in the support console.
  There is no end-to-end browser journey suite and no cross-service contract test (unresolved).
- **Commands and runners**: the canonical build, lint and test commands are:
  - Rust — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo test --workspace`, and `bash scripts/check-boundaries.sh` (the mechanical Clean
    Architecture gate).
  - Frontend — in both `web/` and `support-console/`: `yarn test` (`vitest run`), `yarn lint`
    (ESLint 9 flat config), `yarn build` (`tsc -b && vite build`) and `yarn format:check`
    (Prettier). `web` also defines `yarn size` (size-limit budgets: 300 kB initial entry chunk,
    150 kB route chunk, 150 kB stylesheet, all gzip).

  `aidlc/spaces/default/memory/project.md` holds this list as the single authoritative source;
  `docs/development/README.md` and the per-package `package.json` scripts remain the
  implementation detail behind it.
- **Coverage**: 80% line coverage per Rust crate remains the affirmed target and is measured
  with `cargo llvm-cov`, the coverage tool chosen at this interview. No TypeScript coverage
  threshold has been set (unresolved). The happy-path-plus-two-error/edge-case floor per test
  file still applies and a test that always passes regardless of implementation is not
  acceptable (`memory/phases/construction.md`).
- **Quality gates**: `bash scripts/check-boundaries.sh` is a real mechanical gate — it parses
  `cargo metadata --no-deps` and fails on any outward crate edge. The Rust gates are
  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and
  `cargo test --workspace`; the frontend gates are `yarn test`, `yarn lint`, `yarn build` and
  `yarn format:check`. There is still no CI platform in the repository — no `.github/`, GitLab,
  Jenkins or CircleCI configuration — and the team chose at this interview to leave the platform
  open for now and record CI as an intention, so today these gates run when a person runs them.
  Dependency and container-image scanning (`cargo audit`, Trivy) is required by BRD-012
  NFR-SEC-14 and named in unit design records but has no configuration or pipeline.

## Deployment

We deploy every backend service, the SPA, the support console, and the self-hosted observability
stack as replicated Docker Swarm services, and production is our only environment: there is no
staging tier and no promotion step between environments. A release rolls out as a rolling update
in which each service's health check gates the update. Schema changes ship as versioned sqlx
up/down migration pairs, and rolling a release back applies the matching down migration as an
automatic step of the release job. Every deployment procedure carries a rollback path. Secrets
reach services from Supabase Vault (pgsodium), never from env files, source, or logs.

- **Environments**: production only, on the Docker Swarm cluster. `deployment/localhost` is a
  developer convenience on one machine, not an environment tier and not a promotion step. The
  ideation scope document's staging environment and its C11–C12 staging launch gate were
  dropped by the previous interview and no longer apply; the scope document itself still needs
  that change recorded (unresolved).
- **Deployment strategy**: `infra/swarm-stack.yml` keeps its configured rolling-update policy,
  and the rule now matches it. Every long-running service carries a health check on `/healthz`
  (job-orchestration checks `/readyz`) and an update policy of `parallelism: 1`,
  `failure_action: rollback` and `monitor: 30s`, plus a `rollback_config` that pauses on failure
  and a `stop_grace_period` of 30s. APIs, the gateway, the SPA, the audit retention service, job
  orchestration and `identity-worker` roll `start-first`; the remaining background workers
  (`account`, `portfolio`, `reporting`, `market-data`, `analytics`, `ledger`, `notification`)
  roll `stop-first`. The earlier "using the Swarm defaults" wording is retired: the stack is
  explicit, and `start-first` is not the literal Swarm default (`stop-first` with
  `failure_action: pause`). Replicas are 1 for workers and migrators, 2 for the edge, registry,
  job orchestration and most APIs, and 3 for the audit, identity and account APIs. No
  blue-green or canary path is adopted. *(Sources: `infra/swarm-stack.yml`; interview Q1.)*
- **Images**: the stack references
  `${REGISTRY:-ghcr.io/crypto-pocket-butler}/<service>:${TAG:-latest}`, and its header comment
  says a pipeline builds and digest-pins each image. No pipeline exists in the repository, so
  digest pinning and pre-release vulnerability scanning are stated intentions, not current
  behaviour.
- **Database migrations**: 13 versioned `<NNNN>_<name>.{up,down}.sql` pairs live across the
  services' `migrations/` directories. Each service has a `migrate` binary whose header says
  "The release job runs this once against the migrator DSN before the API and worker are
  updated", and the stack ships one-shot `*-migrate` services with `restart_policy: none`; the
  local stack applies the up migrations in a `provision` job. The team kept the down-migration
  rule at this interview and made the automatic down step a required part of the release job:
  a rollback applies the matching `.down.sql` as part of the release job's rollback path.
  Nothing in the repository applies a `.down.sql` today, so the step is still to be built and
  tested, and who orders the migrate job against the service update remains open. *(Interview
  Q6.)*
- **Release completion**: there is no smoke script or smoke job. A deployment is considered
  healthy when the service health check passes; the stack comments reserve `*_smoke_token`
  secrets for a release job that is not in the repository. Release cadence is not stated
  (unresolved).
- **Secrets**: services read credentials through `*_FILE` environment variables pointing at
  Docker secrets; production materialises those from Supabase Vault. The local stack generates
  its own permissive local tokens and is explicitly not a production credential model.
- **Observability**: every service exposes `/healthz`, `/readyz` and `/metrics`; Prometheus
  alert rules live in `infra/prometheus/`; logs, traces and dashboards target the self-hosted
  Loki/Prometheus/Tempo/Grafana stack. Nothing logs credentials or tokens.
- **Reliability targets**: 99.9% monthly availability for paid plans; RPO ≤24h and RTO ≤4h;
  SEV-1 communication within 15 minutes (BRD-012 NFR-AVL-01/04/05/06).
- **Progressive delivery**: feature flags with owner, expiry and rollout percentage are a
  product capability (BRD-010 FR-ADM-18, BRD-021 FR-ANL-10); whether they gate deployments is
  unresolved.
- **Rollback triggers**: a failed post-update health check is the one stated trigger, and the
  stack's `failure_action: rollback` implements it for long-running services. The wider set of
  production rollback triggers and the approval path for a rollback are not stated (unresolved).

## Code Style

We keep the codebase boring, explicit, and boundary-respecting.

**Rust.** Every service is a Cargo workspace of four crates — `domain`, `application`,
`infrastructure`, `http` — and dependencies point inward only. `domain` holds entities, value
types, and business rules and imports no framework; `application` orchestrates use cases behind
port traits; `infrastructure` implements those ports (sqlx repositories, Vault, provider
clients, JWKS, OTLP); `http` holds Axum routers, middleware, and DTOs. DTOs map to/from domain
types at the `http` boundary and never leak inward. Cross-boundary needs are expressed as ports,
not concrete imports. Quantities and values use decimal arithmetic (`rust_decimal`), never
binary floating point. Errors are typed per service (`DomainError` inward, `AppError` mapping at
`from_domain` / `from_repository` / `from_external`) and rendered at the boundary as RFC 7807
`ProblemDetails`; they are surfaced or logged, never swallowed. Migrations use
`<NNNN>_<name>.{up,down}.sql` pairs. Module-level `//!` headers explain purpose and cite the BR
ids they serve. *(Sources: `.claude/rules/development.md`, `docs/development/README.md`,
`docs/architecture/README.md`, `aidlc/spaces/default/codekb/crypto-pocket-butler/code-structure.md`.)*

**TypeScript/React.** The SPA is React 18 + TypeScript 5 on Vite 6, with TanStack Query for
server state, Tailwind CSS v4 mapped to the Parchment Ledger tokens, and `@supabase/supabase-js`
for auth. Components render; one API-client module (`web/src/api/client.ts`) owns all gateway
calls; business math (valuation, drift, allocation) is computed only in the backend. Use design
tokens, never raw colour, spacing, or type values; financial figures use tabular mono, a missing
value is an em dash rather than `0` or `$0.00`, and colour never carries meaning alone. The
support console is a separate Yarn package with its own lockfile and the same rendering rules.
*(Sources: `.claude/rules/development.md`, `design-system/DESIGN.md`,
`aidlc/spaces/default/codekb/crypto-pocket-butler/code-structure.md`.)*

**Tooling.** Yarn 4.18.1 is the only package manager, pinned through corepack: the root, `web`
and `support-console` `package.json` files all declare `packageManager: yarn@4.18.1`. The
earlier "Yarn ≥ 6" wording is retired — no Yarn 5 or 6 release exists, so the old rule could not
be followed and contradicted the pin. Rust is formatted with `rustfmt` (`rustfmt.toml`: edition
2021, `max_width = 100`, field-init shorthand) and linted with `clippy` (`clippy.toml`: MSRV
1.85, too-many-arguments 8), run as `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings`. TypeScript is linted with ESLint 9
flat config (recommended, typescript-eslint, react-hooks, `@typescript-eslint/no-explicit-any` =
error) and formatted with Prettier 3 (semicolons, double quotes, `printWidth` 100, trailing
commas) through the per-package Yarn scripts. The canonical build, lint and test command list
lives in `aidlc/spaces/default/memory/project.md` as the single authoritative source (interview
Q7). No pre-commit hook or hook manager is configured, and no naming convention beyond the
framework defaults has been stated (unresolved). *(Sources: `.claude/rules/development.md`,
`package.json`, `web/package.json`, `support-console/package.json`, `rustfmt.toml`, `clippy.toml`,
`web/eslint.config.js`, `web/.prettierrc.json`; interview Q2, Q7, Q8.)*

**Repository hygiene.** Generated Structurizr artefacts (`docs/architecture/workspace.json`,
`docs/architecture/.structurizr/`) are build outputs and stay out of git. A change stays scoped
to one unit of work, and commit messages explain why before what. *(Sources: `.gitignore`,
`docs/architecture/README.md` §3, `memory/project.md`.)*
