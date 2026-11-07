# Unit Test Instructions — u4-account

## Framework

Rust's built-in test harness (`cargo test`) with `tokio::test` for async use cases. Property-style
cases are plain table-driven tests; no extra framework is added.

## Exact command for THIS unit

Run from the repository root:

```
cargo test -p account-domain -p account-application -p account-infrastructure -p account-http
```

Integration tests that need Postgres use `DATABASE_URL` pointing at a disposable schema and are
excluded when it is unset:

```
cargo test -p account-infrastructure --features integration
```

Per-crate coverage (checked in CI, not by this unit command):

```
cargo llvm-cov -p account-domain --fail-under-lines 80
cargo llvm-cov -p account-application --fail-under-lines 80
cargo llvm-cov -p account-infrastructure --fail-under-lines 80
cargo llvm-cov -p account-http --fail-under-lines 80
```

## Coverage targets

80% line coverage per crate. Build and Test measures the floor on the full assembled suite; a
unit-scoped run that exits non-zero only on that threshold with every test passing is recorded as
that, not as a failed test.

## Test volume (Standard strategy)

Five to eight tests per component, each file covering the happy path and at least two error or edge
cases. Target: 120+ tests across the four crates.

## Mocking and stubbing

- `application` tests use hand-written port fakes in `tests/common/fakes.rs`: venue/RPC providers,
  Asset Registry, Vault, Supabase Auth admin, Audit, Notification, and the clock.
- `infrastructure` tests run against a real Postgres schema; no repository is mocked.
- Time is injected through a `Clock` port so lease and staleness tests are deterministic.

## Test data

- Fixed UUIDs and decimal strings; no binary floating point anywhere.
- Every credential fixture is a placeholder string, never a plausible key.
- CSV fixtures live under `tests/fixtures/csv/` for Binance, OKX, and generic formats, including a
  malformed row and a duplicate row.

## TDD record

Each Red step records the failing command output before its Green step. A test that passes without
the implementation is rejected and rewritten.
