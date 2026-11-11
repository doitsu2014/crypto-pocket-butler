# Unit Test Instructions — u1-user-registry

**Framework:** Rust's built-in test harness; `#[ignore]` adapter tests need a disposable Postgres via `TEST_DATABASE_URL`.
**Test strategy:** Standard — 5–8 tests per component, unit plus integration for the key boundary, happy path plus at least two error/edge cases per file.

## Exact commands (verified layout)

| Layer | Command |
|---|---|
| Runner check | `cargo test -p identity-domain -- --list` (0 tests; the crate has no test target) |
| Domain | `cargo test -p identity --test domain user_registry` |
| Application | `cargo test -p identity --test application user_registry` |
| API | `cargo test -p identity --test http user_registry` |
| Repository (ignored) | `TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored user_registry` |
| Migrate binary | `cargo run -p identity --bin migrate` against the disposable database |
| Whole workspace gate | `cargo test --workspace` |

## Coverage

80% line coverage per Rust crate, measured with `cargo llvm-cov --workspace` (installed in the U6 run). Report per-crate results honestly; pre-existing crates may remain below the target.

## Mocking and stubbing

- Hand-written port fakes for `UserRepository`, `JurisdictionGate`, `AuditSink`, and `Clock` in `services/identity/tests/common/fakes.rs`.
- The gate fake can clear or deny; the audit fake can fail once then succeed.
- HTTP tests drive the real Axum router.

## Test data

- Fixed subject strings hashed in tests; documentation IPs (`192.0.2.0/24`); no personal data.
- Repository tests create and drop a dedicated schema per run and apply `0004_user_registry_extensions.up.sql`; the down migration is exercised in its own test.

## Red/Green evidence

Record the failing command and its output for each Red step and the passing run for each Green step.
