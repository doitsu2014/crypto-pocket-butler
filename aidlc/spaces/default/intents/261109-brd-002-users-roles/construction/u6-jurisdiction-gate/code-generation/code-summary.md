# Code Summary — u6-jurisdiction-gate

**Unit:** U6 `u6-jurisdiction-gate` · **Service:** Identity · **Scope:** classic · **Depth:** Standard
**Stories:** US6.3 (jurisdiction at sign-up and continued use), US6.4 (a restricted sign-up stores nothing)
**Test methodology:** TDD (team posture), Standard volume · **Status:** implemented, gates run

## What was built

The runtime jurisdiction gate is a module inside the existing Identity service. No new
deployable, Postgres role, or message transport was introduced.

| Layer | Delivered |
|---|---|
| Domain (`identity-domain`) | `JurisdictionStatus`, `RestrictionSource`, `Decision`, `JurisdictionListEntry`, `JurisdictionListVersion`, `decide()`, plus the existing `CountryCode` (BR1.1–BR1.6, BR1.10). A missing country is `Restricted`, so the gate fails closed. |
| Application (`identity-application`) | `CheckJurisdiction`, `PublishList`, and the inward ports `GeoLocator`, `ListRepository`, `AuditSink` (`Clock` reused). The provider budget is 800 ms per attempt, one retry, 1.5 s total; the audit budget is 300 ms per attempt, one retry, with a stable decision-id idempotency key. A circuit breaker opens after 5 consecutive provider failures for 30 s. |
| Infrastructure (`identity-infrastructure`) | `PgJurisdictionListRepository` (active-version read with entry-count and checksum verification; transactional publish that supersedes the previous version), `HttpGeoLocator`, `HttpDecisionAudit`. |
| HTTP (`identity-http`) | `POST /v1/internal/jurisdiction/check` and `POST /v1/internal/jurisdiction/publish`, DTOs mapped at the boundary, RFC 7807 problems, `jurisdiction_checks_total` metrics, and a service-token guard that admits only `gateway` and `identity`. |
| Gateway (`gateway-http`) | New `client_ip` middleware: strips inbound `x-forwarded-for` and `x-client-ip`, derives the address from the observed peer (`ConnectInfo`) or the trusted `x-real-ip` edge header, validates it as an IP, and sets `x-client-ip`. The rate limiter now keys on that header. The server attaches the peer address. |
| Data | Migration pair `0003_jurisdiction_list.{up,down}.sql`: `jurisdiction_list_versions` and `jurisdiction_list_rules` (PK `(version, country_code)`, FK to version, SHA-256 checksum, one active version enforced by a partial unique index), granted to the existing `identity_app` role. |
| Deployment | `infra/swarm-stack.yml` gains the two provider secrets on `identity-api` only; no replica or resource change. |

## Decisions and deviations from the approved plan

1. **Table name `jurisdiction_list_rules`, not `jurisdiction_rules`.** The pre-sign-up gate
   (U8 module, already built) owns `jurisdiction_rules` with a different shape (`code` PK,
   `allowed`, `minimum_age`, `disclosures_required`). Reusing the name would break that unit's
   queries or require changing it. U6's versioned rules therefore live in
   `jurisdiction_list_rules`, keeping both concepts in one schema. The plan's structural intent
   (two tables, FK, checksum, PK `(version, country_code)`) is unchanged. The infrastructure
   design's `0004_jurisdiction_list` number is superseded by the approved plan's `0003`.
2. **Internal paths are `/v1/internal/jurisdiction/...`.** Every internal Identity route in this
   repository is `/v1/internal/...` (for example `/v1/internal/users/due`); the plan's shorter
   `/internal/...` spelling would be the only exception and would not match the gateway
   convention. The gateway still never declares these paths, so `/internal/*` stays unreachable
   from the edge.
3. **The new domain types extend `domain/src/jurisdiction.rs`** (the plan's named path), which
   already held the pre-sign-up gate. The module header now names both concerns and their BR ids.
4. **Provider configuration accepts a secret file for both values.** The plan names
   `JURISDICTION_PROVIDER_URL` and `JURISDICTION_PROVIDER_KEY_FILE`; because the plan also asks
   for two Docker secrets, the config reads `JURISDICTION_PROVIDER_URL_FILE` first and falls back
   to the plain variable, and reads `JURISDICTION_PROVIDER_KEY_FILE` (or the plain variable). An
   empty URL leaves the provider unconfigured and every check fails closed with `503`.
5. **Decision cache not implemented.** The performance design's 5-minute decision cache
   (NFR20.3) is not part of the approved plan's steps; the fail-closed tests also require a
   provider failure on continued use to refuse rather than serve a cached allowance. It remains
   an open follow-up.
6. **Publish audit after commit.** A publish commits the new version and then writes its audit
   event. A failed audit returns `503` and is logged, but cannot roll back a committed immutable
   version; this is surfaced rather than swallowed.

## Test-first evidence

| Step | Red command and result | Green command and result |
|---|---|---|
| Domain | `cargo test -p identity --test domain jurisdiction` — `E0432` for `decide`, `JurisdictionListEntry`, `JurisdictionListVersion`, `JurisdictionStatus`, `RestrictionSource` | same command — 9 passed |
| Repository | `cargo test -p identity --test repositories -- --ignored jurisdiction` — `E0432` for `ListRepository` and `PgJurisdictionListRepository` | same command with `TEST_DATABASE_URL` — 4 passed |
| Business logic | `cargo test -p identity --test application jurisdiction` — `E0432` for `CheckJurisdiction`, `PublishEntry`, `PublishList`, `PublishRequest` | same command — 8 passed |
| API | `cargo test -p identity --test http jurisdiction` — 5 failed with `404` (routes absent) | same command — 5 passed |
| Gateway | new tests written with the middleware | `cargo test -p gateway --test http client_ip` — 3 passed |

Every new test file covers the happy path plus at least two error or edge cases. No fixture uses
personal data; IP values are documentation ranges (`192.0.2.0/24`, `198.51.100.0/24`,
`203.0.113.0/24`).

## Gate results

- `cargo test --workspace` — **1377 passed, 91 ignored** (exit 0).
- `cargo fmt --all -- --check` — clean.
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `bash scripts/check-boundaries.sh` — **FAILED on a pre-existing violation unrelated to this
  unit**: `reporting-domain must not import framework crates: ['serde', 'serde_json']`
  (`services/reporting/domain/Cargo.toml` declares both; the file is untouched by this change).
  The Identity and Gateway crates pass their own boundary edges.
- `#[ignore]` adapter suites against a disposable Postgres (`supabase/postgres`, both the pinned
  `15.8.1.060` and the cached `17.11.0.011`) — **all U6 suites pass** (the three new repository
  tests, plus the existing U8 seed test that the `jurisdiction` filter also matches). Across
  the whole workspace, one **pre-existing U1 test** fails on both images:
  `protect_and_unprotect_are_security_definer_functions_the_app_may_execute` at
  `services/identity/tests/repositories.rs:361` ("a destroyed key no longer decrypts the
  subject"). It exercises only pgsodium key destruction, which this unit does not touch; the
  same connection decrypts from pgsodium's key cache after the key row is deleted. It is
  reported, not skipped.
- Coverage (`cargo llvm-cov`, installed per the unit-test instructions; measured with
  `--include-ignored` and the known U1 failure skipped): per-crate line coverage is
  **domain 84.75%**, **application 91.10%**, **http 76.85%**, **infrastructure 29.50%**. The two
  new logic modules exceed the target (`domain/src/jurisdiction.rs` 93.95%,
  `application/src/jurisdiction.rs` 87.66%); the new adapters are `geo_locator.rs` 61.43%,
  `repositories/jurisdiction.rs` 69.05%, and `audit_client.rs` 0% at file level. The
  **80% per-crate target is not met for `identity-http` and `identity-infrastructure`**, and the
  infrastructure crate is dominated by pre-existing adapters with no unit-level coverage
  (`auth.rs`, `service_token.rs`, and the other repositories are at 0%). This is a project-wide
  coverage gap rather than a regression from this unit; closing it needs adapter-level tests
  beyond U6's approved scope.

## Concerns

- The pre-sign-up gate (`jurisdiction_rules`) and the runtime gate (`jurisdiction_list_*`) are
  two country-status sources. They are deliberately separate today; a future unit should decide
  whether one list should drive both.
- IP geolocation remains a best-effort signal (VPNs, corporate egress). The self-declared country
  and the maintained list remain the primary control, as the security design records.
- U1 must pass the gateway-derived `x-client-ip` value in the C8 body; that consumer code is not
  part of this unit.
