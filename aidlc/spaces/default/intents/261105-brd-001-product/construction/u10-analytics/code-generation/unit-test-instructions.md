# Unit Test Instructions — u10-analytics

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test`.

## Exact command for THIS unit

```
cargo test -p analytics-domain -p analytics-application -p analytics-infrastructure -p analytics-http
```

Integration tests need `TEST_DATABASE_URL`:

```
cargo test -p analytics-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI): `cargo llvm-cov -p analytics-<crate> --fail-under-lines 80`.

## Coverage and volume

80% line coverage per crate; five to eight tests per component, happy path plus two error/edge
cases; target 90+ tests.

## Mocking

- `application` uses fakes for C3 consent, Vault salt, C10 audit, the token registry, and the clock.
- `infrastructure` runs against a real Postgres schema.
- Time is injected for expiry, retention, and cohort tests.

## Test data

- Fixed subjects and a fixed test salt; catalogue events from the spec; fixtures for email, 0x
  address, base58, IPv4, and secret-shaped tokens; flags with missing owner/expiry.
