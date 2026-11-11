# Unit Test Instructions — u5-account-lifecycle

This unit is a slice inside the existing Identity service (`services/identity`). It adds no new crate, so the test runner, linter, and formatter already exist and are green before the first test-first step. Every command below is scoped to this unit only.

## Test framework and configuration

- **Runner:** `cargo test`, with the `identity` package's existing `[dev-dependencies]` (axum, sqlx, tower, uuid, jsonwebtoken, http-body-util, async-trait, rsa, rand, base64, chrono, serde_json).
- **Test layout (matches the existing `tests/` tree under `services/identity/tests/`):**
  - `tests/account_lifecycle_domain.rs` — domain rules: the status machine, the 30-day window, the `DeletionRequest` extension, `anonymise`/`erased_subject_hash`.
  - `tests/account_lifecycle_application.rs` — use cases against hand-written port fakes (`tests/common/fakes.rs`).
  - `tests/account_lifecycle_http.rs` — Axum router tests with status codes and RFC 7807 problem bodies.
  - `tests/account_lifecycle_adapters.rs` — `#[ignore]`d adapter/migration tests against a real Postgres schema.
- **Style gates (unchanged, run by Build and Test on the assembled project):** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `bash scripts/check-boundaries.sh`.

## How to run THIS unit's tests

Verify the runner before the first Red step (brownfield readiness):

```bash
cargo test -p identity --test domain --test application --test http
```

The unit-scoped command for this unit (the exact files it adds):

```bash
cargo test -p identity \
  --test account_lifecycle_domain \
  --test account_lifecycle_application \
  --test account_lifecycle_http
```

The ignored adapter/migration suite (requires a disposable Postgres, provisioned per gate run):

```bash
TEST_DATABASE_URL=postgres://…/identity_test cargo test -p identity --test account_lifecycle_adapters -- --ignored
```

Run the unit's inline domain unit tests alongside the integration tests:

```bash
cargo test -p identity-domain --lib
```

A bare project-wide `cargo test --workspace` is **not** a unit command and is never used here: Build and Test executes each unit's commands, and an unscoped command would rerun the whole suite once per unit.

## Expected coverage targets

- **Per Rust crate:** 80% line coverage, measured with `cargo llvm-cov`. This is a whole-project floor: Build and Test checks it on the full test suite of the assembled project, not on one unit's run. If a unit-scoped run exits non-zero only on that threshold while every selected test passes, record it with this unit's test results as a non-failure.
- **Per test file:** happy path plus at least two error/edge cases; a test that passes regardless of implementation is not acceptable.
- **Volume (Standard strategy):** 5–8 tests per component, plus integration coverage of the key boundaries (the migration, the tombstone lookup, the one-open-request guard, the outbox).

## Mocking / stubbing guidance

- Business rules are tested in `application` use cases against hand-written port fakes (`tests/common/fakes.rs`), never against a live database or real HTTP client.
- HTTP boundary tests mount the Axum router with an `AppState` wired to those fakes and assert status codes and RFC 7807 `ProblemDetails` bodies.
- The audit trail (`Trail`) and the outbox drainer use a fake `Audit`/HTTP client; the erasure cascade's external steps (`AuthAdmin::delete_user`, consent/preference deletes) use fakes recording the call order and idempotency keys.
- `Clock` is a port: tests inject a fixed `DateTime<Utc>` so the 30-day midnight-UTC window and the "restore after the deadline is 410" case are deterministic.

## Test data management

- Domain and application tests use `UserId::generate()`, `DeletionRequestId::generate()`, and fixed timestamps; no database.
- Adapter tests run only when `TEST_DATABASE_URL` is set, against a disposable schema the migration applies; they are `#[ignore]`d by default and are a blocking gate run by the same local command in CI. Each test case provisions its own rows and rolls back or drops the schema so cases stay independent.
- The erased-subject tombstone test seeds an erased subject (status `deleted`, `erased_subject_hash` set, `user_personal` deleted) and asserts its token is denied rather than provisioned.
