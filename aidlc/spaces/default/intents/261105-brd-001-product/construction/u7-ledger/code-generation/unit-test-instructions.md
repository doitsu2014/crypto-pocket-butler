# Unit Test Instructions — u7-ledger

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test`.

## Exact command for THIS unit

```
cargo test -p ledger-domain -p ledger-application -p ledger-infrastructure -p ledger-http
```

Integration tests need `TEST_DATABASE_URL`:

```
cargo test -p ledger-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI): `cargo llvm-cov -p ledger-<crate> --fail-under-lines 80`.

## Coverage and volume

80% per crate; five to eight tests per component, happy path plus two error/edge cases; target 120+
tests.

## Mocking

- `application` fakes C5, C4, C10, the clock, and the id generator.
- `infrastructure` runs against a real Postgres schema with the three roles and grants.
- Money fixtures are decimal strings; nothing uses floats.

## Test data

- Buy/partial-sell/transfer sequences with expected realized/unrealized; an exchange row and a CSV
  row of the same trade; a correction chain; an ingested sell beyond holdings; coverage gaps and
  their clearing.
