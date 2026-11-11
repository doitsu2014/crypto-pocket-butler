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

`cargo llvm-cov` over the identity crates reports 58.39% overall (64.53% lines).
The target is 80% per
crate. The shortfall is dominated by the adapter and binary crates, whose
coverage only accrues when the `#[ignore]` suites run against Postgres; the
domain/application/http paths added here are covered by the non-ignored suites.
The frontends are untouched and were already below no configured TypeScript
threshold.

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
  regression. Worth confirming on the production Postgres.
- The public principal route requires a self-declared country (`x-country-code`)
  and the observed client address (`x-client-ip`); the sign-up UI (U8) must send
  them. Until then a first authentication without them is a 422 by design
  (fail closed).
