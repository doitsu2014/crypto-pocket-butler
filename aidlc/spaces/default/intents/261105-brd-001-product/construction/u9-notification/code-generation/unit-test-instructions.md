# Unit Test Instructions — u9-notification

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test`.

## Exact command for THIS unit

```
cargo test -p notification-domain -p notification-application -p notification-infrastructure -p notification-http
```

Integration tests need `TEST_DATABASE_URL`:

```
cargo test -p notification-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI): `cargo llvm-cov -p notification-<crate> --fail-under-lines 80`.

## Coverage and volume

80% per crate; five to eight tests per component, happy path plus two error/edge cases; target 90+
tests.

## Mocking

- `application` fakes C3 contact, the email provider, C10, the clock, and the id generator.
- `infrastructure` runs against a real Postgres schema.
- Time is injected for backoff and retention tests.

## Test data

Drift-crossing keys, preference defaults, mandatory security alerts with notifications off, provider
failures (permanent address vs transient), sanitised payloads, and an erasure round trip.
