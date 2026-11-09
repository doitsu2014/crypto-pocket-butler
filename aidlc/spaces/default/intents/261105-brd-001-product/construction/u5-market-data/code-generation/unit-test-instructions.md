# Unit Test Instructions — u5-market-data

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test` for async use cases.

## Exact command for THIS unit

```
cargo test -p market-data-domain -p market-data-application -p market-data-infrastructure -p market-data-http
```

Integration tests that need Postgres use `TEST_DATABASE_URL` and are ignored when it is unset:

```
cargo test -p market-data-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI, not the unit command):

```
cargo llvm-cov -p market-data-domain --fail-under-lines 80
cargo llvm-cov -p market-data-application --fail-under-lines 80
cargo llvm-cov -p market-data-infrastructure --fail-under-lines 80
cargo llvm-cov -p market-data-http --fail-under-lines 80
```

## Coverage

80% line coverage per crate; Build and Test measures the floor on the full assembled suite.

## Volume (Standard)

Five to eight tests per component, happy path plus at least two error or edge cases. Target: 90+
tests.

## Mocking

- `application` uses hand-written fakes for the price/FX sources, C4, Vault, Audit, the token
  registry, and the clock.
- `infrastructure` runs against a real Postgres schema; no repository mocks.
- Time is injected so freshness, leases, and circuit-breaker tests are deterministic.

## Test data

Fixed decimal strings ("123.45678901"), fixed timestamps, and source fixtures with a delayed feed,
a failing primary, a malformed payload, and a zero price.
