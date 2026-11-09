# Unit Test Instructions — u6-portfolio

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test`.

## Exact command for THIS unit

```
cargo test -p portfolio-domain -p portfolio-application -p portfolio-infrastructure -p portfolio-http
```

Integration tests need `TEST_DATABASE_URL`:

```
cargo test -p portfolio-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI): `cargo llvm-cov -p portfolio-<crate> --fail-under-lines 80`.

## Coverage and volume

80% per crate; five to eight tests per component, happy path plus two error/edge cases; target 130+
tests.

## Mocking

- `application` fakes C5 holdings, C6 prices/FX, C7 trades, C9 alerts, C10 audit, Identity preference,
  and the clock.
- `infrastructure` runs against a real Postgres schema with both roles.

## Test data

Multi-currency holdings with quotes and FX; unpriced and degraded C5 cases; a 50/50 target with a
60% actual; snapshots with frozen inputs and digest; intent previews with shortfall; a recorded trade
and its overlay; guardrail crossing sequences.
