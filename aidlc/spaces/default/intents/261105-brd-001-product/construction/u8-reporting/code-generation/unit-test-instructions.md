# Unit Test Instructions — u8-reporting

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test`.

## Exact command for THIS unit

```
cargo test -p reporting-domain -p reporting-application -p reporting-infrastructure -p reporting-http
```

Integration tests need `TEST_DATABASE_URL`:

```
cargo test -p reporting-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI): `cargo llvm-cov -p reporting-<crate> --fail-under-lines 80`.

## Coverage and volume

80% per crate; five to eight tests per component, happy path plus two error/edge cases; target 80+
tests.

## Mocking

- `application` fakes C3 due-users, C8 valuation, C7 P&L, C9 alerts, C10, and the clock.
- `infrastructure` runs against a real Postgres schema.

## Test data

A complete valuation brief, a degraded (partial) brief, a failed brief, a missing figure (em dash),
a duplicate period, a daily/weekly boundary, and an email submission with the idempotency key.
