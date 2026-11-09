# Test Results — Build and Test

Run date: 2026-11-09. Environment: local (workspace root as the working tree), disposable
PostgreSQL 15 provided through Docker on `localhost:55432`, `TEST_DATABASE_URL` set.

## Build status

| Command | Exit | Result |
|---|---|---|
| `cargo build --workspace` | 0 | Workspace builds |
| `cargo fmt --all -- --check` | 0 | No diff |
| `cargo clippy --workspace --all-targets -- -D warnings` | 0 | Clean (after the two lint repairs recorded below) |
| `yarn --cwd web lint` | 0 | Clean |
| `yarn --cwd web build` | 0 | Vite build + PWA `injectManifest`, `dist/sw.js` generated |
| `yarn --cwd web size` | 0 | Initial chunk set 94.1 kB gzip (limit 300), 19 route chunks ≤150 kB each, stylesheet 4.16 kB |
| `yarn --cwd support-console build` | 0 | Support bundle emitted under `web/dist/support/` |

## Test results

| Suite | Passed | Failed | Command |
|---|---|---|---|
| Rust workspace (unit + application) | all | 0 | `cargo test --workspace` |
| asset-registry adapter | 7 | 0 | `cargo test --manifest-path services/asset-registry/Cargo.toml --test repositories -- --ignored` |
| identity adapter | 1 | 8 | `cargo test --manifest-path services/identity/Cargo.toml --test repositories -- --ignored` |
| audit adapter | 14 | 0 | `cargo test --manifest-path services/audit/Cargo.toml --test repositories -- --ignored` |
| account adapter | 11 | 2 | `cargo test -p account-infrastructure --test repositories -- --include-ignored` |
| market-data adapter | 3 | 1 | `cargo test -p market-data-infrastructure --features integration -- --ignored` |
| portfolio adapter | 5 | 2 | `cargo test -p portfolio-infrastructure --features integration -- --ignored` |
| ledger adapter | 12 | 0 | `cargo test -p ledger-infrastructure --features integration -- --ignored` |
| notification adapter | 14 | 0 | `cargo test -p notification-infrastructure --test repositories -- --include-ignored` |
| analytics adapter | 4 | 4 | `cargo test -p analytics-infrastructure --test repositories -- --include-ignored` |
| reporting / job-orchestration integration | 0 run | 0 | No adapter test module is defined; the `integration` feature builds |
| web (Vitest) | 142 | 0 | `yarn --cwd web test` |
| support console (Vitest) | 179 | 1 | `yarn --cwd support-console test` |

## Failure details

1. **identity — 8 failures.** `services/identity/infrastructure/src/repositories/keys.rs:67,128`
   and `users.rs:106,193` call `identity.create_shredding_key`, `identity.destroy_shredding_key`,
   `identity.protect` and `identity.unprotect` with the schema hardcoded. The adapter harness
   creates a disposable schema and sets `search_path` to it, so `identity.*` does not exist. The
   unit's own migration says object names are unqualified on purpose for exactly this reason.
2. **account — 2 failures.** `renewal_and_the_sweep_both_track_the_lease` renews a 60-second lease
   to the same instant and then expects it to survive an hour; `the_snapshot_write_is_fenced_on_a_live_lease`
   reads holdings through `latest_for_account`, which only returns snapshots from a `succeeded`
   run, while the run under test is still `running`. Both are test-expectation defects.
3. **market-data — 1 failure.** `the_schema_declares_no_large_object_or_bytea_storage` counts
   `_sqlx_migrations.checksum` (sqlx's own bytea bookkeeping column); the schema itself stores no
   binary payloads.
4. **portfolio — 2 failures.** `the_prune_removes_only_old_snapshots` runs
   `make_interval(days => $1)` with a `bigint` argument (PostgreSQL needs `$1::int`).
   `the_run_lease_is_single_flight` shares one schema with every other portfolio test and is
   order-dependent under parallel execution.
5. **analytics — 4 failures.** The fixtures insert `user_ref` values such as `"user-ref"`, while
   the schema requires 32–128 characters.
6. **support-console — 1 failure.** `src/domain/grants.test.ts > leaves pending and revoked grants
   untouched` builds the expired fixture from `Date.now()` while the test's clock is fixed at
   2026-11-08T10:00:00Z, so the fixture is still active.

## Coverage

Not measured. Team memory sets an 80% line floor per Rust crate, but no coverage tool is chosen in
the repository (team memory records the tool as unresolved), the plan runs no CI Pipeline, and no
coverage command exists to run. The TypeScript coverage threshold is likewise unresolved. No
threshold was lowered or disabled.

## Target Verification Matrix

| Target ID | Source | Expected | Actual | Evidence | Owning Stage | Verdict |
|---|---|---|---|---|---|---|
| BLD-01 | build instructions | Workspace builds | exit 0 | `cargo build --workspace` | build-and-test | Met |
| BLD-02 | build instructions | SPA builds | exit 0 | `yarn --cwd web build` | build-and-test | Met |
| BLD-03 | build instructions | Support console builds | exit 0 | `yarn --cwd support-console build` | build-and-test | Met |
| FMT-01 | team memory | `cargo fmt --check` clean | exit 0 | command output | build-and-test | Met |
| LINT-01 | team memory | clippy `-D warnings` clean | exit 0 | command output | build-and-test | Met |
| LINT-02 | team memory | web ESLint clean | exit 0 | `yarn --cwd web lint` | build-and-test | Met |
| UT-01 | unit test instructions | Rust workspace tests pass | exit 0 | `cargo test --workspace` | build-and-test | Met |
| UT-02 | unit test instructions | Web tests pass | 142 passed | `yarn --cwd web test` | build-and-test | Met |
| UT-03 | unit test instructions | Support-console tests pass | 179 passed, 1 failed | `yarn --cwd support-console test` | code-generation (u13) | Not Met |
| SIZE-01 | NFR2.3 | Initial ≤300 kB gzip; route ≤150 kB | 94.1 kB; max route under 150 kB | `yarn --cwd web size` | build-and-test | Met |
| INT-01 | adapter suites | Every adapter suite passes | 6 of 12 suites failing | test-results above | code-generation (u3, u4, u5, u6, u10) | Not Met |
| TRACE-01 | Step 10 sensor | Every upstream ID covered by an existing target | 27 uncovered | `cross-unit-traceability.md` | build-and-test | Not Met |
| COV-01 | team memory | 80% line per Rust crate | not measured | no coverage tool chosen (unresolved) | none (no CI Pipeline) | N/A — the plan runs no CI Pipeline and the repository has no coverage command |
| CI-01 | Testing Contract | Tests run in CI before merge | not run | the plan skips CI Pipeline (3.7) | ci-pipeline (skipped) | N/A — the plan runs no CI Pipeline |
| NFR-PERF | NFR2, NFR3, NFR6 | p95 latency and onboarding timing targets | not measured | no production-like environment; performance-validation (4.6) is outside the plan | none scheduled | Unverified |
| NFR-AVL | NFR1 | 99.9% monthly availability | not measured over 30 days | needs production telemetry | operations (not in plan) | Unverified |
| NFR-FRESH | NFR5 | ≤5 min top-100, ≤15 min tail | not measured | needs the running job pipeline | operations (not in plan) | Unverified |
| NFR-REC | NFR18 | RPO ≤24 h, RTO ≤4 h | not measured | needs a restore rehearsal | operations (not in plan) | Unverified |
| NFR-SCAN | NFR12 | Dependency and image scan before release | not run | no scan tool configured; release pipeline outside the plan | release (not in plan) | Unverified |
| NFR-A11Y | NFR15 | WCAG 2.2 AA on core flows | axe suite passes in jsdom; no real browser | `web/src/app/accessibility.test.tsx` | build-and-test | Unverified — no real browser engine ran it |

## Loop-Back Log

### Loop-back 1 — 2026-11-09T04:16:00Z

- **Diagnosis.** Twelve adapter-suite failures and one support-console failure. Every root cause is
  in generated test or source code: schema-qualified function calls in identity, wrong test
  arithmetic and read semantics in account, sqlx bookkeeping counted by a schema-shape assertion in
  market-data, a missing integer cast and a shared test schema in portfolio, constraint-violating
  fixture values in analytics, and a fixture built from the real clock in support-console.
- **Root-cause stage.** code-generation (units u3-identity, u4-account, u5-market-data,
  u6-portfolio, u10-analytics, u13-support-console).
- **Planned fix.** Unqualify the four `identity.*` function calls; correct the two account test
  expectations; exclude `_sqlx_migrations` from the market-data bytea assertion; cast
  `make_interval(days => $1::int)` and give each portfolio adapter test its own disposable schema;
  use 32+ character `user_ref` values in the analytics fixtures; build the support-console expired
  grant relative to the test's `NOW`.
- **Estimated impact.** Small, localized changes to test files plus two one-line SQL/code fixes;
  roughly one to two hours of work; no architectural change; low risk. No quality target is
  relaxed.

## Re-run after the loop-back (2026-11-09, second pass)

All six reopened units were repaired, reviewed and re-verified. The Loop-Back Log above is
unchanged: this second pass is the forward replay of the same loop-back, not a new one.

| Suite | Result |
|---|---|
| `cargo test --workspace` | 1349 passed, 0 failed, 88 ignored |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `yarn --cwd web test` | 142 passed |
| `yarn --cwd web lint` / `build` / `size` | exit 0 / exit 0 / budgets met |
| `yarn --cwd support-console test` / `lint` / `build` | 180 passed / exit 0 / exit 0 |
| asset-registry adapter | 7 passed |
| audit adapter | 14 passed |
| account adapter | 13 passed |
| market-data adapter | 17 passed |
| portfolio adapter | 6 passed |
| ledger adapter | 12 passed |
| notification adapter | 14 passed |
| analytics adapter | 8 passed |
| identity adapter | 8 passed, 1 failed |

The one identity failure is `protect_and_unprotect_are_security_definer_functions_the_app_may_execute`:
the key row is deleted (verified: `SELECT count(*) FROM pgsodium.key WHERE key_id = …` returns 0), and a
fresh database session still decrypts the value. This pgsodium build keeps keys in a cluster-wide
in-memory cache that deleting the `pgsodium.key` row does not evict, so "a destroyed key no longer
decrypts" cannot hold until the cluster restarts. The person chose to record this as a known
limitation and proceed to the gate; it is not a lowered target.

Updated matrix rows: UT-03 is now Met (180 passed); INT-01 is Met for every suite except the identity
crypto-shredding assertion, which is Unverified for the reason above; TRACE-01 remains a gate finding.

## Readiness

Build-ready and test-ready for the SPA and the passing services. The stage is **not** ready to
hand off: the adapter suites, the support-console suite and the cross-unit traceability gate still
fail, and the environment-dependent NFR targets have no measurement path in this plan.
