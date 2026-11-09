## Review

**Verdict:** READY
**Reviewer:** aidlc-architecture-reviewer-agent
**Date:** 2026-11-09T05:04:29Z
**Iteration:** 1
**Verdict Dictated:** no

### Findings

| ID | Severity | Location | Finding | Required action | Status |
|---|---|---|---|---|---|
| R-01 | Major | aidlc/spaces/default/intents/261105-brd-001-product/construction/u5-market-data/code-generation/code-summary.md > HTTP surface, `POST /v1/internal/run/{jobName}` | The delivered run endpoint answers `price-refresh-top100` and `price-refresh-tail`, but the shared C11 contract (inception/contract-design/contract-summary.md, "Amendment — Job Orchestration (U11) run call and catalogue") lists Market Data's jobs as `market-data-refresh` and `market-data-fx`. A Job Orchestration caller following the shared contract gets a 404 and scheduled price refreshes never run; there is also no separate FX-only job in the service. The contradiction runs through the unit's functional spec, security design, performance design and code, and predates this repair. | Decide which job names are authoritative, align the C11 catalogue and the Market Data endpoint/allow-list/spec to the same names, and record the resolution in both documents so U11 and U5 stop disagreeing. | New |
| R-02 | Minor | services/market-data/infrastructure/tests/migrations.rs > `the_schema_declares_no_large_object_or_bytea_storage` | The repair's `table_name NOT LIKE '_%' ESCAPE ''` filter skips every underscore-prefixed table, not just sqlx's `_sqlx_migrations`; an application table named with a leading underscore would silently escape the no-bytea check while the assertion message still claims the whole schema is covered. | Exclude exactly the tool-owned table (for example `table_name <> '_sqlx_migrations'`) so the assertion keeps its full intent for application tables. | New |
| R-03 | Minor | aidlc/spaces/default/intents/261105-brd-001-product/construction/u5-market-data/code-generation/code-summary.md > Tests, coverage command | The 80% per-crate line coverage floor has no run behind it: the summary records the four `cargo llvm-cov` commands as CI-only, and `cargo-llvm-cov` is not installed in this workspace, so the floor is unmeasured here. The repair changed no product code, so the risk is unchanged from the prior state. | Run the four per-crate coverage commands in CI (or install `cargo-llvm-cov` and run them here) and record the measured percentages with the result. | New |

### Validation Tool Results

| Tool | Result | Interpretation |
|---|---|---|
| `cargo test -p market-data-domain -p market-data-application -p market-data-infrastructure -p market-data-http` | PASS: 140 passed, 17 ignored | Matches the recorded unit result; the 17 ignored are the database-backed adapter tests. |
| `cargo fmt --all -- --check` | PASS (no output) | Formatting is clean across the workspace. |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS: no issues found | The lint gate is clean workspace-wide. |
| `TEST_DATABASE_URL='postgres://postgres:test@localhost:55432/postgres' cargo test -p market-data-infrastructure --features integration -- --ignored` | PASS: 17 passed, 3 filtered out | Confirms the repaired assertion passes against a real Postgres schema, including the sqlx bookkeeping table that previously made it fail. |

### Summary

The loop-back repair is correct and minimal: the bytea assertion now skips sqlx's `_sqlx_migrations` bookkeeping column, the full unit and integration suites pass on the exact file under review, and only the single path in `source-manifest.json` changed. The item to settle before relying on the run endpoint is the C11 job-name contradiction with the shared contract; coverage measurement and the test's broad underscore filter are minor follow-ups.
