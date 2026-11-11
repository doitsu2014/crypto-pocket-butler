# Unit Test Instructions — u4-preferences-consent

**Framework:** Rust's built-in test harness; `#[ignore]` adapter and migration tests need a disposable Postgres via `TEST_DATABASE_URL`.
**Test strategy:** Standard — 5–8 tests per component, unit plus integration for the key boundary, happy path plus at least two error/edge cases per file.

## Exact commands (unit-scoped)

| Layer | Command |
|---|---|
| Runner check | `cargo test -p identity --test consent_domain -- --list` (0 tests before the first Red) |
| Domain | `cargo test -p identity --test consent_domain` |
| Application | `cargo test -p identity --test consent_application` |
| API | `cargo test -p identity --test consent_http` |
| Repository + migration (ignored) | `TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored consent` |
| Whole workspace gate | `cargo test --workspace` |

## Coverage

80% line coverage per Rust crate for the lines this unit adds, measured with `cargo llvm-cov
--workspace`. Pre-existing crates may remain below the floor; report per-crate results honestly and
never lower a threshold to make a run pass.

## Mocking and stubbing

- Hand-written fakes in `services/identity/tests/common/fakes.rs`: `FakePreferencesRepository`,
  `FakeConsentRepository` (state, history, insert, and a withdraw that can fail its outbox),
  `FakeAuditOutbox`, `FakeUserLookup`, `FakeObservationSink`.
- HTTP tests drive the real Axum router with a signed test JWT and the service-token allowlist.
- The drainer is tested against a stub axum server that answers 202, 500, and 400, asserting the
  attempts/next-attempt state and the idempotency header.

## Test data

- Fixed user ids and UUIDs; no personal data; documentation IPs (`192.0.2.0/24`).
- The migration tests seed legacy `purpose`/`granted`/`recorded_at` rows, run `0005` up, assert the
  mapping, run the down, assert the collapse, and **run the down a second time** to prove the
  `IF NOT EXISTS` archive path.

## Red/Green evidence

Record the failing command and its output for each Red step and the passing run for each Green step
in `code-summary.md`.
