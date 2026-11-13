# Code Summary — u1-user-registry

## What was built

U1 extends the existing Identity service (no new deployable, no new Postgres
role) with the user registry the other identity units read through C3 or
in-process.

### Domain (`services/identity/domain`)

- `UserStatus` now carries the four public statuses (`active`, `disabled`,
  `pending_deletion`, `deleted`) plus the transient `pending`, with
  `denies_use()` as the single denial predicate (BR1.7, BR1.10).
- `User` gains `display_name`, `display_name_overridden`, and
  `email_overridden`; `update_profile` validates and marks an override, and
  `apply_claims` never overwrites an overridden field (BR1.4). An overridden
  address is not provider-verified.
- `User::provision` starts `active` when the provider's facts support it
  (`can_activate`, which keeps the exchange-credential/MFA rule) and `pending`
  otherwise.
- `VerifiedClaims` gains `display_name` plus the `country_code` and `client_ip`
  the trusted caller supplies for the jurisdiction gate.

### Application (`services/identity/application`)

- `PrincipalService` calls the new in-process `JurisdictionGate` port before a
  first provision; a denial or malformed declaration is a 422, an unavailable
  gate a retryable 503, and nothing is written. A subject-hash collision is a
  validation refusal with an audit event, never a returned record (fail closed).
- `Disabled`, `pending_deletion`, and `deleted` accounts are denied and audited
  on resolution; the guarded `pending` → `active` promotion remains the interim
  path until the account-lifecycle unit owns transitions. No
  `identity.role.changed` event is emitted; authorization now reads the role
  from the validated token.
- `resolve_with_origin` reports whether the request created the record, so the
  C3 PUT answers 201 on first provision and 200 on refresh.
- New `user_registry::AdminLookup`: exact-email admin search (admin only) and
  the owner-or-admin record read, both audited and both confined to the
  metadata boundary (BR1.9). `subject_hash` is the shared SHA-256 helper.
- `CheckJurisdiction` implements `JurisdictionGate`, so the C8 endpoint and the
  registry's precondition share one implementation.

### Infrastructure (`services/identity/infrastructure`)

- `PgUserRepository::provision` creates the pgsodium key, the shredding-key
  reference, the user row, and the protected subject in one transaction; a
  failure leaves no partial record.
- `find_by_auth_subject` decrypts the stored subject and fails closed on a hash
  collision; `find_by_subject_hash` and exact `find_by_email` support C3.
- Migration `0004_user_registry_extensions.{up,down}.sql`: widened status check,
  display name and override markers, partial `users(email)` index. The down
  migration maps `disabled`/`pending_deletion` back to `active` before
  re-imposing the narrower check so a rollback cannot fail on live rows.
- `migrations::revert` now reverts the newest migration only (it previously
  reverted to version 0); this is what the down-migration test exercises.

### HTTP (`services/identity/http`)

- `GET /v1/users/{userId}` (owner or admin) and
  `GET /v1/admin/users?email=` (admin), both JWT-authenticated.
- C3: `GET|PUT /v1/internal/users/by-subject/{subjectHash}`,
  `GET /v1/internal/users/{userId}`, `GET /v1/internal/users?email=`.
  Reads allow the gateway and the read-only `identity-smoke` token; the PUT
  allows only the gateway. The PUT checks the path hash against the body
  subject.
- The JWT middleware fills the gate declaration from `x-country-code` and
  `x-client-ip` (the gateway/edge headers), never from the token.
- New `identity_principal_status_total{status}` metric.

### Gateway

- Declared `GET /v1/principals/{authUserId}`, `GET /v1/users/{userId}`, and
  admin-gated `GET /v1/admin/users`, owned by a new `identity` service name.
- The upstream client now dispatches by route service name to a configured URL
  map (`ASSET_REGISTRY_URL`, `IDENTITY_URL`); an unknown service is refused.

### Deployment

- `identity-migrate` binary, `Dockerfile.migrate`, a one-shot Swarm
  `identity-migrate` service, the `identity_migrate_database_url` secret, and
  the `identity_smoke_token` secret (mapped to `identity-smoke` in the service
  token registry).
- `IDENTITY_URL` added to the gateway configuration (local compose and
  `.env.example`).

## Loop-back repair (Build and Test, 2026-11-12)

Build and Test reopened this Unit for a stale seam test (F2a) and crate coverage (F6).
Steps 1–18 are unchanged; the repair is steps 19–22 of the plan.

### F2a — the seam test now matches the erasure guard

`a_user_is_found_by_the_hashed_subject_and_the_subject_is_protected_at_rest` stored an
**active** user and then called `anonymise`, which U5's erasure guard deliberately refuses
(`WHERE status IN ('pending_deletion','deleted')`). The guard is the intended contract and
was not changed. The test now:

- asserts the refusal deliberately in a new case,
  `the_erasure_guard_refuses_an_account_that_is_not_pending_deletion` — a `Conflict`, the
  record still resolves, and the protected subject still reveals;
- drives the stored user to `pending_deletion` through `User::transition` plus
  `UserRepository::update` before erasing, so the at-rest assertions run on a leg the guard
  permits, and their substance is unchanged;
- asserts the tombstone contract instead of a stale "the subject is gone": after erasure
  `find_by_auth_subject` still answers the pseudonymous hash (BR5.8) with the `deleted`
  record and no personal fields, and `reveal_auth_subject` returns `None`.

Red: the suite failed at `users.anonymise(&user)` with
`Conflict("the account is no longer pending deletion; the erasure was refused")` — the F2a
evidence from Build and Test, reproduced locally. Green:
`cargo test -p identity --test repositories -- --ignored --test-threads=1 a_user_is_found the_erasure_guard_refuses`
— 2 passed, 32 filtered out.

### F6 — adapter coverage

New `#[ignore]` cases in `services/identity/tests/repositories.rs`, all against the
disposable Postgres image:

- `user_registry_reveal_auth_subject_is_none_for_an_unknown_user`;
- `shredding_keys_active_for_user_returns_the_live_reference_and_destroy_refuses_an_unknown_key`
  (none → create → active → destroy an unknown key is a `Conflict` → the live key is still
  active);
- `user_registry_lists_only_active_users_for_the_due_sweep` (two active, one pending;
  limit/offset paging; an offset past the end is empty);
- `user_registry_a_second_insert_of_the_same_user_conflicts_and_leaves_one_record`;
- `user_registry_queries_report_an_unavailable_store` (the schema is dropped underneath the
  pool, so every read and write maps to `RepositoryError::Unavailable`).

The collision case was extended: the retry is still refused, `SELECT count(*) FROM users`
is 1, and the stored status is `active` — the fail-closed path creates nothing.

These cases were written against the existing adapters, which did not change; their "Red" is
the F6 shortfall itself (the branches were unexecuted). Green:
`TEST_DATABASE_URL=… cargo test -p identity --test repositories -- --ignored --test-threads=1`
— 35 passed, 1 failed (only the out-of-scope F2b case), 3 filtered out.

### F6 — composition-root coverage

`services/identity/src/` was 0.00 % because nothing could reach it without a database, a
listener and a signal. It now has testable entry points:

- `ApiConfig::from_lookup` / `WorkerConfig::from_lookup` take an injected environment
  lookup; `from_env` reads the process environment. `#[cfg(test)]` cases in `config.rs`
  cover the happy paths and defaults, the zero drainer interval, a bad port, a missing
  required variable, a missing retention DSN, CSV trimming, secret files (trimmed, empty,
  plain fallback, absent), `file_or` and `optional_secret`.
- `lib.rs` re-exports `identity_http::routes::router`, so router and middleware assembly is
  reachable from the crate; `write_heartbeat` takes its path, so the write and the failure
  branch are exercisable.
- `run_worker`'s wiring moved into `pub async fn build_worker_runtime`, leaving `run_worker`
  to build, bind and serve.
- New `services/identity/tests/composition.rs`: the probes and security headers, a failed
  readiness probe as a `problem+json` 503, the fallback plus its metric, the heartbeat, and
  — `#[ignore]`d — `build_api_state` (health 200, readiness 503 on an un-migrated schema),
  its two registry refusals, and `build_worker_runtime` (port carried, worker readiness
  not-ready on an un-migrated schema).

## How to run

```bash
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
bash scripts/check-boundaries.sh

# Disposable Postgres (pgsodium required by the adapter suites):
TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored
DATABASE_URL='postgres://…?options=-csearch_path%3Didentity' cargo run -p identity --bin identity-migrate
```

Composition root (the loop-back repair):

```bash
cargo test -p identity --lib                       # configuration parsing
cargo test -p identity --test composition          # router, probes, heartbeat
TEST_DATABASE_URL=postgres://… cargo test -p identity --test composition -- --ignored
```

Unit-scoped commands: `cargo test -p identity --test domain user_registry`,
`cargo test -p identity --test application user_registry`,
`cargo test -p identity --test http user_registry`.

## Red/Green evidence (this build)

| Layer | Red | Green |
|---|---|---|
| Data model | `cargo test -p identity --test domain user_registry` failed to compile: `UserStatus::Disabled/PendingDeletion` missing, `update_profile`/`display_name`/`email_overridden` missing, `VerifiedClaims.display_name` missing | `cargo test -p identity --test domain` — 41 passed |
| Repository | the new `#[ignore]` tests referenced `provision`, `find_by_email`, `RepositoryError::Validation`, and the `0004` pair before they existed (crate compile failure); the first run against the disposable Postgres then failed on the un-migrated status check | `TEST_DATABASE_URL=… cargo test -p identity --test repositories -- --ignored` — 15 passed, 1 pre-existing environmental failure (see below) |
| Business logic | the application tests were written for the gate, status denial, collision, and admin lookup; the initial crate check failed on the missing `Validation` mapping and wiring | `cargo test -p identity --test application` — 38 passed |
| API | `cargo test -p identity --test http user_registry` — 5 failed with the route `404`/gate `403` before the handlers and routes existed | `cargo test -p identity --test http` — 39 passed |

## Coverage (honest report)

Measured after the loop-back repair with
`TEST_DATABASE_URL=… cargo llvm-cov -p identity -p identity-infrastructure --ignore-run-fail --json --output-path target/coverage-loopback.json -- --include-ignored --test-threads=1`.
Line coverage per crate, before and after:

| Crate | Before (Build and Test) | After the repair |
|---|---|---|
| `identity-infrastructure` | 1 417 / 2 642 = 53.63 % | 1 716 / 2 642 = **64.95 %** |
| `identity` (composition root) | 0 / 535 = 0.00 % | 623 / 703 = **88.62 %** |

The composition root now clears the 80 % floor; its 67 remaining uncovered lines are the
body of `run` after state construction, the listener/service tail of `run_worker`, and the
supervisor's shutdown-signal wait — all of which need a real listener and a signal.

`identity-infrastructure` does not clear the floor, and this repair does not lower it. Its
926 uncovered lines concentrate in adapters this Unit did not add, whose cases belong to
the units that own them (support adapters, lifecycle transitions, JWT validation,
contributors, deletions, storage):

| File | Uncovered lines |
|---|---|
| `infrastructure/src/auth.rs` | 134 |
| `infrastructure/src/support/drainer.rs` | 90 |
| `infrastructure/src/support/sources.rs` | 89 |
| `infrastructure/src/repositories/status_transitions.rs` | 82 |
| `infrastructure/src/contributors.rs` | 54 |
| `infrastructure/src/support/observation.rs` | 42 |
| `infrastructure/src/repositories/deletions.rs` | 42 |

u1's own `repositories/users.rs` and `repositories/keys.rs` error and retry branches are now
exercised. The second-pass support adapter cases now exercise the drainer delivery path and the
Account/Portfolio source success, non-2xx, malformed UUID, and missing-field projections through a
local Axum stub; they assert requests, delivery observations, and mapped errors. The frontends are
untouched and have no configured TypeScript threshold.

### Second-pass evidence (Steps 23–26, 2026-11-13)

The plan's original command was rejected by the installed `cargo-llvm-cov` because
`--include-ignored` is not a cargo-llvm-cov option. The accepted equivalent was run as:
`TEST_DATABASE_URL=postgres://… cargo llvm-cov -p identity-infrastructure -p identity --ignore-run-fail --json --output-path target/coverage.json -- --include-ignored --test-threads=1`.
The run completed its report generation with the known pgsodium assertion failure; the failure is
out of scope and was not altered. The support adapter integration tests were then run explicitly:
`TEST_DATABASE_URL=postgres://… cargo test -p identity --test support_adapters -- --ignored --test-threads=1 support_drainer_delivers_and_records_observation support_sources_cover_success_and_error_projection_paths`
— 2 passed, 0 failed. The complete ignored support-adapter suite was also verified by the build-and-test
run (27 passed, with only the documented pgsodium assertion excluded from the passing baseline).

The second pass adds no behavior changes. Remaining coverage gaps are the inaccessible external
failure/timing arms in JWKS refresh, geo-provider timeout, and service-level adapters not owned by
U1; these require live external systems or are owned by later units. The pgsodium key-destruction
assertion remains intentionally unchanged.

## Known gaps / issues

- `bash scripts/check-boundaries.sh` fails on a pre-existing violation that this
  unit did not introduce: `reporting-domain` depends on `serde` and
  `serde_json`. Confirmed against an unmodified `services/reporting` tree.
- `yarn format:check` reports pre-existing Prettier differences in 60 `web`
  files and 27 `support-console` files; `yarn test`, `yarn lint`, and
  `yarn build` pass in both packages. No frontend file was changed here.
- The adapter test `protect_and_unprotect_are_security_definer_functions_the_app_may_execute`
  fails on the disposable Supabase Postgres image: pgsodium keeps the key
  derivable after `destroy_shredding_key` deletes its row, so the "destroyed
  key no longer decrypts" assertion does not hold in that image. The same
  sequence repros outside the repository, so this is environmental, not a
  regression. Worth confirming on the production Postgres. **The loop-back
  repair left this test untouched (F2b is out of scope); it is the only failure
  in the ignored adapter suite.**
- The public principal route requires a self-declared country (`x-country-code`)
  and the observed client address (`x-client-ip`); the sign-up UI (U8) must send
  them. Until then a first authentication without them is a 422 by design
  (fail closed).
