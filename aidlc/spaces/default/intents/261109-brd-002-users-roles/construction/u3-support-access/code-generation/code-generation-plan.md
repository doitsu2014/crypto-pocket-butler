# Code Generation Plan — u3-support-access

**Unit:** U3 Support Access, embedded in the Identity service
**Upstream:** `functional-design/functional-spec.md` (W1–W5, error catalogue, audit events),
`functional-design/rules.md`, `functional-design/entities.md`,
`inception/contract-design/contract-summary.md` (C1, C2, C5), `nfr-design/*` (performance, security,
reliability, scalability, observability, logical-components),
`infrastructure-design/*` (infrastructure-specification, monitoring-design, cicd-pipeline),
`inception/units-generation/unit-of-work.md` (U3), `inception/requirements-analysis/requirements.md`
(FR16.1–FR16.6, BR3.1–BR3.13), `memory/project.md` (TDD, 80% coverage, canonical commands).

## Summary

- Builds: the four support-grant routes, the metadata projection and its two owner endpoints, the grant outbox and its drainer sink, the gateway route/limit wiring, and the support alert rules
- Touches: `services/identity/**`, `services/gateway/**`, `services/account/**`, `services/portfolio/**`, `infra/swarm-stack.yml`, `infra/prometheus/support-rules.yml`, `docs/architecture/`
- Tests: four new Rust test binaries for Identity (domain, application, http, adapters) plus one each for Account and Portfolio and two gateway boundary cases — roughly 60 tests, unit first, adapter tests behind `#[ignore]`

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "We write a failing test first for every piece of production code, then the implementation that makes it pass, and we land the test in the same change as the code it covers.",
  "scope": "classic",
  "test_strategy": "standard",
  "project_type": "brownfield",
  "applicable_notes": [
    {
      "layer": "org",
      "text": "Standard: requirement-driven tests with a happy-path floor per component."
    },
    {
      "layer": "team",
      "text": "- **Methodology**: tdd\n- **Methodology evidence**: The team affirmed test-first development for all production code at\n  the previous practices interview and re-confirmed it in this run's evidence review. The\n  organisation baseline still applies underneath it: requirement-driven tests with a happy-path\n  floor per component (`memory/org.md`), acceptance criteria in Given/When/Then form\n  (`memory/phases/inception.md`), business rules tested in `application` use cases against port\n  fakes, and one integration test per adapter against a real Postgres schema\n  (`docs/development/README.md`). TDD governs when each of those tests is written, not what they\n  cover. The repository cannot prove write-order from history — there are 71 `.rs` files under\n  `services/*/tests` and the crate `tests/` trees (suites plus shared fakes and helpers), 50\n  frontend `*.test.ts(x)` files, and ten test files whose database-backed cases are marked\n  `#[ignore]` — so ordering remains a team commitment rather than an observable.\n- **Ordering**: We write a failing test first for every piece of production code, then the\n  implementation that makes it pass, and we land the test in the same change as the code it\n  covers.\n- **Test types and layout**: domain rule tests per service; application use cases against\n  hand-written port fakes in `tests/fakes.rs` or `tests/common/fakes.rs`; HTTP boundary tests\n  over the Axum routers with status codes and RFC 7807 problem bodies; infrastructure adapter\n  tests against a real Postgres schema, which are `#[ignore]`d by default and run only when\n  `TEST_DATABASE_URL` is set. The ignored adapter and migration suites are now a blocking gate,\n  not a developer opt-in: every run provisions a disposable Postgres database and points\n  `TEST_DATABASE_URL` at it, so an adapter regression fails the change. Until the CI platform is\n  chosen, that gate is run by the same local command a developer runs today. The frontends use\n  Vitest 3 with Testing Library and jest-axe; MSW mocks the gateway in the support console.\n  There is no end-to-end browser journey suite and no cross-service contract test (unresolved).\n- **Commands and runners**: the canonical build, lint and test commands are:\n  - Rust — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,\n    `cargo test --workspace`, and `bash scripts/check-boundaries.sh` (the mechanical Clean\n    Architecture gate).\n  - Frontend — in both `web/` and `support-console/`: `yarn test` (`vitest run`), `yarn lint`\n    (ESLint 9 flat config), `yarn build` (`tsc -b && vite build`) and `yarn format:check`\n    (Prettier). `web` also defines `yarn size` (size-limit budgets: 300 kB initial entry chunk,\n    150 kB route chunk, 150 kB stylesheet, all gzip).\n\n  `aidlc/spaces/default/memory/project.md` holds this list as the single authoritative source;\n  `docs/development/README.md` and the per-package `package.json` scripts remain the\n  implementation detail behind it.\n- **Coverage**: 80% line coverage per Rust crate remains the affirmed target and is measured\n  with `cargo llvm-cov`, the coverage tool chosen at this interview. No TypeScript coverage\n  threshold has been set (unresolved). The happy-path-plus-two-error/edge-case floor per test\n  file still applies and a test that always passes regardless of implementation is not\n  acceptable (`memory/phases/construction.md`).\n- **Quality gates**: `bash scripts/check-boundaries.sh` is a real mechanical gate — it parses\n  `cargo metadata --no-deps` and fails on any outward crate edge. The Rust gates are\n  `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` and\n  `cargo test --workspace`; the frontend gates are `yarn test`, `yarn lint`, `yarn build` and\n  `yarn format:check`. There is still no CI platform in the repository — no `.github/`, GitLab,\n  Jenkins or CircleCI configuration — and the team chose at this interview to leave the platform\n  open for now and record CI as an intention, so today these gates run when a person runs them.\n  Dependency and container-image scanning (`cargo audit`, Trivy) is required by BRD-012\n  NFR-SEC-14 and named in unit design records but has no configuration or pipeline."
    }
  ],
  "obligations": {
    "strategy": "standard",
    "strategy_volume": [
      "Five to eight tests per component.",
      "Unit tests plus integration tests for key boundaries.",
      "Add E2E, performance, or security tests when requirements demand them."
    ],
    "scope_floor": [
      "Keep the existing test suite green.",
      "This scope adds no extra new-test floor beyond the selected test strategy."
    ],
    "combination_rule": "Apply every selected-strategy obligation and every scope-floor obligation; neither replaces the other, and a targeted scope regression may add the narrowest necessary test type beyond the strategy default."
  },
  "plan_profile": {
    "methodology": "tdd",
    "runner_step": "Verify the existing test runner/configuration and record the exact unit-scoped command.",
    "runner_ready_before_first_test": true,
    "testable_layers": [
      "Data model / database behavior",
      "Repository / data access",
      "Business logic",
      "API / endpoint",
      "Frontend behavior"
    ],
    "steps": [
      "Project structure and production configuration skeleton.",
      "Verify the existing test runner/configuration and record the exact unit-scoped command.",
      "Data model / database behavior - Red: write the failing tests and record the failing command output.",
      "Data model / database behavior - Green: implement only enough behavior to pass.",
      "Data model / database behavior - Refactor: improve the implementation while tests stay green.",
      "Repository / data access - Red: write the failing tests and record the failing command output.",
      "Repository / data access - Green: implement only enough behavior to pass.",
      "Repository / data access - Refactor: improve the implementation while tests stay green.",
      "Business logic - Red: write the failing tests and record the failing command output.",
      "Business logic - Green: implement only enough behavior to pass.",
      "Business logic - Refactor: improve the implementation while tests stay green.",
      "API / endpoint - Red: write the failing tests and record the failing command output.",
      "API / endpoint - Green: implement only enough behavior to pass.",
      "API / endpoint - Refactor: improve the implementation while tests stay green.",
      "Frontend behavior - Red: write the failing tests and record the failing command output.",
      "Frontend behavior - Green: implement only enough behavior to pass.",
      "Frontend behavior - Refactor: improve the implementation while tests stay green.",
      "Environment/build configuration.",
      "Documentation and traceability."
    ]
  },
  "input_sha256": "sha256:28b1b8f64ea2370ac2e8ea34ac6ae154b256412b60b0b2b72311ed876bfb0f5c",
  "contract_sha256": "sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3"
}
```

## What already ships (the delta this plan builds)

| Piece | State before this unit | This plan changes it |
|---|---|---|
| `domain/src/support.rs` | Grant aggregate, four statuses, reason 1–500, 1–60 min window, `effective_status`, `expire_if_due` | Adds the caller-binding and window predicates the read path needs; keeps the shape |
| `application/src/support.rs` | request / approve / revoke / check over a synchronous audit append | Rewrites the audit path onto the outbox, adds the metadata use case and the refusal-audit rule |
| `infrastructure/src/repositories/support.rs` | find by id, insert, update | Adds live-grant lookup, conditional approve/revoke, transactional outbox insert |
| `migrations/0001_init` | `support_access_grants` + `support_grants_approval_shape` | `0007_support_access` relaxes one clause, adds the outbox and two partial indexes |
| `http/src/routes.rs` | request / approve / revoke / internal check | Retires the internal check route, adds `/metadata`, adds the problem codes |
| Gateway route table | no support route | Four route entries and a per-route limit field |
| Account / Portfolio | no support-metadata endpoint | One internal endpoint each, one new operation each |
| Metrics / alerts | no `support_*` series | Ten series, one rule file |

## Traceability — plan step to story and requirement

| Plan steps | Story | Requirement | Design source |
|---|---|---|---|
| 2–4 | US3.1 | BR3.1, BR3.2, BR3.3, BR3.5, BR3.7 | functional-spec §"grant model", W1 |
| 5–7 | US3.1, US3.2 | BR3.3, BR3.4, BR3.12, BR3.13 | W1–W3, reliability-design "The outbox state machine" |
| 8–10 | US3.2, US3.3, US3.4 | BR3.4, BR3.6, BR3.8, BR3.13, FR16.4 | W2–W4, security-design §"Role checks" |
| 11–13 | US3.4, US3.5, US3.6, US7.3 | FR16.3, FR16.5, FR16.6, BR3.9–BR3.11, NFR-SEC-07 | W4, "The metadata boundary", contract C1/C2/C5 |
| 11–13 | — | NFR11.1, NFR23.3 | security-design §"Rate limiting", infrastructure-spec "Configuration this unit depends on" |
| 15 | — | NFR17.1–NFR17.13, NFR1.8 | observability-design, monitoring-design |
| 16 | — | NFR19.1–NFR19.4, NFR22.1–22.3 | functional-spec §"Audit events", security-design §"Audit" |

## Steps

### Step 1 — Project structure and production configuration skeleton

- [ ] Step 1: Confirm no new crate or dependency is needed. The work lands in the existing
      `identity`, `gateway`, `account` and `portfolio` packages and the four-crate layout each
      already has; no `Cargo.toml` gains a dependency, and `services/identity/migrations/` already
      exists. Record the confirmation in `code-summary.md`.
- [ ] Step 1b: Add the module skeleton only where a new file is unavoidable —
      `infrastructure/src/support/mod.rs`, `infrastructure/src/support/outbox.rs`,
      `infrastructure/src/support/backlog.rs` — with `//!` headers citing BR3.1–BR3.13 and the
      contract ids they serve, and no behavior.

### Step 2 — Verify the test runner and record the unit-scoped command

- [ ] Step 2: Verify `cargo test` already runs this workspace and record the exact unit-scoped
      commands in `unit-test-instructions.md`. No runner bootstrap: the repository is brownfield
      and the suite is green before this unit starts.

### Step 3 — Data model / database behavior (Red)

- [ ] Step 3: Write `services/identity/tests/support_adapters.rs` as an `#[ignore]`d,
      `TEST_DATABASE_URL` suite. Failing cases first:
      - `0007_support_access.up.sql` creates `support_audit_outbox` with `state`
        (`pending|delivered|dead`), `attempts`, `next_attempt_at`, `last_error`, `delivered_at`
        and the idempotency key, plus the partial indexes on `next_attempt_at` (pending) and
        `delivered_at` (delivered) — BR3.13, NFR1.4, NFR20.8.
      - The approval-shape constraint is relaxed exactly enough for an `expired` row with no
        approval fields, and nothing else — BR3.5.
      - A second live grant for the same (agent, target) is rejected by the partial unique index —
        BR3.3, NFR20.1.
      - Role grants: `identity_app` gets `select/insert/update/delete` on the outbox,
        `identity_readonly` `select`, `identity_migrator` DDL — NFR23.1.
      - `0007_support_access.down.sql` restores the old constraint as `NOT VALID` and raises while
        a `pending` or `dead` row remains — NFR18.2.
- [ ] Step 3b: Run the suite and record the failing output.

### Step 4 — Data model / database behavior (Green)

- [ ] Step 4: Write `services/identity/migrations/0007_support_access.up.sql` and
      `.down.sql` to pass exactly those cases, including the export-then-guard contract the down
      migration enforces.

### Step 5 — Data model / database behavior (Refactor)

- [ ] Step 5: Re-read the pair for the rules the tests do not see — statement ordering, `NOT VALID`
      usage, grant statements grouped and commented — with the suite staying green.

### Step 6 — Repository / data access (Red)

- [ ] Step 6: Extend `support_adapters.rs` with the adapter behavior:
      - `find_live_by_agent_and_user`, `find_by_id`, `insert`, and the conditional
        `approve`/`revoke` returning the affected-row count — BR3.3, BR3.4, BR3.12, NFR20.2–20.4.
      - The grant write and its outbox row in one transaction: a forced failure of the second
        leaves neither — BR3.13, NFR19.2.
      - Outbox claim under `FOR UPDATE SKIP LOCKED`, backoff, dead-lettering at 3 attempts, and
        the 24 h cleanup of delivered rows — NFR1.4, NFR1.6, NFR1.7, NFR20.5.
      - `SupportOutboxBacklog` scrape-time `COUNT(*)` and `MIN(created_at)` — NFR17.7, NFR17.8,
        NFR1.8.

### Step 7 — Repository / data access (Green)

- [ ] Step 7: Implement in `infrastructure/src/repositories/support.rs`,
      `infrastructure/src/support/outbox.rs` and `infrastructure/src/support/backlog.rs`, and
      export them from `infrastructure/src/lib.rs`.

### Step 8 — Repository / data access (Refactor)

- [ ] Step 8: Tighten the SQL (named binds, one statement per invariant, no read-then-write on the
      conditional paths) with the suite green.

### Step 9 — Business logic (Red)

- [ ] Step 9: Write `services/identity/tests/support_domain.rs` and
      `services/identity/tests/support_application.rs`. Domain: reason trimming and length,
      window bounds, `effective_status`, `expire_if_due` persisting the observation, caller-holder
      binding, target-not-self. Application against `tests/common/fakes.rs`:
      - request: role check, target lookup, `pending_deletion` refusal, 409 on a live grant,
        refusal audited, audit failure → 503 `audit-unavailable` — BR3.1–BR3.3, BR3.13.
      - approve: admin role checked before load, self-approval refusal, expired-request, zero-rows
        → 409 — BR3.4, BR3.5.
      - revoke: conditional, terminal grant refused — BR3.12.
      - metadata: uniform refusal for unknown grant and non-holder, pending/revoked/expired codes,
        the fan-out running concurrently under one 500 ms deadline, no partial projection, the
        `served` event appended after assembly and before the response — BR3.7, BR3.8, FR16.3.
      - Every case gets its happy path plus two error/edge cases.

### Step 10 — Business logic (Green)

- [ ] Step 10: Implement in `domain/src/support.rs` and `application/src/support.rs`, adding the
      `SupportMetadataSource`, `SupportObservation` and `SupportOutbox` ports to
      `application/src/ports.rs`. Refusals are audited before they return; a failed append refuses
      the read.

### Step 11 — Business logic (Refactor)

- [ ] Step 11: Collapse duplicated refusal paths into one audited helper, keep the port fakes
      honest, tests green.

### Step 12 — API / endpoint (Red)

- [ ] Step 12: Write the boundary tests:
      - `services/identity/tests/support_http.rs`: the four routes, status codes and RFC 7807
        codes for every refusal in the error catalogue; the retired internal check route is gone;
        the metadata shape carries exactly the allowlisted fields and nothing else — FR16.5,
        BR3.9–BR3.11.
      - `services/gateway/tests/domain.rs` and `http.rs`: the four route entries resolve;
        `max_requests_per_minute` limits a named route; the token-hash bucket is keyed on the
        bearer token hash, not the `sub` claim, and does not evict the IP windows; 429 carries
        `Retry-After` — NFR11.1.
      - `services/account/tests/support_metadata.rs` and
        `services/portfolio/tests/support_metadata.rs`: the internal endpoint accepts the identity
        service token with the new operation and refuses a token without it, a missing target and
        a malformed id — contract C1/C2.

### Step 13 — API / endpoint (Green)

- [ ] Step 13: Implement, in this order:
      - Identity: `http/src/routes.rs`, `handlers.rs`, `dto.rs` (`SupportMetadata`),
        `http/src/metrics.rs` (the ten `support_*` series, zero-initialised, backlog read at
        scrape time), and the sink the drainer calls in `src/lib.rs`.
      - Gateway: `domain/src/route.rs` (the field), `http/src/rate_limit.rs` (per-route bucket,
        separate bounded map), `http/src/middleware.rs`, `routes.rs`, `server.rs`,
        `infrastructure/src/upstream.rs`, and the four entries in the default route table.
      - Account: the new `Operation` variant, the internal route, the query, and
        `infra/sql/account-service-tokens.sql` (today Account has no seed file at all).
      - Portfolio: the same operation and route, and the operation added to
        `infra/sql/portfolio-service-tokens.sql`.
      - `infra/swarm-stack.yml`: correct the `ACCOUNT_URL` default to `http://account-api:8091`.

### Step 14 — API / endpoint (Refactor)

- [ ] Step 14: One DTO mapping function per direction at the `http` boundary, no DTO type inward,
      `ProblemDetails` construction shared, boundaries gate green.

### Step 15 — Frontend behavior (not applicable)

- [ ] Step 15: Record in `code-summary.md` that this unit changes no frontend file: the support
      console is U9 and the admin surface U10, and neither is in this unit's scope. No `web/` or
      `support-console/` file changes, so no frontend Red step exists.

### Step 16 — Environment/build configuration

- [ ] Step 16: Add `infra/prometheus/support-rules.yml` with the ten rules of
      `monitoring-design.md`, every rule carrying `severity`, `service`, `summary` **and**
      `runbook`. Wire the `Metrics` sink into `src/bin/api.rs` and confirm `/metrics` renders the
      new series.
- [ ] Step 16b: Update `docs/architecture/workspace.dsl` and `docs/architecture/README.md` for the
      new table, the two internal endpoints and the four gateway routes, as the project rule
      requires.

### Step 17 — Documentation and traceability

- [ ] Step 17: Fill `//!` module headers with the BR ids they serve; write `code-summary.md`;
      write `traceability.json` covering this unit's FR/NFR ids with an honest status each.
- [ ] Step 17b: Run all four sensors (`required-sections`, `upstream-coverage`, `linter`,
      `type-check`) and fix what they report.

### Step 18 — Quality gates

- [ ] Step 18: Run, in order: `cargo fmt --all -- --check`,
      `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-boundaries.sh`,
      `cargo test --workspace`, the `#[ignore]` suites with a disposable `TEST_DATABASE_URL`, and
      `cargo llvm-cov --workspace` — every crate at or above 80% line coverage.

## Test files this plan creates

| File | Kind | Count | Covers |
|---|---|---|---|
| `services/identity/tests/support_domain.rs` | unit | ~10 | Grant rules, window, expiry, holder binding |
| `services/identity/tests/support_application.rs` | unit (fakes) | ~14 | W1–W4 use cases, audit-failure paths |
| `services/identity/tests/support_http.rs` | boundary | ~12 | Routes, problem codes, metadata shape |
| `services/identity/tests/support_adapters.rs` | adapter (`#[ignore]`) | ~10 | Migration, repository, outbox, backlog |
| `services/account/tests/support_metadata.rs` | boundary + adapter | ~5 | Internal endpoint, operation allow-list |
| `services/portfolio/tests/support_metadata.rs` | boundary + adapter | ~5 | Same, portfolio side |
| `services/gateway/tests/domain.rs` (additions) | unit | ~2 | Route entries, per-route limit field |
| `services/gateway/tests/http.rs` (additions) | boundary | ~3 | 429, `Retry-After`, token-hash key |

## Open items carried into the build

- The Account service has no `infra/sql/*-service-tokens.sql` at all; this plan writes one, but the
  production release job that runs it is still an open platform item.
- `X-Support-Reason` is forwarded by the gateway today and is not evidence of anything; the read
  path uses the grant's own reason. No change to the header's forwarding is made here.

## Loop-back 1 repair (Build and Test, 2026-11-12)

Build and Test reopened this Unit for **F6 only** — the 80 % per-crate line floor. The behaviour
this Unit builds is already in place and reviewed; the repair adds the tests that exercise the
code paths the floor counts and none of them reach today. Steps 1–18 stand as they are. This is a
test-first change: every test is written and seen to fail first where it can fail, and a test is
never added just to touch a line — a path that cannot be asserted meaningfully is reported as a
gap instead of covered by a hollow assertion.

The five crates below the floor after the last Build and Test run, and what this Unit owns of them:

| Crate | Coverage at the reopen | Lines to reach 80 % | Owned here |
|---|---|---|---|
| `account-http` | 780 / 983 — 79.35 % | +7 | yes |
| `portfolio-http` | 629 / 1 386 — 45.38 % | +480 | yes |
| `gateway-infrastructure` | 145 / 211 — 68.72 % | +24 | yes |
| `gateway` (composition root) | 3 / 93 — 3.23 % | +72 | yes |

`identity-infrastructure` (64.95 %) belongs to `u1-user-registry` and is not in this repair.

19. [x] **Baseline.** Re-measure the four crates with the project's command and record the exact
    output before any test is added:
    `cargo llvm-cov -p account-http -p portfolio-http -p gateway-infrastructure -p gateway --ignore-run-fail --json`.
    Record the per-crate numbers in `code-summary.md` as the before-picture.
20. [x] **`portfolio-http` — Red.** Add router-level tests in `services/portfolio/tests/` that
    drive every route the crate serves — the portfolio CRUD and valuation-inclusion handlers, the
    problem-path branches (unknown portfolio, wrong owner, bad payload), and the support-metadata
    route — asserting status codes and RFC 7807 bodies, not just that a handler returns. Record
    the failing run first (a `404` from a route that is not wired yet, or a compile error on a
    test helper). *Trace:* FR17.x support seam, F6.
21. [x] **`portfolio-http` — Green/Refactor.** Nothing should need implementing; where a test
    exposes a real defect, fix it test-first in the same change and say so in the summary. Re-run
    the crate's suite and the coverage command until the crate clears 80 % or the residual gap is
    reported honestly.
22. [x] **`account-http` — Red/Green.** Extend `services/account/tests/` to reach the remaining
    uncovered handler branches (the reopen leaves seven lines short). Same rule: assert behaviour,
    never the line.
23. [x] **`gateway-infrastructure` — Red/Green.** Add tests for the `upstream` client the gateway
    uses to forward to each service: success, the service-unavailable branch, and the timeout or
    error-mapping branch. *Trace:* NFR2.x budgets, F6.
24. [x] **`gateway` composition root — Red/Green.** Add tests for `services/gateway/src/config.rs`
    and `services/gateway/src/lib.rs` — configuration parsing with and without each required
    variable, and the wiring path that assembles the router. *Trace:* infrastructure-specification,
    F6.
25. [x] **Record.** Re-run step 19's command, put the after-picture beside the before-picture in
    `code-summary.md` with any crate still under the floor and why, update `source-manifest.json`
    and `traceability.json` with the new test files, and re-run
    `aidlc engine sensor-traceability --stage build-and-test`.