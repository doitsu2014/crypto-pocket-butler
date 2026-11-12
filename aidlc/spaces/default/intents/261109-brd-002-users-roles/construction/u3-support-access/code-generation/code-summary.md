# Code Summary — u3-support-access

**Unit:** U3 Support Access, embedded in the Identity service
**Plan:** `code-generation-plan.md`
**Testing contract:** `sha256:3f8863a9358b0b5e19a34ad2fc282a634f0d5323fb50ae1f7429a92a216766c3`
(`tdd`, team source, classic scope, standard strategy, brownfield)

## What this unit builds

A support agent may request a time-bound grant on one user's account and portfolio metadata, a
different admin must approve it, the grant lasts exactly 60 minutes from the approval, and every
request, approval, revocation, served read and refusal is audited. The metadata read fans out to
the two owner services over new internal endpoints gated by the Identity service token, and the
grant's audit intent rides a Postgres outbox drained inside `identity-api`.

| Layer | Where |
|---|---|
| Domain rules | `services/identity/domain/src/support.rs` |
| Use cases, fan-out, audit decisions | `services/identity/application/src/support.rs`, `ports.rs` |
| Postgres adapter, outbox, drainer, backlog | `services/identity/infrastructure/src/repositories/support.rs`, `infrastructure/src/support/{mod,outbox,backlog,drainer,observation,sources}.rs` |
| Schema | `services/identity/migrations/0007_support_access.{up,down}.sql` |
| Routes, handlers, DTOs, problem codes | `services/identity/http/src/{routes,handlers,dto,problem,middleware,metrics}.rs` |
| Composition root | `services/identity/src/lib.rs` |
| Gateway routes and per-token budgets | `services/gateway/domain/src/{route,path}.rs`, `application/src/authorize.rs`, `http/src/{rate_limit,middleware}.rs` |
| Owner internal endpoints | `services/account/{domain,application,http}`, `services/portfolio/{domain,http}` |
| Service-token seeds | `infra/sql/account-service-tokens.sql` (new), `infra/sql/portfolio-service-tokens.sql` |
| Alerts | `infra/prometheus/support-rules.yml` (new) |
| Deployment | `infra/swarm-stack.yml` |
| Architecture docs | `docs/architecture/workspace.dsl`, `docs/architecture/README.md` |

## Design decisions

- **Audit sink split.** `SupportService::new` keeps its arity: the application `SupportObservation`
  port gained a default-provided `source_failure` hook, the infrastructure `AuditSupportObservation`
  forwards it to a new `SupportSourceFailureSink` port, and the drainer consumes a new
  `SupportDrainObservation` port. `Metrics` implements both sinks; only `src/lib.rs` (the
  composition root) wires them, so `infrastructure` never depends on `http`.
- **Read-time decisions.** Grant expiry and revocation are decided at each request from the
  persisted window; the persisted expiry is still written so the row and the audit agree.
- **Closed projection.** The owner endpoints answer only the allowlisted fields, and a missing
  target answers an empty projection (200) so the read is not an existence oracle.
- **Fail-closed audit.** A served read or a refusal whose audit append fails is answered 503
  `audit-unavailable`, never served unrecorded. A grant mutation writes its audit row in the same
  transaction; the `support_audit_outbox` drainer delivers it to C10 with the row id as the
  idempotency key, retries transient failures, dead-letters after three attempts, and deletes
  delivered rows after 24 hours.
- **Per-token route budgets.** `Route.max_requests_per_minute` declares 20/min for grant creation
  and 60/min for the metadata read; the limiter keeps a second bounded map keyed on the SHA-256 of
  the bearer token, so the token itself is never a key. Approve and revoke keep the address budget.

## Frontend behaviour (not applicable)

This unit changes no frontend file. The support console that consumes support access is U9 and the
admin approval surface is U10; neither is in this unit's scope. No file under `web/` or
`support-console/` is created, modified, or deleted, so there is no frontend Red step and no
Vitest, ESLint, Prettier, or `yarn build` obligation introduced by this unit.

## Red evidence

### Data model / database behaviour

```sh
TEST_DATABASE_URL="postgres://postgres:postgres@127.0.0.1:5432/cpb_u3_support" \
  cargo test -p identity --test support_adapters -- --ignored
```

Before `0007_support_access` existed: `test result: FAILED. 3 passed; 7 failed`. First failing
assertion:

```
the_support_outbox_has_the_state_machine_columns_and_partial_indexes
panicked at services/identity/tests/support_adapters.rs:95:9:
support_audit_outbox is missing the attempts column; found []
```

Also captured: `the_approval_shape_admits_an_unapproved_expired_row_and_nothing_else` (the old
check constraint refused `expired` without approval fields) and
`a_second_live_grant_for_the_same_pair_is_rejected_by_the_partial_index` (`Ok(()) — a second live
grant was accepted`).

### Repository / data access

After the outbox and the drainer claims were added, the adapter suite failed on the drainer
contract: `test result: FAILED. 15 passed; 5 failed`. First failing assertions:

```
a_permanent_failure_is_dead_on_the_first_attempt
panicked at services/identity/tests/support_adapters.rs:759:58:
called `Option::unwrap()` on a `None` value

two_concurrent_claims_never_take_the_same_row
panicked at services/identity/tests/support_adapters.rs:657:5:
assertion `left == right` failed: a claimed row is hidden from the other drainer by SKIP LOCKED
  left: 0
 right: 4
```

### Business logic

```sh
cargo test -p identity --test support_domain
```

`test result: FAILED. 6 passed; 3 failed`. First failing assertion:

```
an_approval_window_outside_one_to_sixty_minutes_is_refused
panicked at services/identity/tests/support_domain.rs:95:5:
assertion failed: matches!(zero.approve(UserId::generate(), Role::Admin, at(10), Duration::minutes(0)),
    Err(DomainError::Invalid { field: "expiresAt", .. }))
```

The other two: `an_active_grant_reads_as_expired_at_its_expiry_and_the_observation_persists`
(`left: Some(2026-11-07T10:30:00Z) right: Some(2026-11-07T11:00:00Z)`) and
`an_approval_sets_the_window_from_the_approval_instant` (same 30-vs-60-minute mismatch).

### API / endpoint

```sh
cargo test -p identity --test support_http
```

`test result: FAILED. 9 passed; 11 failed`. First failing assertions, abridged:

- `a_malformed_support_id...`: left 422, right 400.
- `the_metadata_projection...`: left 404, right 200; body `"detail":"the route does not exist"`.
- `an_unknown_or_foreign_grant...`, `a_metadata_read_by_a_non_support_caller...`,
  `the_holders_own_inactive_grant...`, `an_expired_grant...`: left 404, right 403.
- `an_unreachable_metadata_source...`, `an_unrecordable_served_read...`,
  `an_unrecordable_refusal...`: left 404, right 503.
- `a_backend_service_token...`: left 404, right 401.

Gateway and owner Red was compile-level, which is a failing test in this workspace:

- `cargo test -p gateway --test domain`: `no field 'max_requests_per_minute' on type '&Route'`
  (lines 213, 257, 272, 280).
- `cargo test -p gateway --test http`: `no method named 'check_route' found for struct
  'RateLimiter'` (lines 669, 670, 678, 683).
- `cargo test -p account --test support_metadata`: `no variant ... named 'SupportMetadata' found
  for enum 'Operation'` (line 143).
- `cargo test -p portfolio --test support_metadata`: the same error (lines 65, 343).

### Refactor / metrics Red found a real defect

Step 16 needed a write-latency signal for the tenth alert and none existed. The test written first
for `support_write_duration_seconds` failed on both assertions and exposed a second, pre-existing
bug in the read histogram: its zero-init `+Inf` line was written through `push_str` with doubled
braces, rendering `support_read_duration_seconds_bucket{{le="+Inf"}} 0` — invalid exposition text
that would have broken the whole scrape. Both are fixed.

## Green evidence

| Command | Result |
|---|---|
| `cargo test -p identity --test support_domain --test support_application --test support_http` | 52 passed (3 suites) |
| `cargo test -p identity --test support_adapters -- --ignored` (disposable Supabase Postgres) | 20 passed |
| `cargo test -p gateway --test domain --test http` | 35 passed (2 suites) |
| `cargo test -p account --test support_metadata` | 5 passed |
| `cargo test -p portfolio --test support_metadata` | 6 passed |

The new `support_write_duration_seconds` series and the four `support.grants*` route labels are
additionally asserted by the two new metrics tests in `support_http.rs`.

## Sensors (Step 17b)

| Sensor | Output | Result |
|---|---|---|
| `required-sections` | `code-summary.md` | passed |
| `upstream-coverage` | `code-summary.md` | passed |
| `linter` | — | not applicable: the sensor matches `**/*.{ts,js}` and this unit changes no TS/JS file |
| `type-check` | — | not applicable: the sensor matches `**/*.{ts,tsx}` and this unit changes no TS/TSX file |
| `traceability` (declared on the stage) | `traceability.json` | passed after correcting the table to the closed status set (`OK`/`GAP`/`ORPHAN`/`Deferred`/`N/A`) and one existing workspace-relative `target` per row |

## Quality gates (Step 18)

| Gate | Command | Result |
|---|---|---|
| Formatting | `cargo fmt --all -- --check` | pass |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | pass, no issues |
| Boundaries | `bash scripts/check-boundaries.sh` | fails on the pre-existing `reporting-domain` → serde/serde_json edge only; no new violation |
| Unit + integration | `cargo test --workspace` | **1582 passed, 136 ignored, exit 0** |
| Ignored adapters | `TEST_DATABASE_URL=<disposable Supabase Postgres> cargo test --workspace -- --ignored` | 3 failures, all in `services/identity/tests/repositories.rs` and all on code this unit did not touch (see Known gaps); every support-access adapter test passes (20) |
| Coverage | `TEST_DATABASE_URL=<disposable> cargo llvm-cov --workspace --ignore-run-fail -- --include-ignored` | **64.56% line coverage workspace-wide; the 80%-per-crate target is not met** (see Known gaps) |

The disposable database was created per gate run from the project's Supabase test template
(`cpb_test`) on the `cpb-u1-test-postgres` container, so `pgsodium`, `pgcrypto` and
`supabase_vault` are present. Each test opens its own schema.

### Coverage detail

| Crate | Line coverage |
|---|---|
| `identity-domain` | 92.27% |
| `identity-application` | 85.58% |
| `identity-http` | 75.20% |
| `identity-infrastructure` | 47.97% |
| `gateway-domain` | 92.39% |
| `gateway-application` | 86.82% |
| `gateway-http` | 83.99% |
| `gateway-infrastructure` | 69.70% |
| `account-domain` | 91.71% |
| `account-application` | 82.26% |
| `account-http` | 75.95% |
| `account-infrastructure` | 21.83% |
| `portfolio-domain` | 92.79% |
| `portfolio-application` | 79.95% |
| `portfolio-http` | 41.75% |
| `portfolio-infrastructure` | 18.28% |
| every service composition root (`services/*/src/lib.rs`), most workers and migrate binaries | 0% |

Within this unit's own files: `domain/src/support.rs` 97.53%, `application/src/support.rs`
85.04%, `infrastructure/src/support/outbox.rs` 94.74%, `support/backlog.rs` 94.44%,
`infrastructure/src/repositories/support.rs` 65.10%. `support/drainer.rs`,
`support/observation.rs` and `support/sources.rs` measure 0% because only the composition root
(`src/lib.rs`) and a live HTTP peer reach them, and no unit test instantiates either. No target
was lowered to make this gate pass; the shortfall is reported as a gap.

## Metrics and alerts

`infra/prometheus/support-rules.yml` carries one group of ten rules. Every rule has `severity` and
`service` labels and both `summary` and `runbook` annotations, and every threshold sits below the
SLO breach it protects:

`SupportMetadataSourceErrors` (P2), `SupportReadUnavailable` (P2), `SupportWriteFailures` (P2),
`SupportGrantDecisionErrors` (P3), `SupportAuditFailingClosed` (P1), `SupportOutboxBacklog` (P2),
`SupportOutboxStalled` (P2), `SupportOutboxDeadRows` (P2), `SupportReadLatency` (P3),
`SupportWriteLatency` (P3).

The `Metrics` sink is constructed in `services/identity/src/lib.rs::run`, which
`services/identity/src/bin/api.rs` calls; the binary holds no construction code, so wiring it there
is the smallest correct place. `/metrics` is asserted to render every support series
zero-initialised before the first event.

## Deviations from the plan

1. `sha2` added to `services/gateway/http/Cargo.toml` (workspace-pinned) for the token-hash
   rate-limit key.
2. `support/drainer.rs` and the `SupportDrainObservation` / `SupportSourceFailureSink` ports were
   added; the plan names only "the sink the drainer calls", and no support drainer existed.
3. `PathPattern` gained a `raw` field and `as_str`, so the declared route pattern can label the
   per-route bucket.
4. `infra/sql/account-service-tokens.sql` is new (Account had no seed file); it seeds the identity
   caller with `privacy_export`, `privacy_anonymise` and `support_metadata`.
5. `sources.rs` calls `/v1/internal/support-metadata/{userId}` (path parameter). The design note's
   query-parameter form was corrected to match the owner routes.
6. Owner internal endpoints answer 200 with an empty projection for a missing target, so a support
   read is never an existence oracle.
7. `support_write_duration_seconds` and the four `support.grants*` route labels were added so the
   tenth alert and the write-route dashboard row rest on a real series; the monitoring design names
   only `support_read_duration_seconds`. The write histogram measures the use case inside the
   handler, not the full HTTP request, because the repository has no HTTP latency histogram.
8. The two supporting migrations tests and the Account/Portfolio boundary tests required a
   session-lock fix, a test-harness migration gate, an explicit `now` on outbox writes and an
   atomic down-migration `ALTER`; these are code-level corrections inside the plan's file set.
9. `services/identity/tests/account_lifecycle_adapters.rs` reverts migration 0006 by calling
   `migrations::revert`, which reverts the newest applied version. With `0007_support_access`
   applied, both U5 rollback tests now revert 0007 first and then 0006, so each gained one
   explicit 0007-revert step; the 0006 assertions are unchanged. Without this the two tests fail
   on `erased_subject_hash` and on the open-request guard.
10. `services/identity/tests/repositories.rs`'s support-grant round-trip fixture requested at
   09:00 and approved at exactly 10:00. BR3.5's design rule is `now - requestedAt >= 60 minutes →
   refused`, so that fixture sits exactly on the refusal boundary. The fixture now requests at
   09:01, which keeps it inside the window and leaves every assertion about the approval instant
   and expiry unchanged. The design was not relaxed to make the old fixture pass.

## Known gaps

- The 80%-per-crate line-coverage target is not met workspace-wide (64.56% overall). This is not
  introduced by this unit: the composition roots and workers are 0%, and several pre-existing
  adapters are far below 80%. The unit's own adapter modules `drainer.rs`, `observation.rs` and
  `sources.rs` measure 0% because no test instantiates the composition root or a live HTTP peer.
  The target stands; the shortfall is reported rather than waived.
- Three `#[ignore]`d tests in `services/identity/tests/repositories.rs` fail against a correct
  Supabase database, on code paths this unit does not modify:
  `a_user_is_found_by_the_hashed_subject_and_the_subject_is_protected_at_rest` (anonymise refuses
  an active user with `the account is no longer pending deletion`),
  `protect_and_unprotect_are_security_definer_functions_the_app_may_execute` (`a destroyed key no
  longer decrypts the subject` fails), and
  `user_registry_migration_closes_the_status_set_and_the_down_maps_new_states` (the down migration
  maps `disabled`/`pending_deletion` to themselves rather than to `active`). The dispatch named two
  known failures; the environment shows three. All three reproduce in isolation and none of the
  files involved is in this unit's diff.
- The Structurizr model could not be validated end to end: the CLI fetches the default theme from
  `https://static.structurizr.com`, which does not resolve from this environment. No model syntax
  error was reported before the fetch.

## Architecture

`docs/architecture/workspace.dsl` gains the two Identity→owner relationships and the updated
`SupportAccess` dynamic view; `docs/architecture/README.md` gains the support-access route table,
the two internal endpoints and the `support_grants` / `support_audit_outbox` ownership note. The
model could not be validated: the Structurizr CLI reaches the theme at
`https://static.structurizr.com/themes/default/theme.json`, which does not resolve from this
environment. No syntax error was reported before the theme fetch.

## Loop-back 1 repair (Build and Test, 2026-11-12)

Build and Test reopened this Unit for **F6 only** — the 80 % per-crate line floor for the four
crates this Unit owns. Steps 1–18 stand as they were. The repair adds tests and changes no
production file: no test exposed a defect in the code under test, so there was nothing to fix
test-first.

### Before and after

Lines covered / lines counted, measured with `cargo llvm-cov`. The before-picture was
re-measured per crate, each crate instrumented together with the package that carries its tests
(`-p account-http -p account`, `-p portfolio-http -p portfolio`,
`-p gateway-infrastructure -p gateway`), because `-p <crate>` on its own instruments only that
package and does not run the service package's test binaries — the same `--ignore-run-fail --json`
measurement the plan's step 19 names. The after-picture is the same four crates in the step 25
workspace run
(`cargo llvm-cov --workspace --ignore-run-fail --json -- --include-ignored --test-threads=1`), so
the ignored adapter suites are counted too.

| Crate | Before | After | Floor |
|---|---|---|---|
| `account-http` | 780 / 983 — 79.35 % | 803 / 983 — 81.69 % | met |
| `portfolio-http` | 629 / 1 386 — 45.38 % | 1 273 / 1 386 — 91.85 % | met |
| `gateway-infrastructure` | 145 / 211 — 68.72 % | 200 / 211 — 94.79 % | met |
| `gateway` (composition root) | 3 / 93 — 3.23 % | 84 / 93 — 90.32 % | met |

`identity-infrastructure` (64.95 %, unchanged) belongs to `u1-user-registry` and was not touched.

### Tests added

| File | Tests | What it drives |
|---|---|---|
| `services/portfolio/tests/routes_contract.rs` (new) | 50 | Every route the portfolio crate serves over `http::router` and `worker::router`: list/create/get portfolios (including the default portfolio and the drift prompt), account assignment, target preview/save/out-of-range, order-intent preview/create/cancel and the bad-side refusal, trade record with a bad price and a foreign portfolio, snapshot history with `x-has-more`/`x-next-cursor`, the reporting service token, snapshot detail verified/tampered/malformed id, the internal user-portfolios and valuation endpoints with and without the operation, privacy export across all nine collections and anonymise idempotency, support-metadata smoke, identity 503, registry 503, unknown token, missing bearer, invalid correlation id, security headers, healthz, readyz not-ready, the 404 route and the metrics exposition; plus two listener tests that bind an ephemeral port and read `/healthz` and `/readyz` over a real socket |
| `services/account/http/tests/http.rs` (extended) | 2 new (26 in file) | The API and worker listeners on a bound port: `serve` answers `/healthz` with `"status":"ok"`, `serve_worker` answers `/healthz` |
| `services/gateway/tests/adapters.rs` (new) | 9 | The JWKS fetch keeps only usable RSA keys and names a key without a `kid`, refuses a document with no RSA keys, refuses a non-2xx response, and reports an unreachable endpoint; the readiness probe for 2xx, 503 and an unreachable dependency; the system clock's instant; the audit sink accepting a complete event |
| `services/gateway/tests/composition.rs` (new) | 3 | Configuration reads all nine variables, falls back to each documented default, names each of the three missing required variables and each malformed numeric variable; `build_state` wires the service name, health, metrics, readiness and the route table (a declared route reaches the dead upstream as 502/503 while an undeclared route is the gateway's own 404, driven by a real signed JWT and a loopback JWKS document); `run` binds the configured port and serves it |

### Red evidence

- `portfolio/tests/routes_contract.rs` first compile failed, which is a failing test in this
  workspace: `expected MutableClock, found Arc<MutableClock>` at the harness clock field. The
  harness field was corrected to `Arc<MutableClock>` and the suite then ran 50 passed.
- `gateway/tests/composition.rs` first run:
  `assertion left == right failed: left: 401, right: 404` for `/v1/not-declared`. The cause was
  the test's assumption, not the gateway: the gateway authenticates before it routes, so an
  unauthenticated request is 401 whatever the path. The test now signs a real token against a
  loopback JWKS endpoint; the declared route answers 502/503 from the dead upstream and the
  undeclared route answers 404. No production change.
- `account/http/tests/http.rs`, `gateway/tests/adapters.rs` and the gateway configuration test
  passed on their first run: they exercise code that already existed and behaved correctly.

### Paths left uncovered, and why

Every crate clears the floor, so these are reported rather than chased with assertions that
observe nothing:

- **Graceful shutdown** — `http/src/server.rs` in both `account-http` (lines 16, 26, 31, 39, 41,
  50) and `portfolio-http` (15, 25, 30, 38, 40, 49): the `with_graceful_shutdown(shutdown_signal())`
  chain and the SIGINT/SIGTERM handler. Covering it means signalling the test process, which
  would kill the test runner; the listeners themselves are covered by the bound-port tests.
- **Defensive 401 fallbacks** — `portfolio-http/src/handlers.rs` (e.g. 439–449, 513–518) and the
  same shape elsewhere: the arm reached only when neither a validated claim nor a service
  principal is attached, which the auth middleware makes unreachable through the router.
- **503 arms the suites do not drive** — `account-http/src/middleware.rs` 155–162, 183–190 and
  `portfolio-http/src/middleware.rs` 222–230, 238–246, 266–273: the `identity-unavailable` and
  `registry-unavailable` responses in `require_jwt_or_service` and `require_worker_service`. The
  routes the suites drive with a failing dependency use `require_jwt` / `require_service`, whose
  503 arms are covered.
- **Worker endpoints not reached** — `account-http/src/worker.rs` 62–85 and 125–140 (`/readyz`
  503, `/metrics`, `POST /wake`, the 404 fallback) and `portfolio-http/src/worker.rs` 115–119
  (the worker's conflict arm). The account suite drives the API router; only the worker listener's
  `/healthz` is exercised.
- **Gateway infrastructure refusals** —
  `gateway/infrastructure/src/auth.rs` 107 (JWKS body that is not valid JSON), 240 (a token whose
  `kid` matches no key), 254 (empty subject), 264–265 (unknown role), and
  `gateway/infrastructure/src/upstream.rs` 78 (`Method::from_bytes` rejection) and 134 (response
  body read error). Each is a refusal branch; the happy path and the three error classes named by
  step 23 are covered.
- **Gateway process entry and clean return** — `gateway/src/main.rs` 4–7 is the binary entry
  point, and `gateway/src/lib.rs` 98–100 is the `serve`-returned path in `run`, reachable only
  when the listener ends; the test aborts the server task instead.
- **Remaining handler and DTO branches** — the residual lines listed by
  `cargo llvm-cov report --show-missing-lines` in `portfolio-http` (dto 46–64, metrics 49–84,
  problem 111–156) and `account-http` (dto 223–231 wallet-export mapping, problem 72–160) are
  conversion and problem variants that no route in the exercised set produces.

### Gates after the repair

| Gate | Command | Result |
|---|---|---|
| Formatting | `cargo fmt --all -- --check` | pass (the new test files were reformatted with `cargo fmt --all` before the check) |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| Boundaries | `bash scripts/check-boundaries.sh` | fails on the pre-existing `reporting-domain` → serde/serde_json edge only; no new violation |
| Coverage | `TEST_DATABASE_URL=<disposable Postgres> cargo llvm-cov --workspace --ignore-run-fail --json --output-path target/coverage.json -- --include-ignored --test-threads=1` | every crate this Unit owns clears 80 % (table above); `-p identity --test repositories` still fails on one pre-existing test (`protect_and_unprotect_are_security_definer_functions_the_app_may_execute`, `a destroyed key no longer decrypts the subject`, `services/identity/tests/repositories.rs:439`); the other two tests listed under Known gaps pass against this run's fresh database |
| Traceability (Build and Test view) | `aidlc engine sensor-traceability --stage build-and-test` | `pass:false`, with the uncovered ids belonging to other Units (`FR17.4`→`AC4.4.2`, `FR18.1`–`FR18.3`→`AC5.1.1`–`AC5.3.2`, `NFR26`, `AC8.1.1`); every `AC3.x` row this Unit owns is covered |