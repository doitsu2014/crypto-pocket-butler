# Unit Test Instructions — u6-jurisdiction-gate

**Framework:** Rust's built-in test harness; `sqlx` adapter tests use `#[ignore]` and a disposable Postgres via `TEST_DATABASE_URL`.
**Test strategy:** Standard — 5–8 tests per component, unit plus integration for the key boundary, happy path plus at least two error/edge cases per file.

## Exact commands (unit-scoped)

| Layer | Command | Runs before the first test-first cycle |
|---|---|---|
| Runner check | `cargo test -p identity-domain -- --list` | Yes |
| Domain (business rules) | `cargo test -p identity-domain jurisdiction` | Yes |
| Application (use cases against port fakes) | `cargo test -p identity-application jurisdiction` | Yes |
| API (Axum router) | `cargo test -p identity-http jurisdiction` | Yes |
| Repository (adapter, ignored by default) | `TEST_DATABASE_URL=postgres://… cargo test -p identity-infrastructure -- --ignored jurisdiction` | After the disposable database exists |
| Gateway middleware | `cargo test -p gateway-http x_client_ip` | Yes |
| Whole workspace gate | `cargo test --workspace` | Yes |

The first executable step's command is `cargo test -p identity-domain -- --list`; if it fails because no tests exist yet, the Red step adds the first failing test and records its output.

### Verified commands for this repository layout

The four Clean Architecture crates (`identity-domain`, `identity-application`,
`identity-infrastructure`, `identity-http`) carry no `tests/` target of their own. As in every
other service in this repository, the integration suites live in the `identity` package:
`services/identity/tests/{domain,application,http,repositories}.rs`. `cargo test -p
identity-domain` therefore compiles and runs with **0 tests**. The commands actually used for
this unit are:

| Layer | Verified command |
|---|---|
| Runner check | `cargo test -p identity-domain -- --list` (0 tests; the crate has no test target) |
| Domain (business rules) | `cargo test -p identity --test domain jurisdiction` |
| Application (use cases against port fakes) | `cargo test -p identity --test application jurisdiction` |
| API (Axum router) | `cargo test -p identity --test http jurisdiction` |
| Repository (adapter, ignored by default) | `TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored jurisdiction` |
| Gateway middleware | `cargo test -p gateway --test http client_ip` |
| Whole workspace gate | `cargo test --workspace` |

## Coverage

80% line coverage per Rust crate. Measure with `cargo llvm-cov --workspace` (the tool is not installed on this machine; install it before relying on the coverage gate). The full intent-level verification command remains authoritative for checkpoints.

## Mocking and stubbing

- Hand-written port fakes in `services/identity/tests/common/fakes.rs` (or the crate-local `tests/fakes.rs`) for `GeoLocator`, `ListRepository`, `AuditSink`, and `Clock`.
- The provider fake can return allowed/restricted, time out, return 5xx, or fail the circuit; the audit fake can fail on the first attempt and succeed on the retry.
- No mock of the Axum router: HTTP tests drive the real router.

## Test data

- Domain and application tests use fixed country codes (`VN`, `US`, `ZZ` invalid) and a three-entry list fixture.
- Repository tests create and drop a dedicated schema in the disposable Postgres per run; the migration pair is applied by the test setup and rolled back after.
- No personal data is used in any fixture; the IP values are documentation ranges (`192.0.2.0/24`).

## Red/Green evidence

Each Red step records the exact failing command and its output; each Green step records the same command passing. The plan checkbox is ticked only when the evidence exists.
