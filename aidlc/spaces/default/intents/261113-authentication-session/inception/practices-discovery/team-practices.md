# Team Practices — Crypto Pocket Butler

Affirmed team practice, re-verified against the tree at commit `112b354` and settled with the
person at this stage's interview (Q1–Q9). These five sections replace the matching sections of
`aidlc/spaces/default/memory/team.md` on approval. The per-claim evidence behind every statement,
including the corrections made in this run, is in `evidence.md` beside this file. Collaborators
were off for this scope, so the lead drafted from evidence alone and the interview resolved every
point the evidence could not.

## Way of Working

We work trunk-based on `main` with short-lived branches, and we merge unit work back within days
rather than weeks. The one sanctioned exception is an intent's own AI-DLC branch: it may live for
as long as the intent does, because it carries the intent's record and its Bolt boundaries rather
than a single unit. This intent's branch, `feat/authentication-session-management`, is that kind
of branch. We keep no cleanup rule for old branches — the long-dormant remote branches are
dormant history, not a live strategy, and deleting them is housekeeping nobody has to schedule.

Every pull request is small, stays scoped to one unit of work, and links to the requirement or
unit id it serves — a `BR-`/`FR-`/`NFR-` id or a `U-###` unit. A human reviews every change before
merge, and the review looks for correctness and security before style. Commit messages explain why
the change is needed before describing what changed.

History matches the practice. The November 2026 history is single-parent, squash-merged pull
requests landing directly on `main`, nearly all with a `(#NNN)` suffix (`784dae8`, `14742cb`,
`6544576`, `25a344b`, `9f94f48`, `7f7ce1e`, `5bc5eb2`, `e143457`, `c83276e`, `4f89603`); three
are direct commits without one (`a567470` and `dfab8e1` on 2026-11-01, and `6d83dee`). There are no
merge commits in the history. Commit bodies carry the why first — a `Why:` paragraph, then
`What:` — and AI-DLC-era bodies add a `Units:` line naming the unit ids the change serves. No
pull-request template, CODEOWNERS file, branch-name pattern, commit-message convention,
branch-age cap or PR size cap exists, so the unit link and the human review are team conventions
rather than mechanically enforced checks. There is no release tag convention either: the
repository carries zero git tags, and release cadence is settled below under Deployment.

Branch hygiene needs one reading in the team's own terms. Ninety-six of the 106 remote branches
were last touched between February and May 2026 (`copilot/*`, `feature/*`, `claude/*`, `refactor`,
`add-spec-kit`) and are still present. They are dormant, not current work, and we deliberately do
not add a deletion rule for them. The branches with live work are days old: `main`,
`feat/users-roles-identity`, `wip/u12-audit-and-walking-skeleton`, `brds`, and this intent's
`feat/authentication-session-management` (created 2026-11-13 off `main` at `784dae8`, its first
commit `112b354` a direct commit that records the in-flight intent records rather than landing a
reviewed change). *(Sources: `memory/team.md`, `memory/org.md`, `memory/project.md`, `git log
--oneline -25`, `git log --merges`, `git branch -vv`, `git branch -r`, `git for-each-ref
--sort=-committerdate`, `git tag`; previous practices discovery Q1, re-confirmed by the evidence
review in this run, and this run's interview Q1 for the intent-branch exception and the
no-cleanup-rule decision.)*

## Walking Skeleton

We build the thinnest end-to-end slice first, before feature breadth. Our first slice is the
Asset Registry path: the SPA shell calls the API Gateway; the gateway validates and forwards a
Supabase JWT to the Asset Registry service; that service reads and writes its own Supabase
Postgres schema through sqlx; and the whole path runs on the Docker Swarm with health and
readiness endpoints. We start with Asset Registry because it has no exchange or RPC credential
dependency and exercises the full Clean Architecture, schema, and role path. We prove that slice
end to end first, then grow the remaining services behind the same pattern.

The slice has been built, proven, and has since grown. Unit U1 (`u1-walking-skeleton`) is
complete — gateway, Asset Registry, SPA shell, the Swarm stack, the migrations and the test
suite all exist (commit `6544576`). Commit `14742cb` added `deployment/localhost`, a Docker
Compose stack that runs the whole system on one machine: self-hosted Supabase (Postgres with
pgsodium, Auth, REST, Storage, Studio, Mailpit, Kong), every Rust service and worker, the API
Gateway and Asset Registry, the schema migrations, and the SPA and support console behind nginx.
Commit `784dae8` then carried Identity through the edge: `services/gateway/domain/src/route.rs`
now declares 34 routes across two owning services — the Asset Registry paths, plus principal
resolution, user reads, the admin surface, preferences and consent, support grants, account
status, profile, deletion requests and exports for Identity. Identity is no longer reachable only
on its published port.

A slice is complete when it is reachable through the gateway's declared routes. The gateway's
table is deny-by-default: a route it does not declare is refused, and the file says so in its
own header (`//! The declared route table. Anything not declared is denied (BR4.2).`). So the
completion test for a slice is reachability through those declared routes — not a service port,
not a passing unit test, and not a route the SPA merely calls. Every SPA call must have a
declared gateway route behind it and an owning service handler behind that route.

The gap this intent has to close is the authentication and session surface, which fails that
test today. Identity serves `POST /v1/signup-gate` and `GET`/`DELETE /v1/users/{userId}/sessions…`,
and the SPA client calls `/signup-gate`, `/mfa/enrol` and `/mfa/recovery`, but none of those
paths is declared in the gateway table. They are therefore unreachable through the edge, which is
exactly the slice BRD-003 work has to finish. *(Sources: `memory/org.md` — greenfield work starts
with the thinnest end-to-end slice; `docs/architecture/README.md` §§5–6;
`deployment/localhost/README.md` "Known gaps"; `services/gateway/domain/src/route.rs`;
`services/identity/http/src/routes.rs`; `web/src/api/client.ts`; `git log` commits `6544576`,
`14742cb`, `784dae8`; previous practices discovery Q2, re-confirmed and corrected by the evidence
review in this run, and this run's interview Q2 for route-table reachability as the completion
test.)*

## Testing Posture

- **Methodology**: tdd
- **Methodology evidence**: The team affirmed test-first development for all production code at
  the first practices interview and re-confirmed it at the second; this run re-confirms it from
  fresh evidence and the person re-affirmed it at the current interview. The organisation
  baseline still applies underneath it: requirement-driven tests with a happy-path floor per
  component (`memory/org.md`), acceptance criteria in Given/When/Then form
  (`memory/phases/inception.md`), business rules tested in `application` use cases against port
  fakes, and one integration test per adapter against a real Postgres schema
  (`docs/development/README.md`). TDD governs when each of those tests is written, not what they
  cover. The repository cannot prove write-order from history — there are 92 `.rs` files under
  `services/*/tests` and the crates' own `tests/` trees (79 suite files plus 13 shared `common/`
  helpers carrying fakes, fixtures and a fixed clock), 80 frontend `*.test.ts(x)` files (42 in
  `web/`, 38 in `support-console/`), 8 Playwright browser journeys, and 176 `#[ignore]`
  attributes spread over 19 test files — so ordering remains a team commitment rather than an
  observable.
- **Ordering**: We write a failing test first for every piece of production code, then the
  implementation that makes it pass, and we land the test in the same change as the code it
  covers.
- **Test types and layout**: domain rule tests per service; application use cases against
  hand-written port fakes in `tests/fakes.rs` or `tests/common/fakes.rs`; HTTP boundary tests
  over the Axum routers with status codes and RFC 7807 problem bodies; infrastructure adapter and
  migration tests against a real Postgres schema, which are `#[ignore]`d by default and run only
  when `TEST_DATABASE_URL` is set. One file per layer, named for the layer
  (`domain.rs`, `application.rs`, `http.rs`, `repositories.rs`, `*_adapters.rs`, `composition.rs`),
  as `aidlc/spaces/default/knowledge/aidlc-developer-agent/test-layout.md` records. The ignored
  adapter and migration suites are a blocking gate, not a developer opt-in: every run provisions
  a disposable `supabase/postgres:15.8.1.060` database (superuser, because the suites create and
  drop roles) and points `TEST_DATABASE_URL` at it. The frontends use Vitest 3 with Testing
  Library and `jest-axe` (36 files assert accessibility), and MSW 2 mocks the gateway in both
  packages. `web/` also carries a Playwright browser-journey suite (`web/e2e/journeys/`, 3 spec
  files, 8 journeys over sign-in, dashboard and settings; commit `784dae8`). It drives a real
  Chromium against the Vite dev server with Supabase Auth and the gateway answered at the network
  layer, so a run needs no Docker, no database and no credentials — and the gateway mock fails a
  journey on any unmocked `/v1` call. It found a first-render crash that all 312 unit tests had
  passed over, and it is the reason the browser journeys now gate the auth surface. There is
  still no cross-service contract test (unresolved).
- **Commands and runners**: the canonical build, lint and test commands are:
  - Rust — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
    `cargo test --workspace`, and `bash scripts/check-boundaries.sh` (the mechanical Clean
    Architecture gate).
  - Frontend — in both `web/` and `support-console/`: `yarn test` (`vitest run`), `yarn lint`
    (ESLint 9 flat config), `yarn build` (`tsc -b && vite build`) and `yarn format:check`
    (Prettier). `web` also defines `yarn e2e` (`playwright test`) with `yarn e2e:report`, and
    `yarn size` (size-limit plus `scripts/check-size.mjs`: 300 kB gzip initial entry chunk,
    150 kB gzip per route chunk, 150 kB gzip stylesheet — NFR2.3).

  `aidlc/spaces/default/memory/project.md` holds this list as the single authoritative source,
  and the list now includes `yarn e2e` and `yarn size` for `web/`, so a gate run and a local run
  are the same commands. `docs/development/README.md` and the per-package `package.json` scripts
  remain the implementation detail behind it.
- **Coverage**: 80% line coverage per Rust crate remains the affirmed target, and the measurement
  contract is part of the practice, not a footnote: Rust coverage is measured with `cargo
  llvm-cov` using a reading that instruments the package carrying the tests — the workspace run
  (`cargo llvm-cov --workspace --include-ignored --ignore-run-fail …`) or the paired run
  (`cargo llvm-cov -p identity-infrastructure -p identity --include-ignored …`) — always after
  the disposable database is up so the `#[ignore]` suites contribute, and a coverage figure is
  always quoted together with the command that produced it. The contract is written down in
  `aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md` because a plain
  `cargo llvm-cov -p <crate>` instruments only that package, while most suites live in the
  *service* package, so a per-crate run reports a number far below reality — a wrong number has
  already cost one repair round. No TypeScript coverage threshold has been set (unresolved), and
  the runbook says not to introduce one as a side effect of another change. The
  happy-path-plus-two-error/edge-case floor per test file still applies and a test that always
  passes regardless of implementation is not acceptable (`memory/phases/construction.md`).
- **Quality gates**: a green `yarn e2e` gates any change touching the auth/sign-in surface —
  `web/src/auth/`, `web/src/features/auth/` and the sign-in, MFA and session journeys under
  `web/e2e/`. It is a gate there rather than everywhere because it is the only test that has
  caught a real first-render defect, and it is cheap enough (no Docker, no database, no
  credentials) to run on every such change. `bash scripts/check-boundaries.sh` is a real
  mechanical gate — it parses `cargo metadata --no-deps` and fails on any outward crate edge. We
  keep the CI platform open rather than choosing one, and we record the full ordered gate list so
  any platform can implement it without re-deciding anything:

  1. `cargo fmt --all -- --check`
  2. `cargo clippy --workspace --all-targets -- -D warnings`
  3. `bash scripts/check-boundaries.sh`
  4. `cargo test --workspace` (with the disposable database up, so the `#[ignore]` adapter and
     migration suites run)
  5. `cargo llvm-cov` under the measurement contract above, against the 80% per-crate floor
  6. `yarn test`, `yarn lint`, `yarn build`, `yarn format:check` in `web/` and `support-console/`
  7. `yarn e2e` in `web/` (required for any change touching the auth/sign-in surface)
  8. `yarn size` in `web/`

  There is still no CI platform in the repository — no `.github/`, GitLab, Jenkins, CircleCI or
  Buildkite configuration, and no Dependabot or Renovate config — so today these gates run when a
  person runs them, in this order. A pipeline did exist historically
  (`.github/workflows/build-api.yml`, `build-web.yml`, `build-helm.yml`, `deploy-k8s.yml`, added
  in `9f9b230` on 2026-02-22 for a Helm/Kubernetes deployment) and was removed in `a567470` on
  2026-11-01 when the repository was re-based on business requirements; it is not a current
  practice. Dependency and container-image scanning (`cargo audit`, Trivy) is required by BRD-012
  NFR-SEC-14 and named in unit design records but has no configuration or pipeline.
  `scripts/smoke-identity.sh` covers the Identity health and C4 authorisation paths as a
  post-deploy smoke check, but nothing calls it automatically. *(Sources: `memory/org.md`,
  `memory/phases/inception.md`, `memory/phases/construction.md`, `docs/development/README.md`,
  `aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md`, `web/package.json`,
  `support-console/package.json`, `web/e2e/README.md`, `scripts/check-boundaries.sh`, `git log`;
  interview Q3, Q4, Q5.)*

## Deployment

We deploy every backend service and the static frontend artefact as replicated Docker Swarm
services, and production is our only environment: there is no staging tier and no promotion step
between environments. A release rolls out as a rolling update in which each service's health
check gates the update. We release continuously from `main`: every merge is a candidate release,
release images are tagged with the commit SHA, and there is no semantic-versioning ceremony until
a first production release exists. Schema changes ship as versioned sqlx up/down migration pairs;
the release job runs the service's migrate one-shot before the API and worker are updated, and
rolling a release back applies the matching down migration as an automatic step of the release
job. Every deployment procedure carries a rollback path. Secrets reach services from Supabase
Vault (pgsodium), never from env files, source, or logs.

- **Environments**: production only, on the Docker Swarm cluster. `deployment/localhost` is a
  developer convenience on one machine, not an environment tier and not a promotion step; its
  README says so explicitly and adds that nothing there is a production credential or deployment
  model. The ideation scope document's staging environment and its C11–C12 staging launch gate
  were dropped at the first interview and no longer apply; the scope document itself still needs
  that change recorded (unresolved, owned by the scope document's owner).
- **Deployment strategy**: `infra/swarm-stack.yml` (1591 lines, 31 services) is explicit, not
  defaulted. All 22 long-running services carry an update policy of `parallelism: 1`,
  `failure_action: rollback`, `monitor: 30s`, a `rollback_config` with `failure_action: pause`,
  and a `stop_grace_period` of 30s. Order is `start-first` for `asset-registry`, `gateway`, `web`,
  `audit-api`, `audit-retention`, `identity-api`, `identity-worker`, `account-api`, `portfolio`,
  `job-orchestration`, `reporting-api`, `market-data-api`, `analytics-api`, `ledger-api` and
  `notification-api`; it is `stop-first` for the seven background workers (`account-worker`,
  `portfolio-worker`, `reporting-worker`, `market-data-worker`, `analytics-worker`,
  `ledger-worker`, `notification-worker`). Health checks gate every update: 20 services probe
  `/healthz`, `job-orchestration` probes `/readyz`, and `audit-retention` checks the freshness of
  a heartbeat file instead. Replicas are 3 for `audit-api`, `identity-api` and `account-api`; 2 for
  `asset-registry`, `gateway`, `web`, `portfolio`, `job-orchestration` and the remaining APIs; 1
  for every worker, every migrator and `audit-retention`. No blue-green or canary path is adopted.
  *(`infra/swarm-stack.yml`, verified in this run.)*
- **The two frontends are one service**: the SPA and the support console ship in a single `web`
  image — `web/Dockerfile` builds both Yarn bundles so they share one immutable static artefact —
  with the SPA at `/` and the support console at `/support`. There is no separate support-console
  Swarm service. *(`web/Dockerfile`; `infra/swarm-stack.yml`; `deployment/localhost/README.md`.)*
- **Images**: 30 Dockerfiles across `services/*` split each service into api, worker, migrate and
  (for audit) retention images. The stack references
  `${REGISTRY:-ghcr.io/crypto-pocket-butler}/<service>:${TAG:-latest}`, so a deployment always
  pins a specific image tag rather than relying on the `:latest` default; that tag is the commit
  SHA of the merge being released. The stack's header comment says a pipeline builds and
  digest-pins each image; the pipeline is not in the repository yet, so digest pinning and
  pre-release vulnerability scanning remain intended rather than current, while the SHA tag is
  the settled convention.
- **Release cadence**: continuous from `main`. A merge that passes the gate list is a release
  candidate, and its images are tagged with the merge commit's SHA. No git tags are cut and no
  semantic version is assigned until a first production release, so the zero-tag history is
  expected rather than a gap.
- **Database migrations**: 19 versioned `<NNNN>_<name>.{up,down}.sql` pairs live across eleven
  services' `migrations/` directories — Identity carries 8 of them (`0001_init` through
  `0008_support_mine_index`). Every `.up.sql` has its `.down.sql`; no orphan was found. Nine of
  the eleven migration-bearing services ship a `migrate` binary (`src/bin/migrate.rs`) whose
  header says "The release job runs this once against the migrator DSN before the API and worker
  are updated", and the stack ships a one-shot `<service>-migrate` service for each of the nine
  with `restart_policy: condition: none`. That order is the release-job order: migrate first, then
  update the API and worker. Two migration-bearing services are not covered yet: `asset-registry`
  and `audit` each ship `0001_init` but have neither a `migrate` binary nor a `*-migrate` service,
  so their schemas are applied by the local provision job only. The automatic down-migration step
  is an explicit deliverable of this intent, not a future intention: the release job's rollback
  path must apply the matching `.down.sql`. Nothing in the repository applies a `.down.sql` today
  — the only reference is a test that `include_str!`s one — so the step is to be built and tested
  here, alongside the Identity migrations BRD-003 adds.
- **Release completion**: `scripts/smoke-identity.sh` exists and checks `/healthz`, `/readyz`, the
  C4 own-row decision (expects `200` and `"allowed":true`) and a foreign-row decision (expects
  `404` and `not_owner`). It needs three throwaway tokens and is not called by any pipeline, so a
  deployment is still considered healthy when the service health check passes and the smoke
  script has been run. The stack comments reserve `*_smoke_token` secrets for a release job that
  is not in the repository.
- **Secrets**: services read credentials through `*_FILE` environment variables pointing at
  Docker secrets; production materialises those from Supabase Vault. `.env.example` carries
  non-secret configuration only and says so. The local stack generates its own permissive local
  tokens into gitignored `.env` and `secrets/` directories and is explicitly not a production
  credential model.
- **Observability**: the self-hosted Loki/Prometheus/Tempo/Grafana stack is our target, and it is
  not deployed. What exists today is structured logs: all twelve services configure
  `tracing_subscriber::fmt()` and log to stdout, and each exposes `/healthz`, `/readyz` and
  `/metrics`. What does not exist is collection — the Swarm stack contains no Prometheus, Loki,
  Tempo or Grafana service, there is no scrape configuration anywhere in the repository
  (`infra/prometheus/` holds two alert-rule files and their header says plainly "There is no
  Prometheus scrape configuration in this repository"), and no Cargo manifest pulls an
  OpenTelemetry or OTLP exporter, so metrics and traces are not collected. The stack is modelled
  in the C4 and named as an architecture decision; standing it up, with a collector and a scrape
  configuration, is future work. Nothing logs credentials or tokens. *(Sources:
  `infra/swarm-stack.yml` — 31 services, none of them the observability stack;
  `infra/prometheus/support-rules.yml` header; root `Cargo.toml`; `docs/architecture/README.md`
  §10; interview Q6.)*
- **Reliability targets**: 99.9% monthly availability for paid plans; RPO ≤24h and RTO ≤4h;
  SEV-1 communication within 15 minutes (BRD-012 NFR-AVL-01/04/05/06).
- **Progressive delivery**: feature flags with owner, expiry and rollout percentage are a product
  capability (BRD-010 FR-ADM-18, BRD-021 FR-ANL-10); whether they gate deployments is unresolved.
- **Rollback triggers**: a failed post-update health check is the one stated trigger, and the
  stack's `failure_action: rollback` implements it for long-running services. A rollback runs the
  release job's rollback path, which applies the matching down migration. The wider set of
  production rollback triggers and the approval path for a rollback are not stated (unresolved).

## Code Style

We keep the codebase boring, explicit, and boundary-respecting.

**Rust.** Every service is a Cargo workspace of four crates — `domain`, `application`,
`infrastructure`, `http` — and dependencies point inward only. `domain` holds entities, value
types, and business rules and imports no framework; `application` orchestrates use cases behind
port traits; `infrastructure` implements those ports (sqlx repositories, Vault, provider
clients, JWKS); `http` holds Axum routers, middleware, and DTOs. DTOs map to/from domain
types at the `http` boundary and never leak inward. Cross-boundary needs are expressed as ports,
not concrete imports. Quantities and values use decimal arithmetic (`rust_decimal`), never
binary floating point. Errors are typed per service (`DomainError` inward, `AppError` mapping at
`from_domain` / `from_repository` / `from_external`) and rendered at the boundary as RFC 7807
`ProblemDetails` — eleven of the twelve services own a `http/src/problem.rs` and the gateway is one
of them; `job-orchestration` renders the same body from a `problem` helper inside its single
`http/src/lib.rs`. The near-duplication across services is deliberate. Errors are surfaced or
logged, never swallowed.
Migrations use `<NNNN>_<name>.{up,down}.sql` pairs with unqualified object names (the runner sets
`search_path`). Module-level `//!` headers explain purpose and cite the BR ids they serve.
*(Sources: `.claude/rules/development.md`, `docs/development/README.md`,
`docs/architecture/README.md`, `aidlc/spaces/default/codekb/crypto-pocket-butler/code-structure.md`,
`aidlc/spaces/default/knowledge/aidlc-shared/problem-details.md`.)*

**TypeScript/React.** The SPA is React 18 + TypeScript 5 on Vite 6, with TanStack Query for
server state, Tailwind CSS v4 mapped to the Parchment Ledger tokens, and `@supabase/supabase-js`
for auth. Components render; one API-client module (`web/src/api/client.ts`) owns all gateway
calls; business math (valuation, drift, allocation) is computed only in the backend. Use design
tokens, never raw colour, spacing, or type values; financial figures use tabular mono, a missing
value is an em dash rather than `0` or `$0.00`, and colour never carries meaning alone. The
support console is a separate Yarn package with its own lockfile — it ships in the same `web`
image at `/support`, but it keeps its own dependency set and the same rendering rules.
*(Sources: `.claude/rules/development.md`, `design-system/DESIGN.md`,
`aidlc/spaces/default/codekb/crypto-pocket-butler/code-structure.md`, `web/Dockerfile`.)*

**Tooling.** Yarn 4.18.1 is the only package manager, pinned through corepack: the root, `web`
and `support-console` `package.json` files all declare `packageManager: yarn@4.18.1`, and both
frontend lockfiles are committed. The earlier "Yarn ≥ 6" wording in `.claude/rules/development.md`
is retired — no Yarn 5 or 6 release exists, so it could not be followed and contradicted the pin;
the pinned version is the practice, and correcting that sentence in the rules file is recorded as
an open action for its owner. Rust is formatted with `rustfmt` (`rustfmt.toml`: edition
2021, `max_width = 100`, field-init shorthand) and linted with `clippy` (`clippy.toml`: MSRV
1.85, too-many-arguments 8), run as `cargo fmt --all -- --check` and
`cargo clippy --workspace --all-targets -- -D warnings`. TypeScript is linted with ESLint 9
flat config (recommended, typescript-eslint, react-hooks, `@typescript-eslint/no-explicit-any` =
error) and formatted with Prettier 3 (semicolons, double quotes, `printWidth` 100, trailing
commas; identical `.prettierrc.json` in both packages) through the per-package Yarn scripts. The
canonical build, lint and test command list lives in `aidlc/spaces/default/memory/project.md` as
the single authoritative source. The house naming and placement conventions are recorded in
`aidlc/spaces/default/knowledge/aidlc-developer-agent/`
(`rust-conventions.md`, `sqlx-migrations.md`, `test-layout.md`): id newtypes in a `domain` `ids`
module parsed once at the boundary, ports named for the noun (`UserRepository`), adapters
prefixed `Pg` for sqlx, test files named for the layer, and test names that state the behaviour
in the present tense. No pre-commit hook or hook manager is configured and no `.editorconfig`
exists. *(Sources: `.claude/rules/development.md`, `package.json`, `web/package.json`,
`support-console/package.json`, `rustfmt.toml`, `clippy.toml`, `web/eslint.config.js`,
`web/.prettierrc.json`, `support-console/.prettierrc.json`; previous interview Q7, Q8; this run's
interview Q9.)*

**Repository hygiene.** Generated Structurizr artefacts (`docs/architecture/workspace.json`,
`docs/architecture/.structurizr/`) are build outputs and stay out of git, as do the Playwright
report, traces and screenshots under `web/e2e/.artifacts/` and `web/e2e/screenshots/`, and the
local stack's generated `.env`, `secrets/` and Kong config. A change stays scoped to one unit of
work, and commit messages explain why before what. *(`.gitignore`,
`deployment/localhost/.gitignore`, `docs/architecture/README.md` §3, `memory/project.md`.)*
