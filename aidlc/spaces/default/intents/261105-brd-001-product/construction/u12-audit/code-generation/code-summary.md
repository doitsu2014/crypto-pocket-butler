# Code Summary — u12-audit

## What was built

The Audit service as a five-package Cargo workspace under `services/audit/`, matching the
walking skeleton's shape and the approved plan step for step.

| Package | Responsibility |
|---|---|
| `audit-domain` | Append-only `AuditEvent`, the shared secret-shape detector, `SupportSession`, `RetentionRun` (six-calendar-month cutoff and the drop-vs-batch decision), `AuditExport`, `PrivacyDeletion` |
| `audit-application` | Append (BR1.1–BR1.4, BR2.2), admin query, export, support-session start/check/revoke (BR3.1–BR3.3, BR5.1), retention sweep and report (BR4.1–BR4.2), privacy deletion (BR4.3), and the service's own denial/audit write path |
| `audit-infrastructure` | sqlx repositories, JWKS validation, the service-token registry, the Postgres readiness probe, migrations, and the batch-pause adapter |
| `audit-http` | Axum router, DTOs, RFC 7807 problems, correlation id, security headers, service-token auth, JWT roles, per-replica rate limiting, JSONL export streaming, Prometheus text metrics |
| `audit` | Composition root and the two binaries: `audit-api` and `audit-retention` |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `POST /v1/events` | internal service token | 201 with the event id (also on a same-payload retry), 409 on a reused key with a new payload, 422 naming field paths, 503 on storage |
| `GET /v1/events` | JWT + `admin` | cursor-paginated, newest first, mandatory `recordedAt` range |
| `POST /v1/exports` | JWT + `admin` | the response body **is** the export: JSON Lines streamed from 5 000-row batches, `x-export-id`, sha-256 of the exact bytes recorded on success |
| `POST /v1/support-sessions` | JWT + `support` | reason (10–500 chars), independent second approver, expiry ≤ 8 h; a refusal stores a `denied` record and answers 403 |
| `GET /v1/support-sessions/{id}/check?scope=` | internal service token | `allow` only for an active, unexpired, in-scope session; the read event is written **before** the allow; any failure is a deny |
| `POST /v1/support-sessions/{id}/revoke` | JWT + `admin` | ends an active session; a terminal session is immutable |
| `POST /v1/privacy/deletions` | internal service token | records the request and answers 202; the retention worker executes it |
| `POST /v1/runs` | internal service token | records another service's completed retention run (`category: reported`); refuses another service's owner, a missing role, or a missing outcome |
| `GET /healthz`, `GET /readyz`, `GET /metrics` | public | readiness checks the database and `_sqlx_migrations`; metrics are low-cardinality only |

Deny-by-default: only the routes above exist; anything else is a 404 problem document. A
role denial writes a security event before the 403.

### Database

`services/audit/migrations/0001_init.up.sql` creates:

- `audit_events` partitioned by `recorded_at` month, parent plus the current and next three
  months, with the five performance-design indexes, the derived-class check, the actor-shape
  check, and the append-only-by-grant model;
- `append_idempotency` (unpartitioned) with `PRIMARY KEY (emitting_service, idempotency_key)`,
  written in the same transaction as the event;
- `support_access_records`, `retention_runs`, `audit_exports`, `privacy_deletions`, each with a
  state-shape check and a terminal-state trigger;
- `create_audit_partition(date)` and the `SECURITY DEFINER` wrapper `ensure_audit_partition(date)`
  so the retention role can pre-create partitions without owning the parent;
- roles `audit_app`, `audit_readonly`, `audit_retention`, `audit_migrator` and exactly the grants
  in the security design, plus the two minimal additions noted under deviations.

The down migration refuses to drop the trail while any record is inside the six-month window.
The roles survive a rollback: they are cluster bootstrap objects (`infra/sql/audit-schema.sql`),
and the up migration creates them idempotently so a disposable database converges.

### Deployment assets

- `services/audit/Dockerfile.api` and `services/audit/Dockerfile.retention`: multi-stage,
  `USER 10001`, the retention image writes only its tmpfs heartbeat.
- `infra/swarm-stack.yml`: `audit-api` (3 replicas, 0.5 vCPU / 512 MB, no published port,
  `read_only: true`, health-check gated rolling update with `failure_action: rollback`) and
  `audit-retention` (1 replica, 0.25 vCPU / 256 MB, heartbeat health check, `condition: any`,
  `max_attempts: 0`). Both join only the `internal` overlay; secrets are
  `audit_app_dsn`, `audit_retention_dsn`, `audit_service_tokens`.
- `scripts/check-boundaries.sh`: reads `cargo metadata` and fails on any outward dependency.
- `.env.example` gained the non-secret audit variables and the `SERVICE_TOKENS_FILE` path.

## How to run it

```
cargo test --manifest-path services/audit/Cargo.toml            # no database needed
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
scripts/check-boundaries.sh
```

Adapter suite (needs a disposable Postgres whose user may create roles):

```
TEST_DATABASE_URL=postgres://test:test@localhost:55432/cpb_test \
  cargo test --manifest-path services/audit/Cargo.toml --test repositories -- --ignored
```

API locally (secrets through files, never env files):

```
AUDIT_PORT=8084 \
DATABASE_URL_FILE=/run/secrets/audit_app_dsn \
SERVICE_TOKENS_FILE=/run/secrets/audit_service_tokens \
JWT_JWKS_URL=... JWT_ISSUER=... JWT_AUDIENCE=... \
  cargo run -p audit --bin audit-api
```

The service-token file is a JSON map `{"<service>":"<token>"}` (or `{"services":{...}}`); the
token identifies the emitting service and is never echoed in a log or a response.

## Verification output (run in this workspace)

```
$ cargo fmt --all -- --check
(no output, exit 0)

$ cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found

$ cargo test --workspace
(50 suites, all "test result: ok"; 0 failures)
audit domain:        test result: ok. 28 passed; 0 failed; 0 ignored
audit application:   test result: ok. 34 passed; 0 failed; 0 ignored
audit http:          test result: ok. 22 passed; 0 failed; 0 ignored
audit smoke:         test result: ok. 4 passed; 0 failed; 0 ignored
audit repositories:  test result: ok. 0 passed; 0 failed; 10 ignored

$ TEST_DATABASE_URL=... cargo test -p audit --test repositories -- --ignored --test-threads=1
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.24s

$ psql ... \i 0001_init.up.sql \i 0001_init.down.sql \i 0001_init.up.sql
MIGRATION_UP_DOWN_UP_EXIT=0

$ scripts/check-boundaries.sh
boundaries OK (15 packages checked)
```

A live smoke against a disposable Postgres also passed: `GET /healthz` 200, `GET /readyz` 200,
`POST /v1/events` 201, the same-key/same-payload retry 201 with the first id and `created: false`,
a same-key/different-payload request 409, `POST /v1/runs` 201, `POST /v1/privacy/deletions` 202,
and an unknown session check 200 `deny`. The smoke found and fixed one real defect: a retry
without `occurredAt` was hashing the service's receipt time, so an identical retry looked like a
different payload. The idempotency hash now covers only the emitter's payload.

## Gaps and deviations accepted

1. **Two endpoints beyond the dispatch list.** `POST /v1/runs` is required by WF6/BR4.2 (Ledger
   reports its own run) and `POST /v1/support-sessions/{id}/revoke` by the support-session state
   machine and the security design's "admin — session revocation". Both are tested;
   `POST /v1/runs` is the C11 report surface for this unit.
2. **Privacy deletion is asynchronous.** The API records a `PrivacyDeletion` and answers 202; the
   retention worker (the only holder of a `DELETE` grant) polls `requested` rows and executes
   them. WF7 step 4's "answers with what it deleted and kept" is therefore delivered on the
   `privacy_deletions` row (and its audit event), not in the synchronous POST response. A
   synchronous answer would require the append-serving credential to hold `DELETE`, which the
   security design forbids.
3. **The retention worker writes its events directly, not through the API.** The security design
   grants `audit_retention` `INSERT` on `audit_events` for exactly its own run and deletion
   events, so the worker uses that grant. The infrastructure specification's sentence about
   calling `audit-api` outbound was not implemented; direct insert is the narrower path and has
   no extra network dependency.
4. **The retention worker has no `/metrics` endpoint.** It has no inbound listener by design.
   Its health evidence is the tmpfs heartbeat (container health check) plus the
   `retention_runs` table: the age of the newest successful run is the health signal and the
   count of failed runs is the error signal. A Prometheus scrape target would need an inbound
   listener the design forbids.
5. **Two minimal grants added beyond the security design's matrix.** The API records a
   `reported` retention run and a `requested` privacy deletion, so `audit_app` received
   `INSERT` on `retention_runs` and `SELECT, INSERT` on `privacy_deletions`; it has no `UPDATE`
   or `DELETE` on either, so it cannot complete, alter, or remove them. Without these the
   report and request paths would be refused by the database.
6. **Privacy deletion scope is `actor_id = subject`.** Events whose `target` merely mentions the
   user are not selected. Selecting them reliably needs a subject column or a label at append
   time, which the entity model does not have; current scope is the defensible subset.
7. **The seed-phrase dictionary is a representative slice of the BIP-39 English list**, not the
   full 2 048 words. The detector's shape and tests are complete; the list is documented in
   `services/audit/domain/src/secrets.rs` as the place to paste the full list.
8. **The Solana base58 check is a hand-written decoder** (no new dependency) that validates the
   32-byte decoded length.
9. **The eight-hour support-session ceiling is a domain constant**, not read from the
   environment. The design describes it as configured; the value is fixed at eight hours and
   enforced in one place (`audit-domain::support::MAX_SESSION_HOURS`). Wiring it to config is a
   small follow-up.
10. **80% line coverage per crate was not measured in this run.** The tests exist for every layer
    and every test file covers a happy path plus at least two error/edge cases, but no
    `cargo llvm-cov` run was performed, so the team's coverage gate is unverified here.
11. **The API does not run migrations at startup.** `audit_app` holds no DDL grant; the pipeline's
    `audit_migrator` step applies them. Readiness fails until `_sqlx_migrations` exists, which is
    what the deploy smoke test checks.
12. **Rate limits are per replica** (30 queries/min, 5 exports/h, 1 000 appends/min), as the
    security design states; the effective ceiling therefore scales with replica count. The
    gateway remains the primary control.
13. **Export failures have no `failureReason` column** because the `AuditExport` entity has none;
    the reason is logged (redacted by the secret detector in the failure path) and the audit
    event records `outcome: failed`.
14. **The down migration leaves the four roles in place** (cluster bootstrap objects) and refuses
    to drop the trail while a record is inside the retention window. This is deliberate and
    tested.

## Things worth knowing for the next unit

- The emitter side owns retry and circuit breaking (the audit service never retries and never
  queues). The response body never says how important an event is; the class is derived from
  `sensitivity`, and a contradicting `eventClass` is a 422.
- The service-token file is the only place emitting services are registered; rotating a token is
  a secret update and a restart, with no code change.
- `infra/sql/audit-schema.sql` documents the four Vault paths and the Docker secret each feeds.
  No credential is written in the repository.
