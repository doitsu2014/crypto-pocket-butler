# Evidence — Practices Discovery

**Project:** Crypto Pocket Butler · **Intent:** `261109-brd-002-users-roles`
**Project type:** Brownfield — the repository contains running Rust services, two React
frontends, deployments, migrations, and a full test suite.
**Stage:** `inception/practices-discovery`, Step 5 (lead integration).
**Collaborators:** off for this scope. No support-agent contribution files were dispatched or
expected; the lead's draft and the completed interview are the record.
**Re-run context:** the previous intent (`261105-brd-001-product`) affirmed the five sections in
`aidlc/spaces/default/memory/team.md` and the hard constraints in
`aidlc/spaces/default/memory/project.md`. Those non-empty sections are the baseline here; every
change below is marked as a delta and the completed interview is the authority for it.

## Sources inspected

| Source | What it is | What it supports |
| --- | --- | --- |
| `.claude/aidlc-common/stages/inception/practices-discovery.md` | Stage definition | Required artifacts, the five section names, the `Methodology`/`Ordering` fields, promotion flow |
| `aidlc/spaces/default/memory/{org,team,project}.md` | Layered rules; team and project carry the affirmed baseline | Human review before merge; PR links to a requirement/unit id; why-before-what commits; TDD; 80% per crate; production-only Swarm; lint commands; banned stack choices |
| `aidlc/spaces/default/memory/phases/{inception,construction,operation}.md` | Phase guardrails | Given/When/Then criteria; two-alternative decisions; boundary error handling; happy path plus two error/edge cases; rollback and smoke; SLOs and alerts |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/*.md` (the six consumed artifacts plus the run record) | Reverse-engineering knowledge base at commit `14742cb` | Repository layout, crate graph, 12 services, Rust and frontend test inventory, lint/format tooling, absent CI, test gaps, technical debt |
| `git log --oneline -n 40`, `git log --format='%s' -n 200`, `git log --format='%H %p'`, `git log --format='%ad'` | History | Single-parent squash merges on `main`, `(#NNN)` pull requests, why-before-what bodies, commit dates, conventional-prefix history |
| `git branch -a`, `git for-each-ref --sort=-committerdate` | Branch layout | Current work lives on `chore/*`, `wip/*`, `docs/*` branches days old; 19 stale remote branches from March–May 2026 |
| `infra/swarm-stack.yml` (46 KB, 30 services) | Production Swarm stack | Replica counts, health checks, `update_config`/`rollback_config`, image references, Docker secrets, one-shot migrate services, smoke-token reservations |
| `deployment/localhost/docker-compose.yml` + `README.md` | Local whole-system stack | Self-hosted Supabase, every service and worker, nginx, provisioning job, known gaps (gateway routes only `/v1/assets`) |
| `services/*/Dockerfile*` (30 files) | Per-service images | API, worker, migrate and retention image split |
| `services/*/migrations/**` | Schema migrations | 13 `<NNNN>_<name>.up.sql` / `.down.sql` pairs, all matched |
| `services/account/src/bin/migrate.rs` and sibling migrate binaries | Migration runner | Applies up migrations only; header says the release job runs it once before the API and worker update; no down/revert path |
| `deployment/localhost/scripts/{provision.sh,init-env.sh}` | Local provisioning | Applies `.up.sql` files with `search_path` pinned; generates permissive local tokens |
| `scripts/check-boundaries.sh` | Clean Architecture gate | Parses `cargo metadata --no-deps`; fails on outward crate edges; test-only dependencies exempt |
| `tools/preview-architecture.sh` | Structurizr preview helper | Not a build/test/deploy tool |
| `rustfmt.toml`, `clippy.toml`, `web/eslint.config.js`, `web/.prettierrc.json` | Formatter and linter config | Concrete tool settings |
| Root `package.json`, `web/package.json`, `support-console/package.json` | Package definitions | `yarn@4.18.1` pin at all three levels; per-package script names; size-limit budgets |
| `web/vitest.config.ts`, `support-console/vitest.config.ts`, `web/src/test/*` | Frontend test setup | Vitest 3 with jsdom, globals, a setup file; Testing Library and jest-axe |
| `.gitignore`, `.dockerignore` | Repository hygiene | Structurizr artefacts and local secrets excluded |
| Repository scan for `.github/`, `.gitlab-ci.yml`, `Jenkinsfile`, `.circleci/`, scanner and coverage configs | Absent artefacts | No CI, no coverage tooling, no SAST/DAST/dependency/image scanner configuration |
| `.claude/rules/development.md`, `docs/development/README.md`, `design-system/DESIGN.md`, `CLAUDE.md` | Visible team conventions | Fixed stack, Clean Architecture rule, per-service schema/role, Swarm, Vault, tokens, figure rules |
| `docs/architecture/README.md`, `docs/brds/` | Architecture and requirements | Production-only Swarm; container model; NFR ids for scanning, availability, RPO/RTO |
| Previous intent's `inception/practices-discovery/{team-practices,discovered-rules,evidence}.md` | Prior affirmed artifacts | The baseline wording and the previous interview's nine answers (Q1–Q9) |
| `aidlc/spaces/default/intents/261109-brd-002-users-roles/inception/practices-discovery/practices-discovery-questions.md` | Completed interview for this intent | Q1–Q8 answers, which are the authority for this integration |

## Interview decisions

| Question | Area | Chosen answer | Effect on the artifacts |
| --- | --- | --- | --- |
| Q1 | Rolling updates | **A** — keep `infra/swarm-stack.yml` as it is and update the rule to match it: `order: start-first`, `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, with the service health check gating the update. | The Deployment section and the Swarm rule use the stack's configured policy, not the defaults; the per-service `start-first`/`stop-first` split is recorded. |
| Q2 | Yarn version | **B** — keep "Yarn ≥ 6" and upgrade the pin to a matching release. | Superseded by Q8: no Yarn 5 or 6 release exists, so the pin was not changed and the rule was corrected instead. |
| Q3 | CI platform | **A** — leave the platform open for now and record it as an intention. | Testing Posture says the gates run locally today and CI remains unselected; no platform was invented. |
| Q4 | Coverage tooling | **A** — `cargo llvm-cov`. | The coverage bullet and a mandated rule name `cargo llvm-cov`; the 80% per-crate target is unchanged. |
| Q5 | Integration-test database | **A** — provision a disposable Postgres per run and run the `#[ignore]` suites as a blocking gate. | The test-types bullet and a mandated rule make the ignored adapter and migration suites blocking. |
| Q6 | Down migrations | **A** — keep the rule and build the automatic down step as part of the release job. | The migrations rule and Deployment bullet keep the down step and make it a required release-job deliverable. |
| Q7 | Canonical commands | **A** — fill the placeholders in `aidlc/spaces/default/memory/project.md` and make it the single authoritative list. | Both `team-practices.md` and a mandated rule point at `memory/project.md`; the exact Rust and frontend commands are listed. |
| Q8 | Yarn "≥ 6" | **B** — correct the rule to "Yarn 4.18.1, pinned through corepack". | Code Style and the Yarn rule say 4.18.1; the old wording is retired. |

## What the fresh evidence shows

### Way of Working

- The November 2026 commits each have exactly one parent: pull requests are squash-merged
  directly onto `main`, with no merge commits. Almost all carry a `(#NNN)` suffix; three in the
  era are direct commits without one (`a567470` and `dfab8e1` on 2026-11-01, `6d83dee`).
- Commit bodies lead with `Why:` and then `What:`. Some AI-DLC-era bodies add a `Units:` line
  naming unit ids (for example commit `14742cb` closes with `Units: none (developer tooling) —
  makes U1–U13 runnable locally.`), so the requirement/unit link lives in the body rather than
  the subject.
- No commit subject in the last 200 contains a `BR-`, `FR-`, `NFR-` or `U-###` id. There is no
  pull-request template and no `.github/` directory, so the "every PR links to its requirement
  or unit id" rule is not machine-enforced.
- The 2026-11 work sits on branches created within days of landing (`chore/local-dev-stack`,
  `chore/official-aidlc`, `chore/architecture-preview`, `wip/u12-audit-and-walking-skeleton`,
  `parchment-ledger-design-system`, `simplify-architecture`). Sixteen remote branches last
  touched between March and May 2026 (`copilot/*`, `feature/*`, `refactor`, `add-spec-kit`) are
  still present and untouched for months; that is stale housekeeping, not evidence of
  long-lived feature branches.
- Older history uses conventional prefixes (`feat:`, `fix:`, `chore:`, `refactor:`, `docs:`)
  inconsistently, and a few subjects are generic ("Implement code changes to enhance
  functionality and improve performance"). No convention is stated, so none is mandated.
- There is a six-month gap between the May 2026 documentation era and the November 2026
  AI-DLC era; it reflects a project pause, not branch age.
- This run's interview did not change the way of working; Q1–Q8 covered deployment, testing,
  tooling and command location.

### Walking Skeleton

- Unit U1 (`u1-walking-skeleton`) is complete per commit `6544576`: gateway, Asset Registry,
  SPA shell, Swarm stack, migrations and the suite all exist.
- Commit `14742cb` added `deployment/localhost`: a Docker Compose stack running self-hosted
  Supabase, every service and worker, schema provisioning, and both frontends behind nginx. Its
  README states it is developer convenience, not production.
- The gateway's deny-by-default route table still declares only `/v1/assets`, `/v1/admin/assets`,
  token-contract, price-source and observation paths, so the SPA cannot reach the other services
  through `/v1`. This is recorded as a known gap in `deployment/localhost/README.md` and in the
  code knowledge base.
- Health, readiness and `/metrics` endpoints exist for every service; Prometheus rules and the
  Loki/Tempo/Grafana target are modelled and partially configured.
- This run's interview did not change the walking-skeleton stance.

### Testing Posture

- 71 `.rs` files under the `tests/` trees (suites plus shared fakes and helpers); 50 frontend
  `*.test.ts(x)` files. Hand-written port fakes live in `tests/fakes.rs` /
  `tests/common/fakes.rs`.
- Ten test files across nine services mark database-backed cases `#[ignore]`;
  `TEST_DATABASE_URL` is the variable they read. (The lead draft and the interview question said
  six suites; the current tree is the authority and shows ten files — the correction is recorded
  here rather than silently carried.) Nothing in the repository provisions that database or runs
  the ignored suites today.
- Frontend tests run through Vitest 3 (`vitest run`), with jsdom, Testing Library, jest-axe, and
  MSW in the support console. There is no browser journey suite and no contract test between
  services.
- The 80% per-Rust-crate target is now backed by a chosen tool: `cargo llvm-cov` (Q4). No
  llvm-cov configuration, per-crate threshold script or coverage job exists yet, and no
  TypeScript coverage threshold is set.
- `scripts/check-boundaries.sh` is a real mechanical gate. `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` are the stated Rust gates, and both
  frontend packages define `yarn test`, `yarn lint`, `yarn build` and `yarn format:check`.
- No CI platform exists. `docs/development/README.md` says "CI checks crate boundaries… a failing
  check fails the build", but that is an intention. Commit `a567470` deleted the earlier
  Kubernetes-era GitHub Actions workflows (`build-api.yml`, `build-helm.yml`, `build-web.yml`,
  `deploy-k8s.yml`), and nothing replaced them. Q3 deliberately left the platform open as an
  intention.
- No dependency, container-image, secret or SAST/DAST scanner is configured, although BRD-012
  NFR-SEC-14 and unit design records name `cargo audit` and Trivy.
- `memory/project.md` still has placeholder `Build`, `Lint` and `Test` fields; Q7 decided that
  file is the single authoritative command list. The exact commands are now named in
  `team-practices.md`; the placeholder fields still need filling as a promotion follow-up.

### Deployment

- `infra/swarm-stack.yml` declares 30 services: 22 long-running APIs, workers, retention and the
  SPA, plus eight one-shot `*-migrate` services with `restart_policy: none`.
- The 22 long-running services carry a `/healthz` health check (job-orchestration checks
  `/readyz`) and an `update_config` of `parallelism: 1`, `failure_action: rollback`,
  `monitor: 30s`, plus a `rollback_config` that pauses on failure. APIs, the gateway, the SPA,
  the audit retention service, job orchestration and `identity-worker` roll `start-first`; the
  remaining background workers (`account`, `portfolio`, `reporting`, `market-data`, `analytics`,
  `ledger`, `notification`) roll `stop-first`. Replicas are 1 for workers and migrators, 2 for
  the edge, registry, job orchestration and most APIs, and 3 for the audit, identity and account
  APIs. Start-first is not the literal Swarm default (`stop-first`, `failure_action: pause`).
  Q1 resolved the contradiction by keeping the stack and correcting the rule; "Swarm defaults"
  is retired.
- Images are referenced as `${REGISTRY:-ghcr.io/crypto-pocket-butler}/<service>:${TAG:-latest}`.
  The stack header says the pipeline builds and digest-pins them, but no pipeline exists, so
  digest pinning is not evidenced in the repository.
- Migrations: 13 matched up/down pairs. Each service has a `migrate` binary that applies up
  migrations from a migrator DSN; every long-running service has a sibling `*-migrate` stack
  service; the local stack applies up migrations in a `provision` job. No code path, script or
  job applies a `.down.sql`, so "automatic down migration on rollback" is policy without
  implementation. Q6 kept the policy and made the automatic down step a required release-job
  deliverable.
- No smoke script or smoke job exists. The stack comments reserve `*_smoke_token` secrets for a
  release job that is not in the repository. Deployment success is currently the health check.
- Secrets are Docker secrets mounted as files and read through `*_FILE` variables; production
  materialises them from Vault; the local stack generates permissive local tokens and is
  explicitly not a production model.
- Production is the only environment; `deployment/localhost` is developer convenience, not a
  tier. Release cadence is unstated.

### Code Style

- 12 service packages under `services/`, each a five-package Cargo workspace (four crates plus
  the composition root); the root `Cargo.toml` lists every crate. Dependencies point inward and
  the boundary script enforces it.
- Domain types are small validating value objects with a per-service `DomainError`; application
  use cases are structs over `Arc<dyn Port>` traits with `AppError` translation
  (`from_domain`, `from_repository`, `from_external`); HTTP renders RFC 7807 `ProblemDetails`.
  Module headers cite BR ids.
- Migrations are named `<NNNN>_<name>.{up,down}.sql`, one directory per service.
- rustfmt: edition 2021, `max_width = 100`, field-init shorthand. clippy: MSRV 1.85,
  too-many-arguments 8. ESLint 9 flat config with recommended, typescript-eslint and
  react-hooks, `no-explicit-any` = error. Prettier 3: semicolons, double quotes, `printWidth`
  100, trailing commas.
- Yarn is pinned as `yarn@4.18.1` in the root and both frontend packages. Q2/Q8 corrected the
  rule to match: "Yarn 4.18.1, pinned through corepack". The earlier "Yarn ≥ 6" wording is
  retired because no Yarn 5 or 6 release exists.
- No pre-commit hook or hook manager (husky, lint-staged or equivalent) is configured.
- Structurizr generated artefacts are gitignored; the design system supplies Parchment Ledger
  tokens and the no-raw-value, tabular-mono, em-dash-for-missing rules.
- Q7 assigned the canonical build, lint and test command list to `memory/project.md` as the
  single authoritative source; the per-package scripts and `docs/development/README.md` remain
  the implementation detail.

## Re-run delta against the affirmed baseline

| Baseline statement | Fresh evidence | Integrated action |
| --- | --- | --- |
| Trunk-based on `main`, short-lived branches, why-before-what commits | Confirmed: single-parent squash merges, recent branches days old, `Why:`/`What:` bodies | Unchanged; added the observed commit-body shape and the stale-branch housekeeping note |
| Walking skeleton first, Asset Registry path | Confirmed and now built (U1 complete), plus a local whole-system stack | Updated from intention to built-and-proven, with the gateway route gap named |
| TDD, tests first, same change | Affirmed; history cannot prove write-order | Methodology and Ordering unchanged; evidence made explicit about what is observable |
| Test types and runners "not chosen yet" | Runners and commands exist: `cargo test --workspace`; `yarn test/lint/build/format:check/size`; Vitest 3 | Named the runners and scripts; Q4, Q5 and Q7 closed coverage tooling, integration-database provisioning and the canonical-list location; contract testing remains open |
| 80% per crate "enforced in CI" | No coverage tool and no CI exist | Kept the target; Q4 chose `cargo llvm-cov`; CI is still an intention (Q3) |
| Quality gates: boundary check, scanning, CI platform unresolved | Boundary script is real; no CI; no scanner | Boundary gate confirmed mechanical; CI platform left open as an intention; scanning still unimplemented |
| Production-only Swarm, rolling updates "using the Swarm defaults" | Stack configures `parallelism: 1`, `failure_action: rollback`, `monitor: 30s`; APIs, edge, retention, job orchestration and `identity-worker` use `start-first`, the remaining workers use `stop-first` | Q1 kept the stack and corrected the rule to the configured policy; "Swarm defaults" is retired |
| Images digest-pinned and vulnerability-scanned before release | Stack uses mutable tags; no pipeline or scanner | Kept as intention, stated plainly as not yet true |
| Versioned up/down migrations, automatic down on rollback | 13 matched pairs and migrate binaries exist; no down-migration execution anywhere | Kept the pair rule; Q6 made the automatic down step a required release-job deliverable, still to build and test |
| Yarn ≥ 6 pinned through corepack | Repository pins `yarn@4.18.1` at all three levels | Q2/Q8 corrected the rule to Yarn 4.18.1; the old wording is retired |
| Exact Yarn script names and test commands unresolved | Both frontend packages define them; Rust uses the built-in harness | Q7 closed it: exact commands named and `memory/project.md` made the single authoritative list |

## Rule status against the repository

- **Confirmed by code or configuration**: inward crate dependency rule; per-service schema/role;
  no message queues; no RLS in services; no cross-schema grants; decimal arithmetic; React 18 /
  TypeScript 5 / Vite / TanStack Query / Supabase JS / Tailwind tokens; Vault-backed secrets;
  Swarm services with health checks; matched up/down migration pairs; boundary gate; no generated
  Structurizr artefacts in git; no hardcoded credentials found in source.
- **Decided but not yet wired**: `cargo llvm-cov` coverage measurement; the disposable-Postgres
  gate for the `#[ignore]` suites; the release-job automatic down step; the `memory/project.md`
  canonical command fields (the placeholders still need filling); the CI platform and ordered
  gate list (Q3 left the platform open); dependency and image vulnerability scanning;
  digest-pinned images; post-deploy smoke; PR-to-requirement link enforcement; pre-commit hooks.
- **Contradicted by the repository**: `docs/development/README.md`'s claim that CI checks crate
  boundaries — there is no CI. The earlier "Yarn ≥ 6" and "using the Swarm defaults"
  contradictions are resolved by the interview and are no longer listed.

## Unresolved and uncertain

1. **CI platform and ordered gate list** — no platform is configured. Q3 deliberately left the
   choice open and recorded CI as an intention; which platform to use, and the full order of
   build, fmt, clippy, test, boundary, coverage, dependency scan, image build/scan, migrate,
   deploy and smoke, remain open.
2. **TypeScript coverage threshold** — `cargo llvm-cov` covers Rust; no TypeScript coverage
   threshold or tool is set.
3. **Contract testing** — no approach or tool for API or cross-service contracts.
4. **Migration ordering** — who runs each `*-migrate` service relative to the API and worker
   update remains open. Q6 fixed the down-step rule and assigned it to the release job, but the
   step is still to be built and tested.
5. **Release cadence and versioning** — continuous or scheduled; semver, git SHA or build number
   as the image tag.
6. **Rollback triggers and approval path** — a failed health check is the only stated trigger;
   the wider trigger set and who approves a rollback are open.
7. **Feature flags** — whether flags gate deployments or remain a product-only capability.
8. **Digest pinning ownership** — who builds, signs, pins and scans images, and which registry
   credential is used.
9. **Naming conventions and local checks** — beyond framework defaults, and whether a pre-commit
   hook or local check script is required.
10. **Stale branches** — 16 remote branches last touched between March and May 2026 remain;
    whether the team wants a cleanup rule is a housekeeping decision, not a practice change.
11. **Scope document change record** — the ideation scope document still lists staging and the
    C11–C12 staging gate; the previous interview superseded them but the scope document was not
    amended. That edit belongs to the scope owner.

## Residual risk

The interview closed the coverage-tool, integration-database, down-migration and
canonical-command questions, but none of those decisions is wired yet: `cargo llvm-cov` has no
configuration, no automated run provisions the disposable Postgres, the release job does not
exist, and `memory/project.md` still carries placeholder command fields. The strongest
mechanical control in the repository remains the crate-boundary script. Production is the only
environment and the rolling update rolls back on a failed health check, but neither the rollback
path nor the down-migration step has ever been exercised, because no pipeline or smoke job
exists. The first real release should treat the coverage gate, the disposable-database gate, the
automatic down step, digest pinning and smoke as deliverables to build and test, not as
assumptions carried over from the affirmed rules.
