# Evidence — Practices Discovery

**Project:** Crypto Pocket Butler · **Intent:** `261113-authentication-session`
**Project type:** Brownfield — twelve Rust services, the API Gateway, two React front ends, a
production Swarm stack, a whole-system local Compose stack, 19 migration pairs and a large test
suite are all in the tree.
**Stage:** `inception/practices-discovery`, **Steps 2 and 5** (lead draft, then lead integration).
**Collaborators:** off for this scope (`aidlc-state.md`: "Collaborators: off (from scope
classic)"). No support-agent contributions were dispatched or expected and no contribution files
exist; the lead worked alone, the interview resolved the open points, and this run's Step 5
integrated the answers into the lead's own draft.
**Re-run context:** `aidlc/spaces/default/memory/team.md` already carries five affirmed sections
and `project.md` carries the Mandated/Forbidden rules from the two earlier practices discoveries
(`261105-brd-001-product`, `261109-brd-002-users-roles`). Those were the baseline; this run
re-checked each one against the tree rather than restating it. Corrections are marked
**[corrected]**, intentions that are not current behaviour are marked **[intention]**.
**Snapshot:** commit `112b3541355aa1c38d9370d49142a2d3fb9e5866`, branch
`feat/authentication-session-management`.

## Sources inspected

| Source | What it is | What it supports |
| --- | --- | --- |
| `.claude/aidlc-common/stages/inception/practices-discovery.md` | Stage definition | Four declared artifacts, the five section names, the `Methodology`/`Ordering` fields, the promotion flow, Step 5 integration |
| `aidlc/spaces/default/memory/org.md`, `team.md`, `project.md` | Layered rules; team and project already affirmed | The baseline re-checked here: human review, PR-to-unit link, why-before-what commits, TDD, 80% per crate, production-only Swarm, lint commands, banned stack choices |
| `aidlc/spaces/default/memory/phases/inception.md` | Inception guardrails | Given/When/Then; two alternatives per decision; no unresolved contradictions; no undeclared requirements |
| `aidlc/spaces/default/knowledge/aidlc-shared/*.md` | Team knowledge (4 files) | `stack-facts.md` pins Yarn 4.18.1, cargo-llvm-cov 0.9.1 and `supabase/postgres:15.8.1.060`; `service-map.md` gives the 12-service map and the crate anatomy; `problem-details.md` the RFC 7807 contract; `domain-glossary.md` the domain vocabulary |
| `aidlc/spaces/default/knowledge/aidlc-developer-agent/*.md` | Developer conventions (3 files) | Placement table, error mapping, decimal and chrono rules, `//!` BR-id headers, naming, test-layout per layer, migration pair rules |
| `aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md`, `frontend-testing.md` | Quality knowledge | The llvm-cov measurement trap and the paired/workspace commands; Vitest/Testing Library/jest-axe/MSW conventions |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/code-structure.md` | Reverse-engineering artifact | Crate layout, module-level BR-id headers, SPA auth module split, HTTP handler placement |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/technology-stack.md` | Reverse-engineering artifact | Manifest versions for Rust and JS; explicitly not lockfile resolutions and not a build |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/dependencies.md` | Reverse-engineering artifact | External and internal dependency edges for the auth path; states what it did not establish |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/code-quality-assessment.md` | Reverse-engineering artifact | Test inventory, static checks named but not run, absent CI, visible debt |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/architecture.md` | Reverse-engineering artifact | Hexagonal style inside services, gateway allowlist, sign-in and revocation flows, route-table/Identity mismatch |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/business-overview.md` | Reverse-engineering artifact | BRD-003 scope, what the read auth code does, the five open product decisions |
| `aidlc/spaces/default/codekb/crypto-pocket-butler/reverse-engineering-timestamp.md` | Run record | Store kind `partial`, the analysed path list, the never-run constraint |
| `inception/reverse-engineering/developer-scan-crypto-pocket-butler.md` | Developer scan for this intent | Deep-read file list, APIs discovered, technical-debt signals, handoff risks |
| `docs/brds/BRD-003-authentication-and-session-management.md` | The intent's requirement | 22 `FR-AUTH` rows, business rules, acceptance criteria, OQ-001…OQ-005 |
| `git log --oneline -25`, `git log --format='%B' -1 <sha>`, `git log --merges`, `git tag`, `git branch -vv`, `git branch -r`, `git for-each-ref --sort=-committerdate --format='%(refname:short) %(committerdate:short)'` | History and branches | Squash-merged `(#NNN)` PRs on `main`, no merge commits, why/what/Units bodies, zero tags, 106 remote branches of which 96 last touched Feb–May 2026 |
| `git log --all --oneline --diff-filter=A -- '.github/workflows/*'` and `--diff-filter=D` | Pipeline history | A GitHub Actions + Helm pipeline existed (`9f9b230`, 2026-02-22) and was deleted (`a567470`, 2026-11-01); nothing replaced it |
| Repository scan for `.github/`, `.gitlab-ci.yml`, `Jenkinsfile`, `.circleci/`, `azure-pipelines.yml`, `.buildkite`, Dependabot/Renovate | Absent artefacts | No CI platform, no dependency-update bot, no PR template, no CODEOWNERS |
| `infra/swarm-stack.yml` (1591 lines, 31 services) | Production stack | 22 update policies and 22 health checks, replica counts, 9 one-shot migrate services, `${REGISTRY:-ghcr.io/crypto-pocket-butler}/<service>:${TAG:-latest}`, Vault-backed Docker secrets, no observability service |
| `infra/prometheus/{support,job-orchestration}-rules.yml` | Alert rules | Two rule files; the header states there is no scrape configuration in the repository |
| `deployment/localhost/{docker-compose.yml,README.md,scripts/*}` | Local whole-system stack | Self-hosted Supabase, every service, nginx, the provision job, the "Known gaps" list |
| `services/*/migrations/**` | Migrations | 19 `.up.sql` with 19 matching `.down.sql`, zero orphans; Identity owns 8 |
| `services/*/src/bin/migrate.rs` (identity read in full) | Migration runner | Applies up migrations only; header says the release job runs it once before the API and worker update |
| `scripts/check-boundaries.sh` | Clean Architecture gate | Parses `cargo metadata --no-deps`; forbids outward edges and framework crates in `domain`; test-only deps exempt |
| `scripts/smoke-identity.sh` | Smoke check | `/healthz`, `/readyz`, C4 own-row (`200`, `allowed:true`) and foreign-row (`404`, `not_owner`); needs three throwaway tokens; called by nothing |
| `rustfmt.toml`, `clippy.toml`, `web/eslint.config.js`, `web/.prettierrc.json`, `support-console/.prettierrc.json` | Formatter and linter config | Concrete settings, verified verbatim |
| Root `package.json`, `web/package.json`, `support-console/package.json` | Package definitions | `packageManager: yarn@4.18.1` at all three levels; per-package script names; Playwright `^1.65.0`, Vitest `^3.0.0`; size-limit budgets; `yarn e2e` and `yarn size` in `web/` |
| `web/vitest.config.ts`, `web/vite.config.ts`, `web/playwright.config.ts`, `web/e2e/README.md`, `web/e2e/journeys/*.spec.ts` | Frontend test setup | jsdom unit config; Playwright with a mocked network layer, 3 spec files, 8 journeys |
| `web/src/test/*`, `support-console/src/test/*` | Frontend test harness | `axe.ts`, `render.tsx`, `gateway.ts`; console `supportHandlers.ts`, `fixtures.tsx`, `server.ts` |
| `services/*/tests/**`, `services/*/{domain,application,infrastructure,http}/tests/**` | Rust test trees | 92 `.rs` files: 79 suites plus 13 `common/` helpers; 176 `#[ignore]` attributes across 19 files |
| `services/gateway/domain/src/route.rs`, `path.rs` | Gateway allowlist | 34 declared routes; deny-by-default stated in the module header; no `signup-gate`, `sessions` or `mfa` entry |
| `services/identity/http/src/routes.rs`, `services/identity/application/src/sessions.rs` | Identity surface and use case | `POST /v1/signup-gate` public; session list/revoke on user-JWT routes; revoke orders provider → row → audit |
| `web/src/api/client.ts`, `web/src/auth/*`, `web/src/features/auth/*` | SPA auth surface | Gateway base `/v1`; session in memory; PKCE verifier in `sessionStorage`; `/signup-gate`, `/mfa/enrol`, `/mfa/recovery` calls |
| `web/Dockerfile` | Frontend image | Builds both Yarn bundles into one static artefact: SPA at `/`, support console at `/support` |
| `Cargo.toml` (root) | Workspace manifests | 12 service packages plus their four crates; no OpenTelemetry or OTLP dependency anywhere |
| `.gitignore`, `deployment/localhost/.gitignore`, `.env.example` | Repository hygiene | Structurizr artefacts, Playwright artefacts, local `.env`/`secrets/` excluded; `.env.example` is non-secret only |
| Previous intents' `inception/practices-discovery/*` | Prior affirmed artifacts | The baseline wording and both earlier interviews' answers (Q1–Q9, then Q1–Q8) |

Nothing was built, tested, linted or executed. `aidlc engine now` was run for the timestamp only.
Every count above is a file or line count, not a test result.

## What the fresh evidence shows, area by area

### Way of Working

**Confirmed.** November 2026 lands as single-parent, squash-merged pull requests on `main`, each
with a `(#NNN)` suffix (`784dae8`, `14742cb`, `6544576`, `25a344b`, `9f94f48`, `7f7ce1e`,
`5bc5eb2`, `e143457`, `c83276e`, `4f89603`). `git log --merges` returns nothing, so no merge
commit exists anywhere in the history. Bodies open with `Why:` then `What:`, and the AI-DLC-era
ones add `Units:` naming the ids the change serves (`14742cb`: "Units: none (developer tooling) —
makes U1–U13 runnable locally"; `784dae8`: "Units: u1-user-registry, u2-authorisation, …"). Three
commits are direct, without a PR suffix: `a567470`, `dfab8e1` and `6d83dee`.

No mechanism enforces any of it: no `.github/`, no `pull_request_template.md`, no CODEOWNERS, no
`.gitmessage`, no commit-msg hook, no husky/lefthook/pre-commit config. The unit link and the
human review are conventions.

**New evidence against the practice.** 96 of 106 remote branches were last touched between
2026-02-09 and 2026-05-14 (`copilot/*`, `feature/*`, `claude/*`, `refactor`, `add-spec-kit`) and
are still on the remote. That is far more than the "sixteen stale branches" the previous
discovery recorded, and it does not match "short-lived branches". The live branches are days old:
`main`, `feat/users-roles-identity`, `wip/u12-audit-and-walking-skeleton`, `brds`, and this
intent's `feat/authentication-session-management`. That last one was created 2026-11-13 off
`main` at `784dae8` and its first commit `112b354` is a direct commit (not a reviewed PR) that
records in-flight intent records — an intent branch that will outlive a single unit.

Zero git tags exist, so there is no release-tagging convention to infer.

**Settled at the interview (Q1).** An intent's own branch may live as long as the intent; unit
work still merges within days; and no cleanup rule is added for the 96 dormant branches, which
are treated as dormant history rather than a live strategy.

### Walking Skeleton

**Confirmed and corrected.** The slice exists: `6544576` completed U1 (gateway, Asset Registry,
SPA shell, Swarm stack), `14742cb` added `deployment/localhost` (the whole system on one machine).

**Corrected.** The previous wording said the gateway "still declares only the `/v1/assets`
paths". At `112b354`, `services/gateway/domain/src/route.rs` declares 34 routes over two owning
services: the Asset Registry paths plus Identity's principal read, user read, admin lookup, role
change, disable/restore, preferences, consent, support grants, account status, profile, deletion
request and exports. `784dae8` carried Identity through the edge.

**The remaining gap is the one this intent addresses.** The table is deny-by-default — its own
header reads `//! The declared route table. Anything not declared is denied (BR4.2).` — and these
paths are *not* in it: `/v1/signup-gate` (Identity serves it publicly), `/v1/users/{userId}/
sessions` and `/v1/users/{userId}/sessions/{sessionId}` (Identity serves them on its user-JWT
router), `/v1/mfa/enrol` and `/v1/mfa/recovery` (the SPA client calls them; no service declares
them). The SPA's base path is `/v1` (`web/src/api/client.ts:172`), so those calls hit the gateway
and are refused. `deployment/localhost/README.md` "Known gaps" still carries the older, narrower
version of this statement.

**Settled at the interview (Q2).** Both halves are affirmed and mandated: every SPA call must have
a declared gateway route with an owning service handler behind it, and reachability through those
declared routes is the completion test for a slice. The skeleton therefore remains the organising
lens for this intent, and the auth/session surface is the slice it has to finish.

### Testing Posture

**Confirmed.** TDD remains the affirmed methodology and the repository still cannot prove
write-order — 92 Rust test files, 80 frontend test files, 176 `#[ignore]` attributes over 19
files — so ordering stays a commitment. The per-layer layout matches
`aidlc-developer-agent/test-layout.md` (79 suites named for the layer plus 13 `common/` helpers
carrying fakes, fixtures, a fixed clock and a JWKS helper).

**Grown from the previous profile.** `web/` now carries a Playwright browser-journey suite (3 spec
files, 8 journeys over sign-in, dashboard and settings) added in `784dae8`. Its own README records
why it exists: the first run found that `sessionStore` was handed the Supabase client instead of
`client.auth`, so the first render threw — while every one of 312 unit tests passed against a
hand-written fake. Supabase Auth and the gateway are mocked at the network layer, the gateway mock
is exhaustive and fails on any unmocked path or any `/v1` request without a session token, and a
run needs no Docker, no database and no credentials. The earlier "there is no end-to-end browser
journey suite" line is therefore retired.

**Settled at the interview (Q3).** A green `yarn e2e` gates any change touching the auth/sign-in
surface. It is not a gate on every frontend change and it is not diagnostic-only.

**Corrected — coverage measurement.** `coverage-runbook.md` (added in `784dae8`) records that
`cargo llvm-cov -p <crate>` instruments only that package, while most suites live in the *service*
package, so a per-crate run reports a number far below reality and "the wrong number has already
sent one repair round after the wrong thing". The valid readings are the workspace run
(`--workspace --include-ignored --ignore-run-fail`) or the paired run (`-p <crate> -p <service>`),
always with the disposable database up. Older intent records still show the per-crate form
(`construction/u10-analytics/.../unit-test-instructions.md`), so the trap is still reachable.

**Settled at the interview (Q4).** The floor stays at 80% line coverage per Rust crate, measured
with the workspace or paired `cargo llvm-cov` run, and the command is quoted beside every quoted
figure.

**Still absent.** No TypeScript coverage threshold (the runbook explicitly forbids adding one as a
side effect of another change) — **still open**. No cross-service contract test — **still open**.
No committed provisioning script for the disposable database — the runbook shows the manual
`docker run supabase/postgres:15.8.1.060` with a superuser, and says "There is no committed
provisioning script yet — that is a known gap" — **still open**; the interview did not settle it.

**Still no CI.** No platform configuration exists. Two historical commits show why there is vendor
confusion available: `9f9b230` (2026-02-22) added `.github/workflows/{build-api,build-web,
build-helm,deploy-k8s}.yml` plus Helm charts for a Kubernetes deployment, and `a567470`
(2026-11-01) deleted them along with the rest of the pre-BRD repository. Those workflows built a
different application on a different platform; nothing they did survives. Dependency and image
scanning has no configuration, and there is no Dependabot or Renovate file.

**Settled at the interview (Q5).** The CI platform stays open, and the full ordered gate list is
recorded — `cargo fmt --all -- --check` → `cargo clippy --workspace --all-targets -- -D warnings`
→ `bash scripts/check-boundaries.sh` → `cargo test --workspace` (disposable database up) →
`cargo llvm-cov` → `yarn test`/`yarn lint`/`yarn build`/`yarn format:check` in both frontends →
`yarn e2e` → `yarn size` — so any platform can implement it without re-deciding.

`scripts/smoke-identity.sh` (added in `784dae8`) is a real post-deploy smoke check — health,
readiness, the C4 own-row allow and a foreign-row `404`/`not_owner` — but nothing invokes it
automatically.

### Deployment

**Confirmed.** Production only, Docker Swarm, health-checked rolling update, Vault secrets,
versioned up/down pairs, matched pairs with no orphans.

**Corrected — the observability stack is not deployed.** The previous `team.md` Deployment
section says the team deploys "the self-hosted observability stack" as Swarm services, and the
`Walking Skeleton` section says logs, metrics and traces are "visible in the self-hosted
observability stack". Neither is true in the tree:
- `infra/swarm-stack.yml` holds 31 services — 22 long-running service instances plus 9 one-shot
  migrators and `audit-retention` — and no Prometheus, Loki, Tempo, Grafana or collector.
- `grep` for those names finds only comments.
- `infra/prometheus/` holds two alert-rule files whose own header says "There is no Prometheus
  scrape configuration in this repository, so these rules assume the platform targets
  `identity-api`; where it does not, the series are uncollected and no alert can fire."
- No Cargo manifest anywhere declares `opentelemetry` or an OTLP exporter; services configure
  `tracing_subscriber::fmt()`, which logs to stdout. So there are no exported traces or metrics
  either.

What *is* true: all twelve services expose `/healthz`, `/readyz` and `/metrics`, and nothing
scrapes them. The C4 model and `docs/architecture/README.md` §10 keep the self-hosted
Loki/Prometheus/Tempo/Grafana stack as the target and as an accepted cost — that is the target,
not the current state. `docs/architecture/README.md` §13 still says the code "is the next
construction phase", which is now stale for the twelve built services.

**Settled at the interview (Q6).** The self-hosted stack stays the target, and the practice now
says plainly that structured logs exist today while metrics and traces are not collected. A
collector and scrape configuration are not made a deliverable of this intent.

**Corrected — migration and service counts.** 19 up/down pairs across eleven services (Identity
owns 8), not 13. The stack ships 9 one-shot `*-migrate` services with `restart_policy: condition:
none`, and 9 of the 11 migration-bearing services carry a `src/bin/migrate.rs`; `asset-registry`
and `audit` carry `0001_init` but neither a migrate binary nor a migrate service, so their schemas
reach production through no path in the repository. `gateway` owns no schema and correctly has
none. All 22 long-running update policies carry
`parallelism: 1`, `failure_action: rollback`, `monitor: 30s`, a `rollback_config` with
`failure_action: pause`, and `stop_grace_period: 30s`. `start-first` covers 15 services
(including `identity-worker` and `audit-retention`); the 7 remaining background workers are
`stop-first`. Replicas: 3 for `audit-api`, `identity-api`, `account-api`; 2 for `asset-registry`,
`gateway`, `web`, `portfolio`, `job-orchestration` and the other APIs; 1 for every worker,
migrator and `audit-retention`. Health checks: 20 services probe `/healthz`, `job-orchestration`
probes `/readyz`, and `audit-retention` checks the freshness of a heartbeat file. Image references
use the mutable `:${TAG:-latest}` form across all 31 services.

**Settled at the interview (Q7).** Migrate-before-update is the release-job order, and the
automatic down-migration step is an explicit deliverable of this intent rather than a future
intention. Who orders the migrate one-shot against the service update is therefore answered: the
release job does, migrate first.

**Settled at the interview (Q8).** Release is continuous from `main`, images are tagged with the
commit SHA, and no semantic-versioning ceremony runs until a first production release exists. The
mutable `:latest` default in the stack is superseded by that tag convention.

**Corrected — the two frontends are one service.** There is no `support-console` Swarm service.
`web/Dockerfile` builds both Yarn bundles so they share "one immutable static artefact and one
health-checked Swarm service": SPA at `/`, support console at `/support`.

**Unchanged intentions.** No release job exists, so digest pinning, image scanning, the automatic
down migration and the smoke step are all still to be built. Nothing applies a `.down.sql` outside
one test that `include_str!`s it.

### Code Style

**Confirmed.** Inward-only crate edges, `bash scripts/check-boundaries.sh` as a real mechanical
gate (forbids any local dependency in `*-domain`, framework crates in `domain`, adapter crates in
`application`, and the HTTP boundary in `infrastructure`), decimal arithmetic, port traits, DTO
mapping at the boundary, RFC 7807 problem bodies owned per service (11 services carry a
`http/src/problem.rs`, the gateway among them, and `job-orchestration` renders the same body from a
`problem` helper in its single `http/src/lib.rs`), `//!` headers citing BR ids
(`services/identity/application/src/sessions.rs`:
`//! WF3 — the device-session view and revocation (BR8.1, BR8.2).`).

**Confirmed with more detail than before.** The house conventions are now written down in
`aidlc-developer-agent/`: id newtypes in a `domain` `ids` module parsed once at the boundary; ports
named for the noun (`UserRepository`, `ShreddingKeyRepository`, `ConsentRepository`); adapters
prefixed `Pg` for sqlx with the protocol named for HTTP clients; migrations `NNNN_<snake_name>
.{up,down}.sql` numbered within the service with unqualified object names (the runner sets
`search_path`); test names stating the behaviour in the present tense
(`an_active_user_canonised`, `the_same_subject_hash_twice_is_refused`).

**Confirmed tooling.** Yarn 4.18.1 pinned at all three `package.json` files, with `web/yarn.lock`
and `support-console/yarn.lock` committed alongside `Cargo.lock`. `rustfmt.toml` (edition 2021,
`max_width = 100`, `use_field_init_shorthand`), `clippy.toml` (MSRV 1.85, too-many-arguments 8),
ESLint 9 flat config with `@typescript-eslint/no-explicit-any: error`, identical Prettier 3
configs in both packages (semi, double quotes, `printWidth` 100, trailing commas).

**Settled at the interview (Q9).** The memory-level corrections are accepted now (Yarn 4.18.1, the
observability stack as target); the `.claude/rules/development.md` wording edit is left to its owner
as a recorded open action; and `yarn e2e` plus `yarn size` are added to the canonical command list
in `memory/project.md`.

**Still unresolved.** No pre-commit hook, no hook manager, no `.editorconfig`.

## Interview decisions (this run, Step 4)

Nine questions were put; every one was answered. The questions and their options are in
`practices-discovery-questions.md`; these are the chosen answers, verbatim in substance.

| # | Area | Question | Chosen answer | Effect on the drafts |
| --- | --- | --- | --- | --- |
| Q1 | Way of Working | Branch lifetime and the 96 stale remote branches | **A** — an intent's own branch may live as long as the intent; unit work still merges within days; no cleanup rule for the dormant branches | `team-practices.md` Way of Working states the intent-branch exception and the no-cleanup decision; `discovered-rules.md` gains the matching ALWAYS and no deletion rule |
| Q2 | Walking Skeleton | What "done" means for a slice, and the gateway route rule | **A** — make "every SPA call has a declared gateway route and an owning service handler" mandated, and treat reachability through declared routes as the completion test for a slice | Walking Skeleton states the completion test; `discovered-rules.md` gains one ALWAYS and one NEVER |
| Q3 | Testing Posture | Does the browser journey suite gate a change? | **A** — a green `yarn e2e` gates any change touching the auth/sign-in surface | Testing Posture Quality gates lead with the `yarn e2e` gate; `discovered-rules.md` gains the matching ALWAYS and NEVER |
| Q4 | Testing Posture | How the 80% floor is measured | **A** — 80% per Rust crate, measured with the workspace or paired `cargo llvm-cov` run, with the command quoted beside every figure | Coverage bullet states the contract; `discovered-rules.md` adds the quote-the-command rule |
| Q5 | Testing Posture | CI platform and the ordered gate list | **A** — keep the platform open, and record the full ordered gate list so any platform can implement it | Quality gates carry the eight-step ordered list |
| Q6 | Deployment | Is the self-hosted observability stack still the target? | **A** — keep it as the target, and say plainly that structured logs exist today while metrics and traces are not collected | Observability bullet rewritten as target-versus-reality; the Deployment ALWAYS carries the same distinction |
| Q7 | Deployment | Migration ordering and the automatic down migration | **A** — record migrate-before-update as the release-job order, and make the automatic down step an explicit deliverable of this intent | Deployment lead paragraph and migrations bullet state the order and the deliverable; `discovered-rules.md` gains the order rule |
| Q8 | Deployment | Release cadence and image tags | **A** — continuous from `main`, images tagged with the commit SHA, no semver ceremony until a first production release | Deployment lead paragraph, Images bullet and a new Release cadence bullet; `discovered-rules.md` gains the SHA-tag rule |
| Q9 | Code Style / commands | Correcting the ambient rules file, and the canonical command list | **A** — accept the memory corrections now, leave the `.claude/rules/development.md` edit to its owner as a recorded open action, add `yarn e2e` and `yarn size` to the canonical command list | Tooling paragraph records the open action; the commands bullet lists both scripts |

No contribution files exist for this run: collaborators were off, so Step 3 was skipped and the
lead's own draft was the input to the interview.

## Re-run delta against the affirmed baseline

| Baseline statement | Fresh evidence | Effect |
| --- | --- | --- |
| Trunk-based, short-lived branches, merge within days | True for landed work; 96 of 106 remote branches are 6–9 months old | **Settled (Q1)** — kept for unit work, with an intent-branch exception and no cleanup rule |
| PR links to a requirement or unit id | Commit bodies carry `Units:`; nothing enforces it | Kept as convention |
| Walking skeleton first, Asset Registry path | Built in `6544576`, extended by `14742cb` and `784dae8` | **Settled (Q2)** — route-table reachability is now the completion test; the "only `/v1/assets`" claim stays corrected |
| Observability stack deployed on Swarm | No such service, no scrape config, no OTLP exporter | **Corrected (Q6)** — health/metrics endpoints and structured logs exist, the stack does not |
| Testing: tdd, per-layer layout, 80% per crate via `cargo llvm-cov` | Layout matches; the per-crate invocation is the trap the runbook warns about | **Corrected (Q4)** — the measurement contract is explicit and the command is quoted |
| No end-to-end browser suite | Playwright suite with 8 journeys exists (`784dae8`) | **Corrected (Q3)** — and it is now a gate on the auth surface |
| 13 migration pairs | 19 pairs across 11 services | **Corrected** |
| Support console deployed separately | Shipped inside the one `web` image at `/support` | **Corrected** |
| CI platform left open as an intention | No platform; a Kubernetes-era pipeline was added in Feb 2026 and deleted in Nov 2026 | **Settled (Q5)** — still open, with the ordered gate list recorded |
| Yarn 4.18.1 pinned through corepack | Verified at all three levels, both lockfiles committed | Confirmed; rules-file wording correction open (Q9) |
| Production only, no staging | No staging config anywhere; local Compose is documented as non-production | Confirmed |
| Swarm rolling update with `start-first` where stated | 22 policies verified verbatim | Confirmed |
| No release cadence or tag convention | Zero tags; mutable `:latest` default | **Settled (Q8)** — continuous from `main`, commit-SHA tags |

## Settled and still open

Settled at this run's interview: intent-branch lifetime and the stale-branch decision (Q1);
route-table reachability as the slice completion test, with the route rule mandated (Q2); `yarn
e2e` as an auth-surface gate (Q3); the coverage measurement contract and command quoting (Q4);
the open CI platform with a recorded ordered gate list (Q5); the observability stack as target and
logs-only reality (Q6); migrate-before-update as the release order with the automatic down step as
this intent's deliverable (Q7); continuous release from `main` with commit-SHA image tags (Q8);
the memory corrections accepted, the rules-file edit as an owner's open action, and the two new
canonical commands (Q9).

Still open, and carried forward as named uncertainty rather than practice:

1. **Rollback approval path.** A failed post-update health check is the only stated trigger. Who
   may roll back, and on what other signal, is not stated. Owner: this intent's delivery planning.
   Evidence: `infra/swarm-stack.yml` `failure_action: rollback`.
2. **Cross-service contract test.** None exists, and none is planned. Evidence:
   `aidlc/spaces/default/knowledge/aidlc-quality-agent/frontend-testing.md` and the absence of any
   contract-test directory in the tree.
3. **TypeScript coverage threshold.** None set; the coverage runbook explicitly forbids
   introducing one as a side effect of another change.
4. **Scope document staging lines.** The ideation scope document still lists a staging environment
   and launch gates C11–C12, superseded two discoveries ago and never amended. Belongs to the
   scope document's owner, not to this stage.
5. **Disposable-database provisioning script.** The blocking adapter and migration gate depends on
   a manual `docker run supabase/postgres:15.8.1.060`; `coverage-runbook.md` says "There is no
   committed provisioning script yet — that is a known gap", and the interview did not settle it.

Also recorded as an open action, not an unresolved practice: the `.claude/rules/development.md`
wording ("Yarn ≥ 6", "the observability stack" as deployed) is left to that file's owner by Q9.

## Residual risk

The practices most relied on are the least mechanically enforced. The crate-boundary script is
the only automated control in the repository; the human review, the PR-to-unit link and TDD are
conventions with no hook, and dependency and image scanning has no configuration at all. The
newest control — the Playwright suite — is the one that has actually caught a defect unit tests
could not see, and it now gates the auth surface but still covers only the SPA's sign-in,
dashboard and settings journeys. Production is the only environment and the rolling update rolls
back on a failed health check, but the rollback path, the automatic down migration and smoke have
never run, because no release job exists — and building the down step is a deliverable of this
intent precisely because of that. For this intent specifically: the authentication surface the
BRD describes is the surface the gateway does not yet declare, so the first end-to-end proof of
the work is a route-table change plus a browser journey, and neither is in a pipeline.
