## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-11-09T06:26:38Z
**Iteration:** 1
**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | services/portfolio/infrastructure/tests/repositories.rs > module doc and `pool()` | The module header says "the runtime role's insert-only grant on snapshots is part of what these tests prove", and `unit-test-instructions.md` says infrastructure runs against a real Postgres schema "with both roles", but every test in this file connects through the one `TEST_DATABASE_URL` pool (the `postgres` superuser in the recorded command) and no test issues `SET ROLE` or asserts a privilege. The SELECT/INSERT-only, no-DELETE control on `valuation_snapshots` that the security design names is stated in the migration and exercised by no test, so a grant regression would ship unnoticed. | Add one assertion that the runtime role's privileges on `valuation_snapshots` are exactly SELECT and INSERT (for example `has_table_privilege('portfolio_svc','valuation_snapshots','UPDATE'/'DELETE')` is false), or run the snapshot repository under `SET ROLE portfolio_svc` and assert that an update or delete is refused. | New |
| R-02 | Minor | services/portfolio/infrastructure/tests/repositories.rs > module doc, lines 4-7 | The header says "Each test points the migration at a disposable schema so the assertions do not depend on shared state", but `pool()` just connects to `TEST_DATABASE_URL`, and the comment added at `the_run_lease_is_single_flight` now says "Every test shares one schema". The file contradicts itself, and the shared schema is the root cause of the flake this repair worked around. | Either give `pool()` a per-run schema so the header becomes true, or rewrite the header to state that the suite shares one schema and stays stable through unique test-scoped names such as the per-run job name. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/code-generation/code-summary.md > Deviations and follow-ups > Integration tests | This deviation still says the 6 integration tests "were not executed because no disposable `TEST_DATABASE_URL` was available in this environment", while the loop-back section below reports them 6/6 green. A reader at the gate cannot tell which statement is current. | Update the deviation to record that the adapter suite ran green after the repair and when, or mark it superseded, so the summary carries one current statement about those 6 tests. | New |
| R-04 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u6-portfolio/code-generation/code-summary.md > Deviations and follow-ups > Coverage | The 80% per-crate line-coverage floor is recorded as unverified, and I could not verify it either: `cargo llvm-cov --version` reports "no such command: llvm-cov" in this environment. The floor is treated as met only by inference from the test count, not by a measurement. | Run `cargo llvm-cov -p portfolio-domain`, `-p portfolio-application`, `-p portfolio-infrastructure` and `-p portfolio-http` with `--fail-under-lines 80` in CI or an environment that has cargo-llvm-cov, and record the four per-crate results in code-summary. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo test -p portfolio-domain -p portfolio-application -p portfolio-infrastructure -p portfolio-http` | PASS: 198 passed, 6 ignored (15 suites) | Matches the code summary's "198 passed / 6 ignored" exactly. |
| `cargo fmt --all -- --check` | PASS: no output | Formatting clean, as recorded. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS: no issues found | Lint clean, as recorded. |
| `TEST_DATABASE_URL='postgres://postgres:test@localhost:55432/postgres' cargo test -p portfolio-infrastructure --features integration -- --ignored` | PASS: 6 passed, 12 filtered out | All six adapter tests green, including `the_prune_removes_only_old_snapshots` (the `$1::int` cast) and `the_run_lease_is_single_flight` (per-run job name). |
| Same integration command, run a second time in the same schema | PASS: 6 passed, 12 filtered out | Repeatability is the failure mode this repair addresses (the old hardcoded job name failed once its own row was left behind); the suite now stays green on a re-run. |
| `cargo llvm-cov --version` | FAIL: no such command: `llvm-cov` | The coverage floor could not be measured here; see the coverage finding. |
| `git diff` against `source-manifest.json` | MATCH: under `services/portfolio` only `infrastructure/src/migrations.rs` (the `$1::int` cast) and `infrastructure/tests/repositories.rs` (the per-run job name) differ, exactly the two claimed paths | The differential claim is accurate; no unclaimed Portfolio source changed. Nothing else in the working tree belongs to this unit (the `infra/swarm-stack.yml` diff is the web service's `read_only`/tmpfs change). |

### Summary

The two loop-back repairs are correct and verified against a live Postgres, and the remaining test-suite risk is in the same file: the suite's own header claims per-test disposable schemas and runtime-role privilege proof that the code does not provide. The gate should also route the out-of-scope observation already recorded in the summary to its owning units, where `services/reporting/infrastructure/src/migrations.rs > prune_briefs` and `services/job-orchestration/infrastructure/src/lib.rs > prune_runs` still bind an `i64` to `make_interval(days => $1)` and would fail the same way when run.
