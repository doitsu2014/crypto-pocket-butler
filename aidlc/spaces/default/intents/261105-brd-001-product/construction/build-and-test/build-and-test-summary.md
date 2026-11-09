# Build and Test Summary

## Overall Status

**Build: success. Test: passing except one identity assertion.** The workspace builds, formats and
lints clean; 1349 Rust tests, 142 web tests and 180 support-console tests pass; every adapter suite
passes except one identity crypto-shredding assertion that this pgsodium build cannot satisfy (the
deleted key stays in the cluster's in-memory cache until a restart — recorded as a known limitation
by the person's decision). The cross-unit traceability gate still reports 27 uncovered IDs and 80
prose/multi-file targets.

## Test Type Inventory

- **Unit and application tests (Rust):** `cargo test --workspace` — the whole workspace's
  unit and application behaviour tests.
- **Adapter integration tests (Rust + PostgreSQL):** 12 suites across asset-registry, identity,
  audit, account, market-data, portfolio, ledger, notification and analytics.
- **Frontend integration tests (Vitest + jsdom):** `web` (142 tests) and `support-console`
  (180 tests). These run in jsdom, not a real browser engine.
- **Accessibility (automated, jsdom):** axe over every core SPA flow in
  `web/src/app/accessibility.test.tsx`.
- **Not generated at Standard strategy:** performance, security (SAST/DAST) and real-browser
  (Playwright/Lighthouse) instruction sets; no CI Pipeline or performance-validation stage is in
  the plan.

## Coverage Expectations

- Team memory: 80% line coverage per Rust crate, and the TypeScript threshold is unresolved.
- No coverage tool is chosen in the repository and the plan runs no CI Pipeline, so no coverage
  percentage is produced. No threshold was lowered.

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| BLD-01…03 | build instructions | All three builds succeed | exit 0 | test-results.md | build-and-test | Met |
| FMT-01, LINT-01, LINT-02 | team memory | Format and lint clean | exit 0 | test-results.md | build-and-test | Met |
| UT-01 (Rust), UT-02 (web) | unit test instructions | Pass | exit 0 / 142 passed | test-results.md | build-and-test | Met |
| UT-03 (support console) | unit test instructions | Pass | 180 passed | test-results.md | build-and-test | Met |
| SIZE-01 | NFR2.3 | Initial ≤300 kB gzip; route ≤150 kB | 94.1 kB; routes under limit | `yarn --cwd web size` | build-and-test | Met |
| INT-01 | adapter suites | All adapter suites pass | every suite green except one identity crypto-shredding assertion | test-results.md | build-and-test | Unverified — the deleted key stays in pgsodium's in-memory cache until a cluster restart |
| TRACE-01 | Step 10 sensor | Every upstream ID covered by one existing target | 27 uncovered, 80 invalid target rows | cross-unit-traceability.md | build-and-test (gate finding) | Fail — surfaced at the gate |
| COV-01 | team memory | 80% line per Rust crate | not measured | no coverage tool chosen (unresolved) | none — no CI Pipeline | N/A — the plan runs no CI Pipeline |
| CI-01 | Testing Contract | Tests run in CI before merge | not run | CI Pipeline (3.7) is skipped | ci-pipeline (skipped) | N/A — the plan runs no CI Pipeline |
| NFR-PERF, NFR-AVL, NFR-FRESH, NFR-REC, NFR-SCAN | NFR1, NFR2, NFR3, NFR5, NFR6, NFR12, NFR18 | Latency, availability, freshness, recovery and scanning targets | not measured | no production-like environment; the owning stages are outside the plan | none scheduled | Unverified |
| NFR-A11Y | NFR15 | WCAG 2.2 AA on core flows | axe passes in jsdom; no real browser engine | `web/src/app/accessibility.test.tsx` | build-and-test | Unverified — no real browser ran it |

## Readiness Assessment

- **Build-ready:** yes.
- **Test-ready:** yes except one identity crypto-shredding assertion, which this pgsodium build
  cannot satisfy until the cluster restarts (deleted key still cached). All other suites are green.
- **Deployment-ready:** no — the cross-unit traceability gate is red (27 uncovered IDs, 80 invalid
  target rows) and the environment-dependent NFR targets have no measurement path in this plan.

## Known Limitations and Outstanding Items

1. The six loop-back repairs are in and verified (see `test-results.md`).
2. Identity crypto-shredding: `destroy_shredding_key` deletes the key row, but this pgsodium build
   keeps the key in a cluster-wide cache until restart; the test asserting that decryption fails
   after destroy is the one red test, recorded per the person's decision.
3. Cross-unit traceability: 27 uncovered IDs and 80 rows whose `target` is not a single file.
4. No browser matrix or Lighthouse run (no Playwright/Lighthouse in the repository and no CI).
5. No coverage measurement (tool unresolved).
6. Environment-dependent NFR targets (availability, latency, freshness, recovery, scanning) are
   unverified because the plan has no production-like environment and skips performance
   validation.
7. The same `make_interval(days => $1)` defect remains in the reporting and job-orchestration prune
   functions; neither unit ships an adapter test that runs the prune.
