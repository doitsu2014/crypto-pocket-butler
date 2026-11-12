# Build and Test Summary — 261109-brd-002-users-roles

**Scope:** `classic` · ten Units (`u1-user-registry` … `u10-admin-users-ui`) · brownfield ·
Standard test strategy · TDD methodology · Depth Standard.
**Stage:** 3.6 Build and Test, inline, lead `aidlc-quality-agent`.
**Assembly:** the ten Units' generated source is in the working tree; this stage builds it, runs
every documented command once, and reports what it found.

Build and test instructions: `build-instructions.md` (assembly, gates, troubleshooting) and
`integration-test-instructions.md` (the key boundaries and cross-Unit seams). Results and failure
details: `test-results.md`. ID-level coverage: `cross-unit-traceability.md`.

## 1. Overall build status and prerequisites

Status: **failure** — on the loop-back replay the workspace builds, formats, lints and passes its
non-ignored suites, and one of the two database-gate failures is fixed, but the boundary gate, the
remaining database-gate failure, both frontend format gates, five touched crates' coverage floor and
the ID-level traceability gate still do not pass, and the Structurizr model could not be validated
here. Verdicts are finalized row by row in §4; failure details and the escalation record are in
`test-results.md`.

Prerequisites (full list in `build-instructions.md` §1): Rust (2021, MSRV 1.85) with the
`llvm-tools` component and `cargo-llvm-cov` 0.9.1; Node 24 with Yarn 4.18.1 through corepack;
Docker for the disposable PostgreSQL the `#[ignore]` adapter and migration suites need.
No secret and no running service is required for the default suites.

Assembled project: 60 Rust crates (15 belong to the services this intent changed), two Yarn
packages (`web/`, `support-console/`), and nine Identity migrations (`0001`–`0008` plus the
earlier pair) in up/down pairs.

## 2. Test type inventory

| Test type | Generated? | What carries it |
|---|---|---|
| Unit — Rust | yes | each service's `tests/` targets: domain rules, application use cases against hand-written port fakes, HTTP boundary over the real Axum router |
| Unit — TypeScript | yes | `web/` and `support-console/` Vitest 3 suites with Testing Library and jest-axe |
| Integration — boundaries | yes | `integration-test-instructions.md`: gateway route table/limiter/upstream, cross-service Account/Portfolio → Identity, frontend client against MSW in jsdom |
| Adapter and migration (real Postgres) | yes | the `#[ignore]` suites, run as a **blocking** gate against a disposable database with `TEST_DATABASE_URL` |
| Accessibility | yes | `jest-axe` on every rendered state, in **jsdom** |
| Coverage | yes | `cargo llvm-cov` on the full suite, 80 % line coverage per crate |
| Performance / load | no (not generated) | Standard strategy writes no performance instruction file; the units' own performance requirements assign the budgets to timeouts, deadlines and production histograms with alerts, and say no benchmark suite is introduced |
| Security (SAST/DAST) | no (not generated) | Standard strategy writes no security instruction file; the security targets are asserted inside the Unit suites, and dependency/image scanning has no pipeline in this plan (`ci-pipeline` is skipped) |
| Contract | no | the repository has no cross-service contract-test suite (unresolved in team memory) |
| E2E / real browser | no | no Playwright/Chromium journey exists anywhere in the repository |
| CI-before-merge | no | the plan runs no CI Pipeline |

## 3. Coverage expectations

| Scope | Expectation | Source |
|---|---|---|
| Each Rust crate | ≥ 80 % line coverage, measured with `cargo llvm-cov` on the full suite with the ignored suites included | `memory/project.md`, `memory/team.md` |
| Lines a Unit adds | the same crate floor; a crate's number is the crate's, not the Unit's | per-Unit `unit-test-instructions.md` |
| TypeScript | no numeric threshold (unresolved in team memory); per-file floor of happy path plus two error/edge cases | `memory/team.md`, `memory/phases/construction.md` |
| Per test file | happy path plus at least two error or edge cases; no test that passes regardless of implementation | `memory/phases/construction.md` |

The crates this intent touches: `identity-{domain,application,infrastructure,http}` and the
`identity` composition root; `gateway-{domain,application,infrastructure,http}` and `gateway`;
`account-{domain,application,infrastructure,http}` and `account`; `portfolio-{domain,application,
infrastructure,http}` and `portfolio`. `web/` and `support-console/` are TypeScript only.

## 4. Target Verification Matrix

Rows group the requirement ids that share one producing check; the per-id mapping for each Unit is
that Unit's `code-generation/traceability.json`, and `cross-unit-traceability.md` carries the
id-level view produced by the traceability sensor. `Owning Stage` names the later stage that owns a
target this stage cannot execute, when one exists in the plan.

### 4.1 Build and project-wide gates

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| BT-B1 | `memory/project.md` Build; `build-instructions.md` §3 | `cargo build --workspace` exit 0 | exit 0, 63 crates compiled | `test-results.md` §1 | — | Met |
| BT-B2 | `memory/project.md` Build; `build-instructions.md` §3 | `yarn --cwd web build` (`tsc -b && vite build`) exit 0 | exit 0 | `test-results.md` §1 | — | Met |
| BT-B3 | `memory/project.md` Build; `build-instructions.md` §3 | `yarn --cwd support-console build` exit 0 | exit 0 | `test-results.md` §1 | — | Met |
| BT-Q1 | `memory/project.md` Lint; Interview Q8 | `cargo fmt --all -- --check` exit 0 | exit 0, no diff | `test-results.md` §1 | — | Met |
| BT-Q2 | `memory/project.md` Lint; Interview Q8 | `cargo clippy --workspace --all-targets -- -D warnings` exit 0 | exit 0, no warnings | `test-results.md` §1 | — | Met |
| BT-Q3 | `rules/development.md` Clean Architecture; `team.md` Quality gates | `bash scripts/check-boundaries.sh` exit 0 — no outward crate edge | exit 1: `reporting-domain must not import framework crates: ['serde', 'serde_json']` | `test-results.md` §3 F1 | — | Not Met |
| BT-Q4 | `memory/project.md` Test | `cargo test --workspace` exit 0, zero failed | exit 0: 1 659 passed, 0 failed, 154 ignored | `test-results.md` §2 | — | Met |
| BT-Q5 | Interview Q5; `memory/project.md` Test | the `#[ignore]` adapter and migration suites pass against a disposable Postgres with `TEST_DATABASE_URL` set — blocking gate | exit 101: 91 passed, 1 failed (`protect_and_unprotect_…` in `identity/tests/repositories.rs` — F2b) | `test-results.md` §3 F2 | — | Not Met |
| BT-Q6 | Interview Q4; `memory/project.md` Test | `cargo llvm-cov` runs on the full suite and every touched crate reports ≥ 80 % line coverage | ran; workspace 70.99 %; five touched crates below 80 % | `test-results.md` §4 | — | Not Met |
| BT-F1 | `memory/project.md` Test | `yarn --cwd web test` exit 0 | exit 0: 312 passed in 42 files | `test-results.md` §2 | — | Met |
| BT-F2 | `memory/project.md` Lint | `yarn --cwd web lint` exit 0 | exit 0 | `test-results.md` §2 | — | Met |
| BT-F3 | `memory/project.md` Lint | `yarn --cwd web format:check` exit 0 | exit 1: 81 files differ (41 source, 40 generated `dist`) | `test-results.md` §3 F3 | — | Not Met |
| BT-F4 | `yarn --cwd web size`; performance-design §3 budgets | initial entry ≤ 300 kB gzip, route chunk ≤ 150 kB, stylesheet ≤ 150 kB | initial 101.2 kB gzip over 2 files; 27 route chunks ≤ 150 kB; budgets met | `test-results.md` §2 | — | Met |
| BT-F5 | `memory/project.md` Test | `yarn --cwd support-console test` exit 0 | exit 0: 272 passed in 38 files | `test-results.md` §2 | — | Met |
| BT-F6 | `memory/project.md` Lint | `yarn --cwd support-console lint` exit 0 | exit 0 | `test-results.md` §2 | — | Met |
| BT-F7 | `memory/project.md` Lint | `yarn --cwd support-console format:check` exit 0 | exit 1: 11 files differ (all pre-existing) | `test-results.md` §3 F3 | — | Not Met |
| BT-I1 | `integration-test-instructions.md` §3 | the gateway route table, limiter and upstream forwarding suites pass | pass inside `cargo test --workspace` | `test-results.md` §2 | — | Met |
| BT-I2 | `integration-test-instructions.md` §5 | the cross-service Account/Portfolio → Identity metadata suites pass | pass inside `cargo test --workspace` | `test-results.md` §2 | — | Met |
| BT-A1 | `memory/phases/construction.md`; NFR25.1–25.4 | jest-axe reports no violation on any rendered state | every `jest-axe` assertion in both suites passes (jsdom) | `test-results.md` §2 | — | Met |
| BT-T1 | Testing Contract, `obligations.strategy` (`standard`) | five to eight tests per component, unit plus integration for key boundaries | 1 659 Rust tests over 226 suites, 584 frontend tests over 80 files | `test-results.md` §2 | — | Met |
| BT-T2 | Testing Contract, `methodology` (`tdd`) | a failing test lands with each piece of production code, and the failing run is recorded | write order is not observable in history (team memory says so); one Unit's summary was found to overstate its Red/Green record | `test-results.md` §2, u9 review record | — | Unverified |
| BT-C1 | Testing Contract / memory "CI execution before merge" | the selected tests run in CI before merge | the plan lists `ci-pipeline` as SKIP, so this stage is the run | `test-results.md` §3 F5 | — | N/A |
| BT-D1 | `docs/architecture/README.md` §12 | the Structurizr model validates | validator reaches the model but cannot load the default theme: `static.structurizr.com: Name does not resolve` | `test-results.md` §3 F4 | — | Unverified |

### 4.2 Per-Unit targets

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| BT-U1-S | `u1-user-registry/…/unit-test-instructions.md` | `domain`, `application`, `http` and repository suites for the user registry pass | domain/application/http pass; the repository suite fails 1 of 36 (F2b) | `test-results.md` §3 F2 | — | Not Met |
| BT-U1-N | `u1…/traceability.json` | AC1.x, AC4.1, AC7.1, BR1.x and NFR2/9/10/19/22/23/24 ids traced `OK` are exercised; the deferred id stays deferred | every id covered; AC1.3.2 covered through U2 | `cross-unit-traceability.md` | — | Met |
| BT-U2-S | `u2-authorisation/…/unit-test-instructions.md` | the four authorisation suites plus the scoped repository cases pass | all pass | `test-results.md` §2 | — | Met |
| BT-U2-N | `u2…/traceability.json` | AC1.3, AC2.x, AC7.x, BR2.x and NFR2/9/10/17/19/22/23/24 ids traced `OK` are exercised | AC5.1.1–AC5.3.2 (and FR18.1–18.3) are `Deferred` in U2's file: the portfolio-link storage they describe belongs to the Account and Portfolio services, outside this intent | `cross-unit-traceability.md` §5 | an Account/Portfolio intent | Unverified |
| BT-U3-S | `u3-support-access/…/unit-test-instructions.md` | domain, application, HTTP, the two cross-service metadata suites and the gateway suites pass; `support_adapters` runs under the DB gate | all pass, including the DB gate | `test-results.md` §2 | — | Met |
| BT-U3-N | `u3…/traceability.json` | AC3.x, AC7.3, FR16.x, BR3.x and NFR1/2/9/10/11/17–20/22–24 ids traced `OK` are exercised | every FR16 and AC3 id covered | `cross-unit-traceability.md` | — | Met |
| BT-U4-S | `u4-preferences-consent/…/unit-test-instructions.md` | consent domain, application, HTTP and the consent repository/migration cases pass | all pass | `test-results.md` §2 | — | Met |
| BT-U4-N | `u4…/traceability.json` | AC6.x, BR4.x and NFR1/2/9/10/17/19/20/22–24 ids traced `OK` are exercised | all U4 ids covered; AC6.4 belongs to U6 and is unbuilt there | `cross-unit-traceability.md` | — | Met |
| BT-U5-S | `u5-account-lifecycle/…/unit-test-instructions.md` | lifecycle domain, application, HTTP and adapter suites pass | all pass | `test-results.md` §2 | — | Met |
| BT-U5-N | `u5…/traceability.json` | AC4.x, AC7.2, BR5.x and NFR1/2/9/10/18–24 ids traced `OK` are exercised; AC4.4.2 stays `Deferred` | AC4.4.2 (and FR17.4) uncovered — legal hold has no owning Unit in this plan | `cross-unit-traceability.md` | later compliance unit (not in this plan) | Unverified |
| BT-U6-S | `u6-jurisdiction-gate/…/unit-test-instructions.md` | jurisdiction domain, application, HTTP, repository cases and the gateway `client_ip` suite pass | all pass | `test-results.md` §2 | — | Met |
| BT-U6-N | `u6…/traceability.json` | BR1.x, NFR1.4, NFR2.2–2.4, NFR19.1/2, NFR23.1–23.3, C8 and C9 ids traced `OK` are exercised | AC6.4.1 and AC6.4.2 are claimed but have no coverage row (the code that satisfies them exists in `principal.rs:176`); three NFR23 entries are not workspace-relative paths | `cross-unit-traceability.md` §3, §6 | — | Not Met |
| BT-U7-S | `u7-settings-ui/…/unit-test-instructions.md` | Identity `http`/`application`/`repositories`, gateway `domain`/`http`, and the three `web` filters pass | the `web` filters and the other crates pass; the Identity repository suite fails 1 of 36 (F2b) | `test-results.md` §3 F2 | — | Not Met |
| BT-U7-N | `u7…/traceability.json` | AC1.4, AC4.3, AC6.1, NFR2/9/10/11/21/22/24–26 ids traced `OK` are exercised | every U7 id covered | `cross-unit-traceability.md` | — | Met |
| BT-U8-S | `u8-signup-gates-ui/…/unit-test-instructions.md` | the signup-gates feature suite, `gates.test.ts`, `RequireGatesPassed.test.tsx` and the client regression pass | all pass | `test-results.md` §2 | — | Met |
| BT-U8-N | `u8…/traceability.json` | AC6.2/6.3/6.5, NFR2/9/10/11/22/24–26 ids traced `OK` are exercised | every U8 id covered | `cross-unit-traceability.md` | — | Met |
| BT-U9-S | `u9-support-console-ui/…/unit-test-instructions.md` | the console support suites, gateway `domain`/`http`, `support_application`/`support_http` and `support_adapters` (DB gate) pass | all pass, including the DB gate | `test-results.md` §2 | — | Met |
| BT-U9-N | `u9…/traceability.json` | NFR2/9/10/11/21–26 ids traced `OK` are exercised; NFR26.2 stays `Deferred` | NFR26.2 uncovered — the Vietnamese half of the console has no owning Unit | `cross-unit-traceability.md` | console-wide localisation unit (does not exist yet) | Unverified |
| BT-U10-S | `u10-admin-users-ui/…/unit-test-instructions.md` | the five Rust crates, the admin console suites and the three regression suites pass | all pass | `test-results.md` §2 | — | Met |
| BT-U10-N | `u10…/traceability.json` | AC7.1, NFR2/9/10/11/21–26 ids traced `OK` are exercised; NFR26.2 stays `Deferred` | NFR26.2 uncovered — same as U9 | `cross-unit-traceability.md` | console-wide localisation unit (does not exist yet) | Unverified |

### 4.3 Coverage, per touched crate

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| BT-CV1 | `memory/project.md` Test; Interview Q4 | `identity-domain` ≥ 80 % lines | 90.97 % (2 106/2 315) | `test-results.md` §4 | — | Met |
| BT-CV2 | as above | `identity-application` ≥ 80 % lines | 90.35 % (3 174/3 513) | `test-results.md` §4 | — | Met |
| BT-CV3 | as above | `identity-infrastructure` ≥ 80 % lines | 64.95 % (1 716/2 642) | `test-results.md` §4 | — | Not Met |
| BT-CV4 | as above | `identity-http` ≥ 80 % lines | 80.45 % (2 925/3 636) | `test-results.md` §4 | — | Met |
| BT-CV5 | as above | `identity` (composition root) ≥ 80 % lines | 84.65 % (623/736) | `test-results.md` §4 | — | Met |
| BT-CV6 | as above | `gateway-domain` ≥ 80 % lines | 92.80 % (232/250) | `test-results.md` §4 | — | Met |
| BT-CV7 | as above | `gateway-application` ≥ 80 % lines | 91.58 % (174/190) | `test-results.md` §4 | — | Met |
| BT-CV8 | as above | `gateway-infrastructure` ≥ 80 % lines | 68.72 % (145/211) | `test-results.md` §4 | — | Not Met |
| BT-CV9 | as above | `gateway-http` ≥ 80 % lines | 83.84 % (358/427) | `test-results.md` §4 | — | Met |
| BT-CV10 | as above | `gateway` (composition root) ≥ 80 % lines | 3.23 % (3/93) | `test-results.md` §4 | — | Not Met |
| BT-CV11 | as above | `account-*` crates touched by the support-metadata boundary ≥ 80 % lines | domain 92.01 % and application 88.26 % Met; http 79.35 % (780/983) Not Met; infrastructure untouched | `test-results.md` §4 | — | Not Met |
| BT-CV12 | as above | `portfolio-*` crates touched by the support-metadata boundary ≥ 80 % lines | domain 90.69 % Met; http 45.38 % (629/1 386) Not Met; application and infrastructure untouched | `test-results.md` §4 | — | Not Met |

### 4.4 Targets this stage records but cannot make `Met`

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| BT-X1 | frontend acceptance criteria of U7–U10 (AC6.1, AC6.2, AC6.3, AC7.1) | the sign-up gate, settings, support console and admin journeys behave as specified | specified behaviour is exercised in jsdom only; no real browser engine ran | `test-results.md` §2 | — | Unverified (browser note) |
| BT-X2 | AC4.4.2 legal hold (BRD-017 register + statutory retention schedule) | erasure respects a legal hold | not built by any Unit in this plan (uncovered as `AC4.4.2`, FR17.4) | `cross-unit-traceability.md` | later compliance unit (not in this plan) | Unverified |
| BT-X3 | AC1.3.2 / BR1.3 (U1 deferred) | the deferred user-registry behaviour | built and covered through U2 | `cross-unit-traceability.md` | — | Met |
| BT-X4 | AC5.1–AC5.3 (U2 deferred) | the deferred authorisation acceptance criteria | six ids unbuilt (FR18.1–18.3) | `cross-unit-traceability.md` | later Unit (not in this plan) | Unverified |
| BT-X5 | BR5.7 (U5 deferred) | the read-versus-write predicate for a pending-deletion account | not traced by any Unit's traceability | `cross-unit-traceability.md` | U2 follow-up (not in this plan) | Unverified |
| BT-X6 | NFR26.2 (U9, U10 deferred) | the English/Vietnamese console half | not built by either console Unit | `cross-unit-traceability.md` | console-wide localisation unit (does not exist yet) | Unverified |

## 5. Readiness assessment

| Dimension | Assessment |
|---|---|
| Build-ready | **No** — the workspace builds and lints clean, but the Clean Architecture boundary gate fails on `reporting-domain` and both frontend format gates are red |
| Test-ready | **No** — the non-ignored suites are green, but the blocking database gate still fails the pgsodium key-destruction case, five touched crates sit under the 80 % line floor, and 17 requirement ids are uncovered or unbuilt |
| Deployment-ready | **No** — the C4 model could not be validated here, and the one still-failing database-gate case is the crypto-shredding guarantee itself |

## 6. Known limitations and outstanding items

- **The plan runs no CI Pipeline** (`ci-pipeline` is SKIP). The CI-before-merge obligation is
  therefore `N/A` here, not `Unverified`, and this stage's local run is the only evidence.
- **No real browser engine runs anywhere.** Every frontend result is jsdom evidence.
- **`performance-validation` is skipped** by the plan, so no sustained-load measurement is
  scheduled. The units' performance requirements place their budgets in timeouts, deadlines and
  production histograms with alerts, and say no benchmark suite is introduced.
- **Pre-existing reds measured, not inherited as excuses:** `check-boundaries.sh` fails on
  `reporting-domain` (last touched by commit `6544576`, outside this intent); `format:check` is red
  in both frontend packages on 46 source files this intent never touched, plus generated `dist`
  output that no `.prettierignore` excludes. One `#[ignore]`d Identity repository case still fails —
  the pgsodium key-destruction guarantee; the other one (a stale U1 seam test against the U5
  erasure guard) was fixed on the replay. None is relaxed or hidden.
- **Coverage** was measured on the assembled project; the `identity` composition root reached the
  floor through U1's repair, and five touched crates remain below it.
- **The live-stack smoke script** (`scripts/smoke-identity.sh`) needs running services and real
  tokens, and still has no support-role fixture for the `mine` read.
- **Structurizr validation** needs network access to `static.structurizr.com`; this machine's DNS
  cannot resolve it, so the model edited by U10 is unvalidated.