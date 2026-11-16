# Code Generation Plan — U1 Identity Auth Domain

## Summary

- Builds: Extend Identity domain/application policy for session lifecycle and auth policy; close only U1-owned behaviors
- Touches: `services/identity/{domain,application}/`, Identity tests, and Identity-owned migrations only if the implementation requires persistent U1 state
- Tests: TDD-first domain/use-case tests, HTTP-adjacent application tests where relevant, and Postgres integration/migration tests

## Scope and constraints

U1 is the existing Identity service's domain/application work, not a new service. Preserve Clean Architecture, inward dependencies, one Identity schema/role, Supabase Auth as credential/factor/token authority, unchanged user JWT validation, existing provider and audit ports, and the project's no-cross-schema/no-queue rules. U2 owns provider/HTTP/gateway adapters; U3 owns SPA behavior; U4 owns integrated route/browser proof. Do not implement their surfaces in this Unit.

Build the behaviors currently represented by existing Identity domain/application structure: session ownership and revocation consistency, absolute session cap anchored to each session's first sign-in, trusted-side user/MFA policy where a defined Identity port/data source exists, idempotent session operations, and safe audit intent semantics. Do not silently create provider MFA implementations, a separate auth service, an unapproved provider integration, or new routes. Where existing ports or authoritative data are absent (MFA factor policy/source, exchange-credential presence, support recovery approval semantics, persistent signup-gate single-use authorization), identify the gap in code summary and traceability rather than inventing a cross-service dependency.

A prior advisory review of Functional Design identified concrete design concerns: the 30-day cap was specified on `InternalUser.first_signed_in_at` rather than a session-level anchor; pending MFA enrolment had no representable state; exchange-credential presence lacked a supplying port; and device-history authority was ambiguous. Correct these within U1 only where the current Identity contracts and owned data support it; otherwise mark the affected criteria as deferred to U2/U4 or explicitly unresolved in the implementation notes. Preserve the later human answers: first sign-in starts a session's 30-day absolute cap; support recovery uses a distinct existing support-grant approver; first-seen device only triggers alerts.

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
      "text": "- **Methodology**: tdd\n- **Methodology evidence**: The team affirmed test-first development for all production code at\n  the first practices interview and re-confirmed it at the second; this run re-confirms it from\n  fresh evidence and the person re-affirmed it at the current interview. The organisation\n  baseline still applies underneath it: requirement-driven tests with a happy-path floor per\n  component (`memory/org.md`), acceptance criteria in Given/When/Then form\n  (`memory/phases/inception.md`), business rules tested in `application` use cases against port\n  fakes, and one integration test per adapter against a real Postgres schema\n  (`docs/development/README.md`). TDD governs when each of those tests is written, not what they\n  cover. The repository cannot prove write-order from history — there are 92 `.rs` files under\n  `services/*/tests` and the crates' own `tests/` trees (79 suite files plus 13 shared `common/`\n  helpers carrying fakes, fixtures and a fixed clock), 80 frontend `*.test.ts(x)` files (42 in\n  `web/`, 38 in `support-console/`), 8 Playwright browser journeys, and 176 `#[ignore]`\n  attributes spread over 19 test files — so ordering remains a team commitment rather than an\n  observable.\n- **Ordering**: We write a failing test first for every piece of production code, then the\n  implementation that makes it pass, and we land the test in the same change as the code it\n  covers.\n- **Test types and layout**: domain rule tests per service; application use cases against\n  hand-written port fakes in `tests/fakes.rs` or `tests/common/fakes.rs`; HTTP boundary tests\n  over the Axum routers with status codes and RFC 7807 problem bodies; infrastructure adapter and\n  migration tests against a real Postgres schema, which are `#[ignore]`d by default and run only\n  when `TEST_DATABASE_URL` is set. One file per layer, named for the layer\n  (`domain.rs`, `application.rs`, `http.rs`, `repositories.rs`, `*_adapters.rs`, `composition.rs`),\n  as `aidlc/spaces/default/knowledge/aidlc-developer-agent/test-layout.md` records. The ignored\n  adapter and migration suites are a blocking gate, not a developer opt-in: every run provisions\n  a disposable `supabase/postgres:15.8.1.060` database (superuser, because the suites create and\n  drop roles) and points `TEST_DATABASE_URL` at it. The frontends use Vitest 3 with Testing\n  Library and `jest-axe` (36 files assert accessibility), and MSW 2 mocks the gateway in both\n  packages. `web/` also carries a Playwright browser-journey suite (`web/e2e/journeys/`, 3 spec\n  files, 8 journeys over sign-in, dashboard and settings; commit `784dae8`). It drives a real\n  Chromium against the Vite dev server with Supabase Auth and the gateway answered at the network\n  layer, so a run needs no Docker, no database and no credentials — and the gateway mock fails a\n  journey on any unmocked `/v1` call. It found a first-render crash that all 312 unit tests had\n  passed over, and it is the reason the browser journeys now gate the auth surface. There is\n  still no cross-service contract test (unresolved).\n- **Commands and runners**: the canonical build, lint and test commands are:\n  - Rust — `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`,\n    `cargo test --workspace`, and `bash scripts/check-boundaries.sh` (the mechanical Clean\n    Architecture gate).\n  - Frontend — in both `web/` and `support-console/`: `yarn test` (`vitest run`), `yarn lint`\n    (ESLint 9 flat config), `yarn build` (`tsc -b && vite build`) and `yarn format:check`\n    (Prettier). `web` also defines `yarn e2e` (`playwright test`) with `yarn e2e:report`, and\n    `yarn size` (size-limit plus `scripts/check-size.mjs`: 300 kB gzip initial entry chunk,\n    150 kB gzip per route chunk, 150 kB gzip stylesheet — NFR2.3).\n\n  `aidlc/spaces/default/memory/project.md` holds this list as the single authoritative source,\n  and the list now includes `yarn e2e` and `yarn size` for `web/`, so a gate run and a local run\n  are the same commands. `docs/development/README.md` and the per-package `package.json` scripts\n  remain the implementation detail behind it.\n- **Coverage**: 80% line coverage per Rust crate remains the affirmed target, and the measurement\n  contract is part of the practice, not a footnote: Rust coverage is measured with `cargo\n  llvm-cov` using a reading that instruments the package carrying the tests — the workspace run\n  (`cargo llvm-cov --workspace --include-ignored --ignore-run-fail …`) or the paired run\n  (`cargo llvm-cov -p identity-infrastructure -p identity --include-ignored …`) — always after\n  the disposable database is up so the `#[ignore]` suites contribute, and a coverage figure is\n  always quoted together with the command that produced it. The contract is written down in\n  `aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md` because a plain\n  `cargo llvm-cov -p <crate>` instruments only that package, while most suites live in the\n  *service* package, so a per-crate run reports a number far below reality — a wrong number has\n  already cost one repair round. No TypeScript coverage threshold has been set (unresolved), and\n  the runbook says not to introduce one as a side effect of another change. The\n  happy-path-plus-two-error/edge-case floor per test file still applies and a test that always\n  passes regardless of implementation is not acceptable (`memory/phases/construction.md`).\n- **Quality gates**: a green `yarn e2e` gates any change touching the auth/sign-in surface —\n  `web/src/auth/`, `web/src/features/auth/` and the sign-in, MFA and session journeys under\n  `web/e2e/`. It is a gate there rather than everywhere because it is the only test that has\n  caught a real first-render defect, and it is cheap enough (no Docker, no database, no\n  credentials) to run on every such change. `bash scripts/check-boundaries.sh` is a real\n  mechanical gate — it parses `cargo metadata --no-deps` and fails on any outward crate edge. We\n  keep the CI platform open rather than choosing one, and we record the full ordered gate list so\n  any platform can implement it without re-deciding anything:\n\n  1. `cargo fmt --all -- --check`\n  2. `cargo clippy --workspace --all-targets -- -D warnings`\n  3. `bash scripts/check-boundaries.sh`\n  4. `cargo test --workspace` (with the disposable database up, so the `#[ignore]` adapter and\n     migration suites run)\n  5. `cargo llvm-cov` under the measurement contract above, against the 80% per-crate floor\n  6. `yarn test`, `yarn lint`, `yarn build`, `yarn format:check` in `web/` and `support-console/`\n  7. `yarn e2e` in `web/` (required for any change touching the auth/sign-in surface)\n  8. `yarn size` in `web/`\n\n  There is still no CI platform in the repository — no `.github/`, GitLab, Jenkins, CircleCI or\n  Buildkite configuration, and no Dependabot or Renovate config — so today these gates run when a\n  person runs them, in this order. A pipeline did exist historically\n  (`.github/workflows/build-api.yml`, `build-web.yml`, `build-helm.yml`, `deploy-k8s.yml`, added\n  in `9f9b230` on 2026-02-22 for a Helm/Kubernetes deployment) and was removed in `a567470` on\n  2026-11-01 when the repository was re-based on business requirements; it is not a current\n  practice. Dependency and container-image scanning (`cargo audit`, Trivy) is required by BRD-012\n  NFR-SEC-14 and named in unit design records but has no configuration or pipeline.\n  `scripts/smoke-identity.sh` covers the Identity health and C4 authorisation paths as a\n  post-deploy smoke check, but nothing calls it automatically. *(Sources: `memory/org.md`,\n  `memory/phases/inception.md`, `memory/phases/construction.md`, `docs/development/README.md`,\n  `aidlc/spaces/default/knowledge/aidlc-quality-agent/coverage-runbook.md`, `web/package.json`,\n  `support-console/package.json`, `web/e2e/README.md`, `scripts/check-boundaries.sh`, `git log`;\n  interview Q3, Q4, Q5.)*"
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
  "input_sha256": "sha256:a0d3270c6568820ec58c7730d68d9863a6c0ab59acee1b4c251da3aeffd0dd91",
  "contract_sha256": "sha256:16825f49e9f0f0e0225263a8b0d9fc73a44694c3da182cfbd5663d211eeccd17"
}
```

## Ordered implementation steps

1. [x] Test-runner readiness: verified `cargo test -p identity --lib` and `cargo test -p identity-domain` (service unit suite passed; the domain crate currently has no unit tests). No global coverage thresholds changed.
2. [x] Domain Red/Green/Refactor — session lifetime: added `DeviceSession.first_seen_at` as the per-provider-session anchor; refresh activity does not reset it. Added exact-cap and before-expiry tests. Existing rows are backfilled from `last_activity_at`, conservatively starting their 30-day window at migration time's last-known session activity.
3. [x] Domain Red/Green/Refactor — policy state: checked current claims and domain contracts. Existing validated `VerifiedClaims` already carries `has_exchange_credentials` and `assurance`; `User::next_step` already expresses the existing limited-session decision. MFA enrolment pending/factor lifecycle remains provider-owned and lacks an Identity port, so no unsupported policy model was added.
4. [x] Application Red/Green/Refactor — session use cases: verified current `SessionService` already enforces ownership, provider-first revoke, local-state preservation on provider error, and duplicate-revoke idempotency. The existing public repository port has no bulk-revoke operation, so no unsupported bulk behavior was added.
5. [ ] Application Red/Green/Refactor — signup/recovery/MFA policy: extend existing use cases only where their required typed ports and owned state already exist. Do not implement provider adapters, SPA route handlers or gateway declarations in U1. Any unresolved approval/source boundary becomes an explicit deferred traceability entry and code-summary note.
6. [ ] Persistence tests/migrations (conditional): if the selected U1 domain/application behavior requires a new Identity-owned column/table, first write a failing migration/adapter test; add a versioned sqlx up/down pair and verify both directions against disposable Postgres. Do not add a migration for behavior that can be represented by existing data.
7. [ ] Security and audit tests: test sanitization, ownership, provider outage, idempotency and audit behavior. Keep credentials, tokens, factor seeds, recovery codes, raw IP and full fingerprints out of persistent Identity data and logs.
8. [ ] Final unit tests and diff review: run the scoped command, relevant integration tests, formatting and clippy; record commands/results and any unexecuted DB suite with reason in `code-summary.md`. Do not report the project-wide 80% floor based on the unit-only command.

## Traceability plan

Map each implemented acceptance criterion and detailed NFR/BR to a concrete changed source or test file. Mark criteria outside U1 ownership as `Deferred` with the owning Unit and reason; do not claim gateway/UI/provider adapter coverage from Identity domain/application code alone. The integration completion invariant—every SPA auth path has a declared gateway route and exactly one owning handler—belongs to U2/U4, not this Unit.
