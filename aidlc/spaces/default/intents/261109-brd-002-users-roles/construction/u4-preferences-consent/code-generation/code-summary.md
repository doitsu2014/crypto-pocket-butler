# Code Summary — u4-preferences-consent

## What was built

U4 adds the preference store, the append-only consent-event model, the consent-state
gate, the withdrawal with its durable audit outbox, and the paged history to the
existing Identity service. No new deployable and no new container: the outbox drainer
is a `tokio` task started by `identity-api`'s composition root.

### Domain (`services/identity/domain`)

- `consent.rs` gains `ConsentKind` (`terms`, `risk`, `age`, `analytics`, `marketing`;
  `REQUIRED` is `terms`/`risk`/`age`), `ConsentAction`, `ConsentState`,
  `ConsentPolicy` (trimmed, required, ≤120 chars), `ConsentEventDraft`, `ConsentEvent`
  (id, seq, database timestamp), and `ConsentStateView`. `missing()` lists every
  required kind that is not granted, in `REQUIRED` order; `analytics` and `marketing`
  can never appear (BR4.5, BR4.6, BR4.11).
- The preference value types: `BaseCurrencyCode` (the closed USD/EUR/GBP/VND set),
  `parse_iana_timezone` (validated against `chrono-tz`, stored unchanged),
  `PreferenceSource`, `PreferenceValue<T>`, `StoredPreferences` (all fields optional —
  null is "never set"), `PreferenceUpdate`, `ProviderProfile`, and
  `EffectivePreferences::resolve`, which applies
  `COALESCE(preference, provider, literal default)` per field and marks the source
  `user` only for an explicit choice. A provider currency or timezone outside the
  closed set falls back to the literal default rather than being echoed (BR4.1, BR4.2,
  BR4.9).
- `chrono-tz.workspace = true` added to the domain manifest.

### Application (`services/identity/application`)

- `consent/ports.rs`: `ConsentRepository` (newest-per-kind, paged history, insert,
  `begin_withdraw`), `ConsentTransaction` (newest, insert event, enqueue audit,
  commit, rollback), `AuditIntent::consent_withdrawn`, `HistoryCursor`,
  `ObservationSink`, and `DrainerObservation`. The withdrawal transaction is a port so
  the state decision stays in the use case while the event and its outbox row commit
  as one unit (NFR19.1–19.3).
- `consent/state.rs`: `ConsentStateQuery` — one repository read, then the view and
  `missing`; a read failure is `AppError::Unavailable` (503) and is counted
  `outcome="error"` (BR4.6, fail closed).
- `consent/grant.rs`: `ConsentGrant` appends a database-timestamped, versioned
  `granted` event; an insert failure is surfaced, never claimed as success (BR4.8).
- `consent/withdraw.rs`: `ConsentWithdraw` opens the locked transaction, refuses an
  unknown kind with `Conflict` (409), returns a no-op for an already-withdrawn kind,
  inserts the withdrawal carrying the newest grant's `policyVersion` resolved
  server-side, enqueues the audit intent, and commits. An insert failure rolls back
  with `cause="insert_failed"`; an outbox failure rolls back with
  `cause="outbox_failed"` (BR4.7, BR4.8, NFR17.6).
- `consent/history.rs`: `ConsentHistory` pages newest first.
- `preferences/ports.rs`, `read.rs`, `write.rs`: `PreferencesRepository`,
  `UserLookup`, `ReadUserPreferences`, and `WriteUserPreferences`. The write applies
  only supplied fields; an empty update is a read and creates no row; the read/write
  response carries each field's value and source. U4 never writes `users.locale`,
  `users.base_currency`, or `users.timezone` (BR4.1, BR4.2, BR4.9).
- The earlier `consent.rs` and `preferences.rs` files were moved into `consent/mod.rs`
  and `preferences/mod.rs`; their `ConsentService` and `PreferencesService` behaviour
  is unchanged.

### Infrastructure (`services/identity/infrastructure`)

- `consent/repository.rs`: `PgConsentEventRepository` and `PgWithdrawTransaction`.
  The state read is a `DISTINCT ON (kind) … ORDER BY kind, occurred_at DESC, seq DESC`
  inside a transaction with `SET LOCAL statement_timeout = '100ms'`. The withdrawal
  sets `SET LOCAL statement_timeout = '2s'` first, then takes
  `pg_advisory_xact_lock(hashtextextended(user_id || ':' || kind, 0))` before the
  newest read; insert, outbox insert, and commit share the transaction. The outbox
  payload maps field-for-field onto C9's event schema (actor, action, target,
  occurredAt, outcome, fixed context, sensitivity `security`).
- `consent/outbox.rs`: `AuditOutboxDrainer` selects due rows under
  `FOR UPDATE SKIP LOCKED`, posts each with its `Idempotency-Key`, marks
  delivered/retryable/dead (three attempts, bounded exponential backoff with jitter, a
  4xx is permanently dead), refuses a token- or wallet-shaped payload without sending
  it, deletes delivered rows after 24 hours, and emits the backlog and delivery
  observation. `run_forever` is the `identity-api` task.
- `preferences/repository.rs`: `PgPreferencesRepository` (load; one
  `INSERT … ON CONFLICT (user_id) DO UPDATE` using `COALESCE(EXCLUDED.x, existing.x)`
  so only supplied columns change) and `UserLookup` for `PgUserRepository`.
- `infrastructure/src/lib.rs` exports the two new modules.

### Migration `0005_preferences_consent`

Up: creates `identity_retention` (LOGIN, no password — its credential comes from
Vault); adds `kind`, `action`, `occurred_at`, `policy_version`, and nullable `seq`;
creates `consent_records_seq_seq`; drops the legacy NOT NULLs/CHECKs; backfills the
event shape in 10 000-row batches (one-to-one
`analytics→analytics`, `marketing→marketing`, `disclosures→terms`;
`granted=false→withdrawn`; `recorded_at→occurred_at`;
`source→'legacy:<source>'`); backfills `seq` in batches, attaches the sequence default,
and sets it NOT NULL; adds a shape CHECK plus the kind/action/policy-version CHECKs;
replaces the old index with the state and history indexes; creates `user_preferences`,
`consent_audit_outbox`, and `consent_records_legacy_archive`; revokes UPDATE/DELETE
from `identity_app`; grants the erasure role `SELECT (user_id)` + `DELETE` on the
three tables; and installs a `BEFORE UPDATE OR DELETE` trigger that raises for
`identity_app` and permits the erasure and migration roles. Every step is idempotent.
The new columns stay nullable through this release so a previous replica keeps writing
during the `start-first` overlap (expand now, contract later).

Down: creates the archive with `IF NOT EXISTS`; while the event columns exist, copies
`risk`/`age` events into it and collapses each `(user, kind)` to its newest mapable
event with the inverse mapping in one data-modifying statement; removes the trigger;
restores the legacy NOT NULLs/defaults and CHECKs; drops the new columns, sequence, and
indexes; restores `consent_records_latest_idx` and the application's
`SELECT, INSERT, DELETE` grant; drops the two new tables; leaves the archive in place.
A second run skips the event-shape block and re-executes only the idempotent DDL.

### HTTP (`services/identity/http`)

- `GET|PUT /v1/me/preferences`, `GET /v1/me/consent/state`, `POST /v1/me/consent`,
  `POST /v1/me/consent/{kind}/withdraw`, and `GET /v1/me/consent`, all JWT-authenticated;
  the user id always comes from the token.
- `GET /internal/users/{userId}/consent/state` (C6) requires the `x-service-token`
  allowlist (`account-api` by default) and the forwarded JWT; another user's id answers
  404, indistinguishable from absent (BR4.3).
- DTOs validate closed sets at the boundary and return 422 naming the field. The
  history uses an opaque `occurredAt|seq` keyset cursor (`Z`-suffixed, microsecond
  precision so it survives a query string); the page size defaults to 50 and caps at
  100.
- `consent_*` metrics — state queries by outcome, queries with a missing acceptance,
  a state-query duration histogram, events by kind/action, preference writes by
  outcome, withdrawal rollbacks by cause, outbox pending/oldest-age/last-pass gauges,
  and deliveries by outcome — plus `route` labels for the new paths.

### Configuration and deployment

- `ApiConfig` gains `IDENTITY_CONSENT_SERVICES` (default `account-api`) and
  `IDENTITY_DRAINER_INTERVAL_SECONDS` (default 5). No secret is added: the drainer
  reuses the Identity service's existing C9 token from the service-token file, and
  `identity_retention`'s credential is materialised from Supabase Vault by the platform
  role bootstrap.
- `build_api_state` now returns `ApiRuntime { state, drainer }`; `run` spawns
  `drainer.run_forever()` before serving and aborts it after shutdown. No
  `infra/swarm-stack.yml` change.

## How to run

```bash
cargo test -p identity --test consent_domain
cargo test -p identity --test consent_application
cargo test -p identity --test consent_http
TEST_DATABASE_URL=postgres://… cargo test -p identity --test repositories -- --ignored consent
cargo test -p identity
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

## Red/Green evidence (this build)

TDD ordering: each layer's tests were written before its implementation, and the
failing run is recorded below.

| Step | Red | Green |
|---|---|---|
| Baseline (before any edit) | — | `cargo test -p identity` → 170 passed, 17 ignored |
| Runner (step 2) | — | `cargo test -p identity --test consent_domain -- --list` → exit 0, 0 tests |
| Data model (steps 3–5) | `cargo test -p identity --test consent_domain` → `error[E0432]`: unresolved `parse_iana_timezone`, `BaseCurrencyCode`, `ConsentAction`, `ConsentEvent`, `ConsentKind`, `ConsentPolicy`, `ConsentState`, `ConsentStateView`, `EffectivePreferences`, `PreferenceSource`, `PreferenceUpdate`, `ProviderProfile`, `StoredPreferences` | `cargo test -p identity --test consent_domain` → 10 passed |
| Repository + migration (steps 6–8) | `cargo test -p identity --test repositories --no-run` → `error[E0432]`: unresolved `AuditOutboxDrainer`, `PgConsentEventRepository`, `PgPreferencesRepository`; `E0599` no `newest`/`insert_event`/`enqueue_audit`/`commit`/`rollback` | `TEST_DATABASE_URL=… cargo test -p identity --test repositories -- --ignored consent` → 9 passed, 19 filtered (8 new tests + the existing `consent_latest…`); the first run was 6 passed / 3 failed on a missing sequence `USAGE` grant and a test fixture with a dangling FK, both fixed |
| Business logic (steps 9–11) | `cargo test -p identity --test consent_application --no-run` → `error[E0432]`: unresolved `ConsentGrant`, `ConsentHistory`, `ConsentStateQuery`, `ConsentWithdraw`, `WithdrawResult`, `ReadUserPreferences`, `WriteUserPreferences` | `cargo test -p identity --test consent_application` → 14 passed |
| API (steps 12–14) | **Gap, stated honestly:** the HTTP tests were written before the handlers and routes, but the failing run was not captured separately; the first run after implementation surfaced one genuine failure — the history cursor used `+00:00`, which a query string decodes to a space, so page 2 got 422. Fixed to a `Z`-suffixed microsecond cursor. | `cargo test -p identity --test consent_http` → 12 passed |
| Whole suite | — | `cargo test -p identity` → 206 passed, 25 ignored; `cargo test --workspace` → 1486 passed, 104 ignored |

`cargo fmt --all -- --check` → clean. `cargo clippy --workspace --all-targets -- -D warnings`
→ clean. `bash scripts/check-boundaries.sh` → **FAILED**, only on the pre-existing
`reporting-domain` → `serde`/`serde_json` edge, which is not this unit's.

## Review fixes (the round after the first review)

The person chose to wire the erasure path in this unit rather than defer it. All
six majors and the three minors are fixed.

| Finding | Fix | Evidence |
|---|---|---|
| 1. The deletion worker ran as `identity_app` and its consent delete now fails | New `ErasureStore` port; `PgErasureStore` on a pool built from `IDENTITY_ERASURE_DATABASE_URL`; `DeletionWorker` routes its consent/preferences/archive delete through it; worker config, swarm secret and local compose/bootstrap wiring added (secret value is the platform's) | New adapter test `consent_erasure_store_deletes_as_the_erasure_role_and_is_refused_as_identity_app` (10 passed in the U4 ignored suite); `tests/worker.rs` and `tests/application.rs` assert the delete reaches the erasure port |
| 2. The up migration held `ACCESS EXCLUSIVE` across the backfills, `SET NOT NULL`, CHECKs and non-concurrent indexes | `migrations.rs` now applies `0005` **statement by statement outside any transaction** (sqlx's `no_tx` alone is not enough — its simple-query batch is one implicit transaction, which refuses `CONCURRENTLY`); the backfills are procedures that `COMMIT` every 10 000 rows; `seq NOT NULL` uses a validated CHECK; the CHECKs are `NOT VALID` then `VALIDATE`; the indexes are `CONCURRENTLY`; the header documents the order and the residual lock window | `consent_migration_maps_legacy_rows_and_the_down_collapses_and_archives` runs the real runner; the parallel adapter suite no longer deadlocks (the test harness creates the cluster roles under a short lock, then migrates unlocked) |
| 3. The drainer held a transaction across up to 20 C9 posts | `claim()` takes the batch with `FOR UPDATE SKIP LOCKED` in a short transaction, increments `attempts`, sets a 30 s lease in `next_attempt_at`, and commits; each post runs outside any transaction and each outcome is its own short update | Existing drainer test still asserts delivered/retry/dead and the idempotency header; a slow post now blocks no other writer |
| 4. The legacy `POST /v1/consent` wrote rows the gate could not see | The route is retired with **410 Gone**; the gate's `newest_per_kind`/`history` SQL now `COALESCE`s `kind`/`action`/`occurred_at`/`policy_version` from the legacy columns, so a row written by a previous replica during the overlap still appears | `tests/http.rs` now asserts 410 and an empty purpose history; the repository tests cover the event shape and the mapping |
| 5. The down migration collapsed and lost history | It archives **every** event into `consent_records_legacy_archive` (all five kinds) before the collapse; the archive CHECK admits all five | The migration test counts every event before the down and asserts the archive holds all of them, including analytics/marketing/terms |
| 6. C6's declared routes were missing | Implemented `/internal/users/{id}/preferences` (GET/PUT), `/internal/users/{id}/consent` (GET history, POST grant or withdraw) and kept `/consent/state`, all through the shared allowlist + token-user check | New `the_c6_declared_preferences_and_consent_routes_work` test (200/201/403/404) |
| Minor: zero drainer interval panics | `IDENTITY_DRAINER_INTERVAL_SECONDS = 0` is refused at config load with a named error | Config change; no test (the reader requires a process env) |
| Minor: the 200 no-op and 409 paths were counted as withdrawal failures | `roll_back_quietly` rolls back without the `consent_withdrawal_rollbacks` counter; only a store/insert/commit failure is counted | `consent_application` now asserts the observation is empty for both paths |
| Minor: document the LOGIN decision | The `0005` header names `0001`'s rule and states why this role is the deliberate exception | Migration header |

Honest evidence note: the new tests for the erasure adapter, the C6 routes and the
archive-all down were written with their implementations, and no separate
pre-implementation Red run was captured for them. The outbox claim rework and the
migration rework were exercised by the existing tests, which failed and then passed
as the code changed (the migration deadlock above was found this way). The first
review round's Red/Green table above remains the record for the original build.

The migration runner also lifts the pool's 10 s `statement_timeout` on its dedicated
connection (a backfill batch or a concurrent index build may exceed it), and
`split_statements` has two unit tests covering comments, quoted semicolons, and
`$$`/`$tag$` bodies.

## Second review fixes

| Finding | Fix | Evidence |
|---|---|---|
| R-04 — the `seq` window | The sequence default is set **before** the backfill; the `seq` backfill runs a second pass just before `SET NOT NULL`; the `seq IS NOT NULL` CHECK is added only after the default exists, so a late legacy insert cannot abort `VALIDATE` | `consent_migration_gives_a_late_legacy_row_a_seq_before_the_constraint` drives 0005 statement by statement, inserts a legacy-shaped row after the first `seq` backfill, and asserts it gets a `seq`, survives `SET NOT NULL`, and is still visible to the gate |
| R-06 — gateway routes | The six U4 `/v1` routes are declared in `services/gateway/domain/src/route.rs` as identity routes, none requiring the admin role | Gateway domain test `the_me_preferences_and_consent_routes_are_identity_and_not_admin` and HTTP test `a_u4_me_route_is_proxied_to_identity` |
| R-07 — retire the legacy PATCH | `PATCH /v1/users/{userId}/preferences` is repointed to the `user_preferences` write through the U4 use case; it no longer writes `users.*`, and a write through it reads back with source `user`, not `default`. The legacy flat response shape is preserved for existing callers | `a_preference_change_with_a_bad_currency_is_a_field_level_422` still passes; the write path now goes through `WriteUserPreferences` |
| R-09 — role name | `identity_erasure` is renamed to `identity_retention` in the migration, config (`IDENTITY_RETENTION_DATABASE_URL`), local init script, compose, stack and tests; the LOGIN decision stays documented as the deliberate `0001` exception with the Vault bootstrap step | All role references renamed; the adapter is now `PgRetentionStore` |
| R-11 — C9 path | The open-contract item now states the drainer posts to `/v1/events` because that is what the shipped audit service serves, so the C9 amendment covers the path as well as the idempotency header and completion wording | `code-summary.md` open-contract item |
| R-12 — coverage | The summary names the owner and moment: the integration/release owner runs `cargo llvm-cov` in the environment where `llvm-tools-preview` is installed, before merge | Coverage section |
| New — local provision path | `deployment/localhost/scripts/provision.sh` special-cases 0005 to run it without `--single-transaction` (statement by statement), matching the migrate runner | Verified against the disposable PostgreSQL 17: 0001–0004 under `--single-transaction`, then 0005 statement-wise; both concurrent indexes built and the three tables exist |
| New — the down lost legacy-shaped rows | The down archives every row (event-shaped and legacy-shaped) with the same mapping, keyed by id with `ON CONFLICT DO NOTHING`; the collapse also maps legacy-shaped rows | The migration test inserts a late legacy-shaped row and asserts `deleted count == archived count` and that its id is archived |
| New — `migrator_before` filtered `!= 5` | It now filters `< 5`; `run` applies `< 5`, then 0005, then a `> 5` subset, so a future `0006` can never run before 0005 on a fresh database; `revert` now undoes the newest applied version | Migration-runner change; all migration tests pass |

## Coverage (honest report)

`cargo llvm-cov -p identity-domain -p identity-application -p identity-infrastructure
-p identity-http -p identity --summary-only` **could not run**:

```
error: failed to find llvm-tools-preview, please install llvm-tools-preview, or set
LLVM_COV and LLVM_PROFDATA environment variables
```

`cargo-llvm-cov 0.9.1` is installed; its `llvm-tools-preview` rustup component is not,
and the plan does not name a toolchain install. The 80% per-crate line target for this
unit's lines is **unverified**, not met-by-claim. To run it:

```bash
rustup component add llvm-tools-preview
cargo llvm-cov -p identity-domain -p identity-application -p identity-infrastructure -p identity-http -p identity
```

The non-ignored suites cover the domain value types, every application decision
(including both rollback causes), the HTTP statuses, and the metrics/DTO mapping; the
adapters, the migration, and the drainer gain theirs from the `#[ignore]` Postgres
suite, which was run here against a disposable database.

## Open contract changes (recorded, not assumed)

- **C6 (owned by contract-design):** must gain the consent-state query
  (`GET /internal/users/{id}/consent/state`), the consent-record event schema
  (`id`, `seq`, `kind`, `action`, `occurredAt`, `policyVersion`), the 404/409/422/503
  statuses this unit uses, and `account-api` as a consumer beside U7. Until it is
  written into `contract-summary.md`, C6 and the persisted shape disagree; the
  `components.md` `ConsentRecord` description (`grantedAt`/`withdrawnAt`) is also stale.
- **C9 (owned by the audit service):** the drainer posts to **`/v1/events`**, because
  that is the path the shipped audit service actually serves. The amendment must cover
  that path as well as two rules: `/v1/events` must honour an `Idempotency-Key` header,
  and the "complete only once the audit write is accepted" rule is amended by the outbox
  model to "complete once the audit intent is durably recorded locally, delivered
  asynchronously with a bounded lag".
- **Gateway:** the SPA routes `/v1/me/preferences` and `/v1/me/consent…` are now declared
  in the gateway's deny-by-default route table (R-06). The older `/v1/consent` and
  `/v1/users/{id}/preferences` legacy paths remain as before; `/v1/consent` is retired
  with 410 on the Identity side.

## Known gaps / issues

- **Coverage is unverified here, not waived.** `cargo-llvm-cov 0.9.1` is installed but
  its `llvm-tools-preview` component is not, and installing it is outside this build's
  named steps. **Owner and moment:** the integration/release owner runs
  `cargo llvm-cov -p identity-domain -p identity-application -p identity-infrastructure
  -p identity-http -p identity` in the CI/release environment where the component is
  available, **before merge**, and reports the per-crate line result against the 80%
  floor. Until that run, the floor is unverified rather than met.
- **`identity_retention` is created `LOGIN` with no password**, as the plan settles. The
  migration mints no password; production materialises it from Supabase Vault
  (`identity_retention_database_url`), and the local stack gives it the local developer
  password in `deployment/localhost/init/01-roles.sql`. This is the deliberate exception
  to `0001`'s "a migration must not mint a login" rule, stated in the migration header.
- **The erasure path now needs the second DSN in every environment.** The swarm stack
  declares `identity_retention_database_url` as an external secret and the worker reads
  `IDENTITY_RETENTION_DATABASE_URL_FILE`; the local compose points it at the
  `identity_retention` login. The platform must create that secret from Vault before the
  release, or the worker will refuse to start (the config reader requires it).
- **`consent_records_legacy_archive` is created by the up (empty) and written by the
  down.** The down archives every row (event-shaped and legacy-shaped) idempotently
  (`ON CONFLICT (id) DO NOTHING`), and a second rollback skips the event-shape block
  because the columns are gone.
- **The old C3 `getConsent` surface reads legacy rows only.** Events written by the new
  code leave `purpose` NULL, so `PgConsentRepository::latest` does not see them. This
  is the intended expand/contract split — the old read is re-pointed in a later
  unit — but it means the C3 analytics/marketing answer is stale for new events until
  then. The **gate** does see them: `newest_per_kind`/`history` map the legacy columns.
- **The legacy `PATCH /v1/users/{userId}/preferences` is repointed, not removed.** It now
  writes `user_preferences` through the U4 use case and returns the legacy flat shape, so
  a write through it is read back with source `user`, not `default`. The old
  `application::PreferencesService` (which wrote `users.*`) is no longer routed and
  should be deleted in the cleanup round together with the flat response.
- **The withdrawal's `statement_timeout` is set before the advisory lock**, not after as
  the design's snippet shows, so the timeout bounds the lock wait as the reliability
  design intends ("lock wait + work").
- **The migration's residual write window.** The backfills and concurrent index builds no
  longer block, but each `ALTER TABLE` takes a brief ACCESS EXCLUSIVE lock and each
  `VALIDATE CONSTRAINT` holds SHARE UPDATE EXCLUSIVE for its table scan. On a large table
  that scan is the longest write-blocking step; the migration header states it.
- **The adapter and migration suites need `TEST_DATABASE_URL`** with `pgsodium` and a
  role that can create roles; they ran here against a disposable PostgreSQL 17 instance.
  The full ignored repository suite is green except the pre-existing
  `protect_and_unprotect_are_security_definer_functions_the_app_may_execute` test, which
  fails on pgsodium key-cache invalidation after a key destroy — it touches nothing U4
  changed and was not run in this session's baseline (`--ignored consent` filtered it).

