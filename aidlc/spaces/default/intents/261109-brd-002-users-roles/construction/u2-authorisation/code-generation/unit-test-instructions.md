# Unit Test Instructions — u2-authorisation

**Framework:** Rust's built-in test harness; `#[ignore]` adapter tests need a disposable Postgres via `TEST_DATABASE_URL`.
**Test strategy:** Standard — 5–8 tests per component, unit plus integration for the key boundary, happy path plus at least two error/edge cases per file.

## Exact commands (unit-scoped)

| Layer | Command |
|---|---|
| Runner check | `cargo test -p identity --test authorisation_domain -- --list` (0 tests before the first Red) |
| Domain | `cargo test -p identity --test authorisation_domain` |
| Application | `cargo test -p identity --test authorisation_application` |
| API | `cargo test -p identity --test authorisation_http` |
| Repository (ignored) | `TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored authorisation` |
| Whole workspace gate | `cargo test --workspace` |

Every run command above names one test target, so Build and Test can run each Unit's suite
without rerunning the whole workspace once per Unit.

## Coverage

80% line coverage per Rust crate for the lines this unit adds, measured with
`cargo llvm-cov --workspace`. Pre-existing crates may remain below the floor; report per-crate
results honestly and never lower a threshold to make a run pass.

## Mocking and stubbing

- Hand-written fakes in `services/identity/tests/common/fakes.rs`: `FakeStatusReader` (serving,
  timing out, erroring), `FakeAuditSink` (accepting, failing on the nth write), `FakeRoleWriter`
  (applying, failing), `FakeObservationSink` (recording metric calls).
- HTTP tests drive the real Axum router with a real service-token allowlist entry and a signed
  test JWT.

## Test data

- Fixed user ids and UUIDs; no personal data; documentation IPs (`192.0.2.0/24`).
- The repository tests create and drop a dedicated schema per run and rely on the existing
  `0001`–`0004` migrations.

## Red/Green evidence

Record the failing command and its output for each Red step and the passing run for each Green
step in `code-summary.md`.
