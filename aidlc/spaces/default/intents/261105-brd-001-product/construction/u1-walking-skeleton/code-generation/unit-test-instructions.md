# Unit Test Instructions — u1-walking-skeleton

## Framework and setup

- **Rust services:** built-in `cargo test` with `tokio` test runtime; `sqlx` for integration tests against a real Postgres schema.
- **Frontend shell:** `vitest` with `@testing-library/react` and `jsdom`.
- Toolchain: pinned Rust stable; Node with corepack and Yarn ≥6.

## Commands (scoped to this Unit)

Rust — asset registry:

```
cargo test --manifest-path services/asset-registry/Cargo.toml
```

Rust — gateway:

```
cargo test --manifest-path services/gateway/Cargo.toml
```

Frontend shell:

```
yarn --cwd web test
```

Format and lint (not tests, but part of the gate):

```
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
yarn --cwd web lint
```

These commands exist and run before the first test-first cycle: the runner bootstrap in plan
Step 2 creates a placeholder test per service and a vitest config, so each command exits 0 on
the skeleton.

## Expected coverage targets

- **80% line coverage per Rust crate**, the team's floor. Build and Test checks it on the full
  assembled suite; a Unit-scoped run does not judge the project-wide threshold.
- Every test file covers the happy path and at least two error or edge cases.
- No test may pass regardless of the implementation.

## Mocking and stubbing

- Business rules are tested in `application` against port fakes; no database or network in unit tests.
- Each adapter gets one integration test against a real Postgres schema, using a disposable
  schema and a migration run.
- The gateway's JWKS validation is tested with a local test key set, never a live Supabase.

## Test data

- Fixtures are small, explicit, and created per test.
- Decimal values use `rust_decimal` string literals, never floats.
- No credentials or real addresses appear in fixtures; use synthetic identifiers.

## Recorded commands (implementation)

The unit-scoped commands above are the ones that were run. Two additions were needed for the
real-Postgres adapter tests and the workspace-wide gate:

```
# Adapter integration tests against a disposable Postgres (creates and drops a schema):
TEST_DATABASE_URL=postgres://test:test@localhost:55432/cpb_test \
  cargo test --manifest-path services/asset-registry/Cargo.toml --test repositories -- --ignored

# Workspace-wide gate (same as CI):
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

The web commands are `yarn --cwd web test` (vitest), `yarn --cwd web lint` (ESLint), and
`yarn --cwd web build` (tsc + vite). Yarn 4.18.1 is pinned through corepack; no Yarn ≥ 6 release
exists, so 4.18.1 is the highest available Berry release.

