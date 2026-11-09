# Unit Test Instructions — u11-job-orchestration

## Framework

Rust's built-in harness (`cargo test`) with `tokio::test`.

## Exact command for THIS unit

```
cargo test -p job-orchestration-domain -p job-orchestration-application -p job-orchestration-infrastructure -p job-orchestration-http
```

Integration tests need `TEST_DATABASE_URL`:

```
cargo test -p job-orchestration-infrastructure --features integration -- --ignored
```

Per-crate coverage (CI): `cargo llvm-cov -p job-orchestration-<crate> --fail-under-lines 80`.

## Coverage and volume

80% per crate; five to eight tests per component, happy path plus two error/edge cases; target 80+
tests.

## Mocking

- `application` fakes the run/lease/heartbeat repositories, the clock, the id generator, the target
  client and the audit client.
- `infrastructure` runs against a real Postgres schema.

## Test data

A daily job due now; a window already succeeded; a failed row with a future retry; an expired
running lease; a 24 h-old watermark gap; a 7-day backfill; a manual run of a covered window; a 202
answer with a runId; a 409; a direct 4xx abandonment; a stale and an absent heartbeat.
