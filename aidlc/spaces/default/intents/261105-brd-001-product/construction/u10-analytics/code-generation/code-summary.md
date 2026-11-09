# Code Summary — u10-analytics

## What was built

The Analytics service as a five-package Cargo workspace under `services/analytics/`, matching
the walking skeleton's shape and the approved plan step for step.

| Package | Responsibility |
|---|---|
| `analytics-domain` | Closed event catalogue and per-event allow-lists, the PII/credential detector, consent purpose and `ConsentCheck`, `FeatureFlag` governance and effective status, aggregate formulas and the ten-user suppression rule, retention cutoffs. `sessionRef` was removed from the entity, DTO, table, and repository; events carry only the salted `userRef`. |
| `analytics-application` | Ingest (BR1.1–BR1.3, BR2.1–BR2.3), flag evaluation and administration (BR4.1–BR4.3), nightly aggregation (BR3.1–BR3.2), retention (BR5.1), erasure under every retained salt version (BR6.1), HMAC/bucket hashing, and the C10 trail |
| `analytics-infrastructure` | sqlx repositories, JWKS validation, the Postgres service-token registry, Vault salt versions, C3/C10 HTTP clients, the readiness probe, migrations, and the batch pause |
| `analytics-http` | Two Axum routers, DTOs, RFC 7807 problems, correlation id, security headers, `X-Auth-Subject` agreement check, JWT roles, service-token operation allow-lists, Prometheus metrics |
| `analytics` | Composition root and three binaries: `analytics-api`, `analytics-worker`, `analytics-migrate` |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `POST /v1/events` | JWT, `X-Auth-Subject == sub` | 202 `{status: accepted}` after catalogue/PII validation, pseudonymisation, and a per-request C3 consent check; 403 when consent is absent or withdrawn; 503 when consent or the salt is unavailable (fail closed); 422 naming the offending field paths |
| `GET /v1/flags/{key}` | JWT, `X-Auth-Subject == sub` | `{key, enabled}`; missing, disabled, or expired flags are off; otherwise `SHA-256(subject) mod 100 < rolloutPercentage`; a 30-second per-replica cache |
| `GET /v1/admin/flags` | JWT + `admin` | every flag with its derived status; `?expired=true` lists only expired flags |
| `POST /v1/admin/flags` | JWT + `admin` | 201; owner and future expiry mandatory, rollout 0–100; every change is audited through C10 |
| `PATCH /v1/admin/flags/{key}` | JWT + `admin` | partial update of owner, expiry, rollout, or active/disabled; expired is derived and refused as input |
| `DELETE /v1/admin/flags/{key}` | JWT + `admin` | 204; unknown key 404; audited |
| `GET /v1/analytics/aggregates` | JWT + `admin` | filtered, newest-first aggregate page (metric, period, from, to, limit ≤ 500) |
| `POST /v1/internal/privacy/erase` | Identity service token + `privacy.erase` | hashes the raw auth user id under every retained salt version, deletes every matching event, audits, and answers 204; any other caller 403 |
| `GET /v1/internal/smoke` | service token + `smoke` | readiness |
| `POST /v1/internal/run/analytics-aggregate` | service token + `runs` (worker router) | nightly aggregation; returns stored and suppressed counts |
| `POST /v1/internal/run/analytics-retention` | service token + `runs` (worker router) | six-month event and 90-day consent sweep in 10 000-row batches |
| `GET /healthz`, `GET /readyz`, `GET /metrics` | public | readiness checks Postgres, `_sqlx_migrations`, and the current salt; metrics are low-cardinality |

Deny by default: only the routes above exist; anything else is a 404 problem document.

### Database

`services/analytics/migrations/0001_init.up.sql` creates:

- `events` with the catalogue class/purpose consistency check, a JSON-object property check, and
  the `(event_name, occurred_at)`, `(user_ref)`, and `(occurred_at)` indexes;
- `consent_checks` and its `checked_at` index;
- `metric_aggregates` keyed by `(metric, period, cohort, period_start)` and its read index;
- `feature_flags` with a unique key, the rollout range check, and the expiry index;
- `service_token_registry` storing only SHA-256 hashes plus each caller's operation allow-list;
- `salt_versions` storing version labels only, and `read_analytics_salt(version)` as a
  `SECURITY DEFINER` PL/pgSQL function reading `vault.decrypted_secrets` under
  `analytics/salt/<version>` through dynamic SQL (so a disposable database without Vault still
  creates the function and fails loudly only when a salt is requested);
- roles `analytics_svc`, `analytics_readonly`, `analytics_migrator` and exactly the grants the
  runtime needs: read/insert/delete events and consent checks (erase and retention run under the
  one runtime role), read/insert/update aggregates, read/insert/update/delete flags, read the
  token registry, read salt labels, execute the salt function.

The down migration refuses to drop `events` while a row is inside the six-month retention window.
The roles survive a rollback because they are cluster bootstrap objects.

### Deployment assets

- `services/analytics/Dockerfile.api` (exposes 8095), `Dockerfile.worker` (8096), and
  `Dockerfile.migrate`: multi-stage, `USER 10001`.
- `infra/swarm-stack.yml`: `analytics-api` (2 replicas, 0.2 vCPU / 192 MB, start-first),
  `analytics-worker` (1 replica, 0.25 vCPU / 256 MB, stop-first so runs never overlap), and
  `analytics-migrate` (one-shot). All join only the encrypted `internal` overlay; secrets are
  `analytics_database_url`, `analytics_migrate_database_url`, `analytics_outbound_token`, and
  `analytics_smoke_token`.
- `infra/sql/analytics-schema.sql`: the bootstrap that creates the schema and roles and records
  the Vault paths, with no credential in the file.
- `.env.example` gained the non-secret analytics variables and secret paths.

## How to run it

```
cargo test -p analytics-domain -p analytics-application -p analytics-infrastructure -p analytics-http
cargo fmt --all -- --check
cargo clippy -p analytics -p analytics-domain -p analytics-application \
  -p analytics-infrastructure -p analytics-http --all-targets -- -D warnings
```

Adapter suite (needs a disposable Postgres whose user may create roles):

```
TEST_DATABASE_URL=postgres://test:test@localhost:55432/cpb_test \
  cargo test -p analytics-infrastructure -- --ignored
```

API locally (secrets through files):

```
ANALYTICS_PORT=8095 \
DATABASE_URL_FILE=/run/secrets/analytics_database_url \
OUTBOUND_TOKEN_FILE=/run/secrets/analytics_outbound_token \
SMOKE_TOKEN_FILE=/run/secrets/analytics_smoke_token \
JWT_JWKS_URL=... JWT_ISSUER=... JWT_AUDIENCE=... \
  cargo run -p analytics --bin analytics-api
```

## Verification output (run in this workspace)

```
$ cargo test -p analytics-domain -p analytics-application -p analytics-infrastructure -p analytics-http
analytics-domain:         38 passed; 0 failed
analytics-application:    38 passed; 0 failed
analytics-infrastructure:  5 passed; 0 failed; 8 ignored (adapter suite)
analytics-http:           32 passed; 0 failed

$ cargo fmt --all -- --check
(no output, exit 0)

$ cargo clippy -p analytics -p analytics-domain -p analytics-application \
    -p analytics-infrastructure -p analytics-http --all-targets -- -D warnings
(no warnings, exit 0)
```

113 tests run and pass; 8 adapter tests are `#[ignore]`d because no disposable Postgres is
available here. They compile, and the command is above.

## Gaps and deviations accepted

1. **Aggregation reads three separate bounded windows and computes in Rust.** The performance
   design describes one SQL `GROUP BY` pass. The implemented path reads `[day-7, day+2)` for the
   daily metrics (500 000-event cap), `[day-7, day-6)` for the day-7 cohort, and
   `[day-30, day-29)` for the day-30 cohort (200 000-event cap each) through the
   `(event_name, occurred_at)` index. A window that exceeds its cap fails the run instead of
   truncating; the closed day is therefore always complete. The scan counts are returned on the
   run response. A single grouped statement remains a performance follow-up if volume grows.
2. **Retention carries its age in the metric name.** `MetricAggregate` has no retention-age
   column, so day 7 is stored as metric `retention_d7` and day 30 as `retention_d30`, both with
   the non-identifying `all` cohort label, weekly, keyed by the run week's start.
3. **Run endpoints live on the worker.** The API (8095) serves C12 ingest, C1 flags and
   aggregates, erase, and smoke; the worker (8096) serves the two C11 run paths and the probes.
   This matches the infrastructure design's two binaries and ports.
4. **`X-Auth-Subject` is required on every user path**, not only ingest. The security design says
   the header must equal the token's `sub`; requiring it everywhere applies that rule uniformly.
   A missing header is 401, a mismatch 403.
5. **One runtime role holds delete rights.** `analytics_svc` has `DELETE` on `events` and
   `consent_checks` because erasure runs synchronously in the API and retention in the worker,
   and the infrastructure design gives the service one schema role. No cross-schema grant exists.
6. **Vault salt formats.** `decode_salt` accepts a 64-character hex string, a 44-character base64
   string, or 32 raw bytes, and refuses anything that does not decode to exactly 32 bytes. The
   migration seeds no salt: the platform bootstrap creates `analytics.salt_versions` rows and the
   Vault secrets, and the function fails loudly when one is absent.
7. **The service-token registry is database-backed.** The deploy job seeds the gateway, Identity,
   and job-orchestration hashes. The optional `analytics_smoke_token` Docker secret is added to
   the in-process registry as `smoke-reader`, so release smoke does not depend on a registry
   write ordering.
8. **Aggregation and retention runs are not audited through C10.** The security design requires
   audit for flag create/update/delete, erasure, and refused ingests (consent refused, event
   rejected, salt unavailable); those are emitted. C10 refuses `actorKind: system` from an
   emitter, requires a UUID actor for `user`, and allows only identifier keys in context. The
   audit actor is therefore a pseudonymous UUID derived from the user-reference hash (never the
   raw subject), the refusal category is recorded as `ruleId`, and erasure names the acting
   service in `sourceService`. A refused consent stores no `ConsentCheck` row (BR1.1); the
   accepted analytics path still stores its granted check. Erasure deletes both the user's events
   and their `consent_checks` rows under every retained salt version. Run outcomes are visible in
   the log and in the `analytics_aggregate_runs_total` counter.
9. **80% line coverage per crate was not measured.** `cargo llvm-cov` is not installed in this
   workspace. The suite has 113 passing tests and every test file covers a happy path plus at
   least two error or edge cases; the coverage gate remains unverified here.
10. **The eight adapter tests were not executed** because no disposable Postgres was available.
    They cover migrations up/down, event+consent atomic insert, retention, erase by user refs,
    flag CRUD, aggregate upsert, consent retention, and token resolution.
11. **No idempotency key on ingest.** The reliability design states events have no natural key and
    duplicates are accepted and counted, so the ingest path inserts every accepted call.

## Build and Test loop-back repair (2026-11-09)

Two adapter-test defects were repaired in
`services/analytics/infrastructure/tests/repositories.rs`; no product code changed.

1. The fixtures used `user_ref` values shorter than the schema's 32–128 character check
   (`"user-ref"`, `"new"`, `"ref-a"`, `"ref-b"`), so four tests failed on the check constraints.
   Every reference now goes through a `fixture_user_ref` helper that pads to 32 characters.
2. `retention_deletes_only_rows_older_than_the_cutoff` built its "old" row at `at(10)` because
   `AnalyticsEvent::store`'s fourth argument is `ingested_at`, while retention keys on
   `occurred_at`; the event is now sanitised at `at(10) - 200 days`, outside the 180-day cutoff.

Result: adapter suite 8/8 green; unit suite 113 passed / 8 ignored; format and clippy clean.
