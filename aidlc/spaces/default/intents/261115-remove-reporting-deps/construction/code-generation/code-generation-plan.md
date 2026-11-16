# Code Generation Plan — Remove unused reporting-domain serialization dependencies

## Summary

- Builds: Remove the two unused serialization dependencies from `reporting-domain`.
- Touches: `services/reporting/domain/Cargo.toml`; `Cargo.lock` only if Cargo proves it must change.
- Tests: Run `cargo check -p reporting-domain` and `bash scripts/check-boundaries.sh`.

## Approved scope

Remove `serde` and `serde_json` only from `services/reporting/domain/Cargo.toml`. Preserve these dependencies in reporting application, infrastructure, HTTP, and service crates, where source uses them. No source behavior, schema, API, frontend, CI, or unrelated crate changes.

## Testing Contract

```json
{
  "version": 1,
  "methodology": "tdd",
  "source": "team",
  "ordering": "We write a failing test first for every piece of production code, then the implementation that makes it pass, and we land the test in the same change as the code it covers.",
  "scope": "bugfix",
  "test_strategy": "minimal",
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
    "strategy": "minimal",
    "strategy_volume": [
      "One verifiable test per requirement at the narrowest effective level.",
      "At least one happy-path unit test per component.",
      "Unit tests are the default; a bugfix/security scope floor may require an integration or E2E regression when that is the narrowest level that reproduces the defect."
    ],
    "scope_floor": [
      "Include a targeted regression for the bug or vulnerability.",
      "Keep the existing test suite green."
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
  "input_sha256": "sha256:d291e7816c6e69c3fcb73644eef38e159519516ea41cfdc7fa5574e638a29523",
  "contract_sha256": "sha256:fd8e3e3b887931972763398966c904489d6edcf728ab73bc343fd6b6f2a63c7c"
}
```

## Ordered implementation steps

1. [x] Runner readiness: `cargo check -p reporting-domain` verified and passed.
2. [x] Regression Red: `bash scripts/check-boundaries.sh` should reproduce the reporting-domain dependency failure.
3. [x] Green: remove only `serde` and `serde_json` from `services/reporting/domain/Cargo.toml`; rerun compile and boundary checks.
4. [x] Refactor/verification: inspect the scoped manifest/lock diff; ensure other reporting crates remain unchanged.
5. [x] Record traceability and actual verification outcomes.