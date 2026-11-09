# Integration Test Instructions

Standard test strategy: unit tests are covered per unit by Code Generation; this file covers the
cross-unit and adapter integration layer.

## Framework

- Rust's built-in `cargo test` harness, `#[ignore]`-gated adapter suites that need a real
  PostgreSQL, and `tokio::test` for async use cases.
- Vitest + Testing Library (jsdom) for the SPA and the support console. A jsdom journey is
  integration evidence, not browser evidence.

## Environment

A disposable PostgreSQL 15+ database with `pgsodium` available for the identity suite, reached
through `TEST_DATABASE_URL`. Locally:

```
docker run -d --name cpb-test-pg -e POSTGRES_PASSWORD=test -e POSTGRES_USER=postgres -p 55432:5432 postgres:15
export TEST_DATABASE_URL='postgres://postgres:test@localhost:55432/postgres'
```

Each suite creates a disposable schema (or the unit's harness does) so the assertions do not
depend on shared state or execution order.

## Commands for THIS project

```
cargo test --workspace
cargo test --manifest-path services/asset-registry/Cargo.toml --test repositories -- --ignored
cargo test --manifest-path services/identity/Cargo.toml --test repositories -- --ignored
cargo test --manifest-path services/audit/Cargo.toml --test repositories -- --ignored
cargo test -p account-infrastructure --test repositories -- --include-ignored
cargo test -p market-data-infrastructure --features integration -- --ignored
cargo test -p portfolio-infrastructure --features integration -- --ignored
cargo test -p ledger-infrastructure --features integration -- --ignored
cargo test -p notification-infrastructure --test repositories -- --include-ignored
cargo test -p analytics-infrastructure --test repositories -- --include-ignored
yarn --cwd web test
yarn --cwd support-console test
```

`reporting` and `job-orchestration` declare the `integration` feature but ship no adapter test
module; their application-level tests run under `cargo test --workspace`.

## Expected coverage

- Unit tests plus adapter integration tests for every boundary that talks to PostgreSQL.
- The Rust 80%-per-crate line floor is recorded in team memory, but no coverage tool is chosen in
  the repository and the plan runs no CI Pipeline, so this stage records the suite results instead
  of a coverage percentage.
- Every suite covers the happy path and at least two error or edge cases.

## Test data

Disposable schemas per test, generated UUIDs, and migrations applied by the suite itself. No
production data is copied. Test values must satisfy the schema constraints (for example the
analytics `user_ref` length is 32–128 characters).
