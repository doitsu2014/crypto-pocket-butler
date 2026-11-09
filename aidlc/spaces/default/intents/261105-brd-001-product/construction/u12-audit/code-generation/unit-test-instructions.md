# Unit Test Instructions — u12-audit

## Framework and setup

- **Rust:** built-in `cargo test` with the `tokio` test runtime; `sqlx` against a real Postgres
  schema for adapter tests.
- Toolchain: the pinned Rust stable already used by the walking skeleton.

## Commands (scoped to this Unit)

```
cargo test --manifest-path services/audit/Cargo.toml
```

Adapter integration tests (need a disposable Postgres; skipped by `#[ignore]` otherwise):

```
TEST_DATABASE_URL=postgres://test:test@localhost:55432/cpb_test \
  cargo test --manifest-path services/audit/Cargo.toml --test repositories -- --ignored
```

Format and lint (part of the gate, not tests):

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Boundary gate:

```
scripts/check-boundaries.sh
```

## Expected coverage targets

- **80% line coverage per Rust crate**, the team's floor, checked by Build and Test on the
  assembled suite.
- Every test file covers the happy path and at least two error or edge cases.
- No test may pass regardless of the implementation.

## Mocking and stubbing

- Business rules are tested in `application` against port fakes — no database, no network.
- One integration test per adapter against a disposable Postgres schema, with migrations run
  first so the partitioned table and its grants exist.
- The JWT path is tested with a local test key set, never a live Supabase.
- Time is injected through a clock port so expiry (BR3.2) and retention cutoffs (BR4.1) are tested
  at their boundaries rather than with sleeps.

## Test data

- Fixtures are explicit and created per test.
- No real addresses, credentials, or tokens: synthetic identifiers only.
- A fixture that must look like a secret (for the BR1.3 rejection tests) uses an obviously fake
  value, never a live-looking key.

## Recorded commands (implementation)

The commands above are the ones this Unit runs. The Postgres adapter suite is the only one that
needs a running database; when it is not running those tests are ignored and the rest still run.
