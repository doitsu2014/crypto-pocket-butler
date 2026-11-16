# U1 Identity Auth Domain — Unit Test Instructions

## Framework and setup

- Rust workspace test runner: Cargo (`cargo test`).
- The service package is `identity`; the existing service-level unit test suite is run with `cargo test -p identity --lib`.
- Domain crate tests use `cargo test -p identity-domain`; U1 application behavior is exercised in service-package tests against hand-written port fakes.
- Adapter/migration integration tests require a disposable `supabase/postgres:15.8.1.060` database because suites may create/drop roles. Set `TEST_DATABASE_URL` to that throwaway database only.
- Do not install a new framework or change workspace dependencies unless the approved plan is revised and re-approved.

## Exact U1 test commands

- `cargo test -p identity-domain`
- `cargo test -p identity --lib`

Run the applicable scoped command after each U1 domain/application red-green-refactor cycle. For a migration or SQLx adapter step, run the exact ignored integration test with `TEST_DATABASE_URL` set to the disposable database, and include the command and outcome in the summary.

## Test scope and expected coverage

- Standard strategy: target 5–8 behavior tests per U1 component/use-case group, including happy-path behavior and at least two error/edge cases per test file.
- TDD is mandatory: record a failing test before implementation, then make it pass and refactor while green.
- Coverage floor: 80% line coverage per Rust crate. Build and Test evaluates this on the full assembled workspace or a paired crate/service coverage run so service-package tests are included; the unit-scoped command is not a valid coverage measurement.
- Do not weaken or modify coverage thresholds to make a test pass.

## Test design

- Domain tests cover session cap anchoring/expiry, state transitions, ownership-relevant invariants and safe typed errors without framework imports.
- Application tests use hand-written port fakes for repositories, provider admin, clock and audit trail. Assert ordering: provider revocation succeeds before local row mutation; provider failure leaves local state unchanged; duplicate revoke is idempotent; another user's session is not disclosed.
- Where the current contract supports the use case, exercise concurrent/idempotent operations and audit payload sanitization. Do not fabricate a provider MFA or exchange-credential source when no authoritative port exists.
- Postgres integration tests cover migration up/down, constraints and repository behavior against the disposable database. Never run role-creating suites against shared or production Postgres.

## Test data and secrets

Use fixed clocks and synthetic IDs, emails and provider responses. Use dummy token-shaped strings only in redaction tests; never use real credentials, tokens, TOTP seeds or recovery codes. Assert prohibited values are absent from logs, errors, audit records and persisted entities.
