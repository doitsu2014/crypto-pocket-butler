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
| `POST /v1/events` | internal service token | 201 with the event id (also on a same-payload retry), 409 on a reused key with a new payload, 422 naming field paths, 503 on storage; `actorKind: system` is refused — only Audit's own write path attributes system events |
| `GET /v1/events` | JWT + `admin` | cursor-paginated on a lossless (nanosecond) `(occurredAt, id)` cursor, newest first, mandatory `recordedAt` range |
| `POST /v1/exports` | JWT + `admin` | the response body **is** the export: JSON Lines streamed from 5 000-row batches, `x-export-id`, sha-256 of the exact bytes recorded on success |
| `POST /v1/support-sessions` | JWT + `support` | reason (10–500 chars), independent second approver, expiry ≤ 8 h; a valid request is stored as `requested`; a refusal stores a `denied` record and answers 403 |
| `POST /v1/support-sessions/{id}/approve` | JWT + `support`, and the subject must equal the stored second approver | moves a requested session to `active`; the requesting agent cannot approve their own session |
| `GET /v1/support-sessions/{id}/check?scope=` | internal service token | `allow` only for an active, unexpired, in-scope session; the read event is written **before** the allow; any failure is a deny |
| `POST /v1/support-sessions/{id}/revoke` | JWT + `admin` | ends an active session; a terminal session is immutable |
| `POST /v1/privacy/deletions` | internal service token issued to **Identity only** | records the request and answers 202; any other registered service gets 403 and its attempt is audited; the retention worker executes it |
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
  written in the same transaction as the event, plus a `created_at` index for the sweep that
  removes rows whose event is past the retention window;
- `support_access_records`, `retention_runs`, `audit_exports`, `privacy_deletions`, each with a
  state-shape check and a terminal-state trigger; `audit_exports` also stores `failure_reason`,
  which carries `replica_lost` when startup recovers a stale request;
- `create_audit_partition(date)` both creates and grants `SELECT, DELETE` on each new child, so a
  partition created after the parent grants is still usable by `audit_retention`;
- `ensure_audit_partition(date)` (SECURITY DEFINER) for pre-creation, and
  `drop_audit_partition(text, timestamptz)` (SECURITY DEFINER) which re-checks the retention
  window and legal holds inside the database before dropping. The retention role therefore never
  needs DROP ownership of a partition, and a raw `DROP TABLE` from the sweep cannot bypass the
  checks;
- insert-shape triggers on `retention_runs` and `privacy_deletions` that narrow `audit_app`'s
  INSERT to a completed `reported` run and a `requested` deletion respectively, so the
  append-serving credential cannot forge an `audit_logs` run or an already-completed deletion;
- `GRANT SELECT ON _sqlx_migrations TO audit_app`, so `/readyz` works under the runtime role;
- roles `audit_app`, `audit_readonly`, `audit_retention`, `audit_migrator` and exactly the grants
  in the security design, plus the two minimal additions noted under deviations. The roles are
  created `NOLOGIN`; the platform's Vault bootstrap owns LOGIN and the passwords.

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

## Verification output (run in this workspace, after the review fixes)

```
$ cargo test --manifest-path services/audit/Cargo.toml
audit domain:        test result: ok. 30 passed; 0 failed; 0 ignored
audit application:   test result: ok. 42 passed; 0 failed; 0 ignored
audit http:          test result: ok. 25 passed; 0 failed; 0 ignored
audit smoke:         test result: ok. 4 passed; 0 failed; 0 ignored
audit repositories:  test result: ok. 0 passed; 0 failed; 14 ignored

$ cargo fmt --all -- --check
(no output, exit 0)

$ cargo clippy --workspace --all-targets -- -D warnings
(no warnings, exit 0)

$ scripts/check-boundaries.sh
boundaries OK (15 packages checked)
```

The 14 adapter tests are `#[ignore]`d because no disposable Postgres is available in this
workspace; they compile, and the command that runs them is in `unit-test-instructions.md`. They
include the new role-scoped tests: `retention_operates_as_the_audit_retention_role`,
`readiness_works_for_the_runtime_role`, `the_runtime_role_cannot_forge_a_running_retention_run`,
and `the_down_migration_refuses_while_a_legal_hold_exists`. They were not executed here, so the
role/partition SQL paths remain unverified by a live run in this environment.

## Gaps and deviations accepted

1. **Three endpoints beyond the dispatch list.** `POST /v1/runs` is required by WF6/BR4.2
   (Ledger reports its own run), `POST /v1/support-sessions/{id}/revoke` by the support-session
   state machine and the security design's admin revocation path, and
   `POST /v1/support-sessions/{id}/approve` by WF2's second-operator approval. All are tested;
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
5. **Two minimal grants beyond the security design's matrix, narrowed at the row level.** The API
   records a `reported` retention run and a `requested` privacy deletion, so `audit_app` received
   `INSERT` on `retention_runs` and `SELECT, INSERT` on `privacy_deletions`; it has no `UPDATE`
   or `DELETE` on either. Insert-shape triggers refuse any INSERT other than a completed
   `reported` run (owner not audit) or a `requested` deletion with zero counts, so the
   append-serving credential cannot forge an `audit_logs` run or a completed deletion. The
   security design records this narrowed grant.
6. **Privacy deletion scope is `actor_id = subject`.** Events whose `target` merely mentions the
   user are not selected. Selecting them reliably needs a subject column or a label at append
   time, which the entity model does not have; current scope is the defensible subset.
7. **The Solana base58 check is a hand-written decoder** (no new dependency) that validates the
   32-byte decoded length. The seed-phrase dictionary is now the full 2 048-word BIP-39 English
   list, with a test that pins the length and recognises a phrase outside the old slice.
8. **The eight-hour support-session ceiling is a domain constant**, not read from the
   environment. The design describes it as configured; the value is fixed at eight hours and
   enforced in one place (`audit-domain::support::MAX_SESSION_HOURS`). Wiring it to config is a
   small follow-up.
9. **80% line coverage per crate was not measured in this run.** The tests exist for every layer
   and every test file covers a happy path plus at least two error/edge cases, but neither
   `cargo llvm-cov` nor tarpaulin is installed, so the team's coverage gate is unverified here.
10. **The API does not run migrations at startup.** `audit_app` holds no DDL grant; the pipeline's
    `audit_migrator` step applies them. Readiness fails until `_sqlx_migrations` exists, which is
    what the deploy smoke test checks.
11. **Rate limits are per replica** (30 queries/min, 5 exports/h, 1 000 appends/min), as the
    security design states; the effective ceiling therefore scales with replica count. The
    gateway remains the primary control.
12. **The down migration leaves the four roles in place** (cluster bootstrap objects), refuses to
    drop the trail while a record is inside the retention window, and now also refuses while any
    row carries a legal basis.
13. **Support-session audit events name both actors through two events.** BR1.4 keeps context to
    identifiers only, so the scope and expiry are not copied into the event's context; they live
    on the session row, which the event names by id. The agent is recorded on
    `support.session.started` and the approver on `support.session.approved`.
14. **The adapter suite was not executed in this workspace** because no disposable Postgres was
    available. The 14 tests compile; the command is in `unit-test-instructions.md`.
15. **The retention sweep removes `append_idempotency` rows past the same cutoff**, and the policy
    is recorded in the security design so the table cannot grow for the life of the schema.

## Review findings addressed

The reviewer's 18 findings in `reviews/review-01.md` are all addressed:

| Finding | Fix |
|---|---|
| R-01 retention cannot read/drop partitions | `create_audit_partition` grants `SELECT, DELETE` on each child; `drop_audit_partition(text, timestamptz)` is SECURITY DEFINER and re-checks the window and legal holds; adapter test runs as `audit_retention` |
| R-02 seed list is a slice | the full 2 048-word BIP-39 English list, pinned by a length test and a phrase test outside the old slice |
| R-03 any service can request a deletion | only the Identity service token is accepted; others get 403 and the attempt is audited; HTTP and application tests |
| R-04 emitters can forge system events | the public append path rejects `actorKind: system`; Audit's own `Trail` still writes system events; application and HTTP tests |
| R-05 readiness cannot read `_sqlx_migrations` | `GRANT SELECT ON _sqlx_migrations TO audit_app` (guarded for a hand-run bootstrap); adapter test runs as `audit_app` |
| R-06 cursor loses sub-millisecond precision | RFC 3339 nanosecond cursor; test with two rows in the same millisecond across a page boundary |
| R-07 unbounded privacy delete then failure | delete exactly the scanned, bounded id set and record the actual count; the count-difference test now expects a completed row |
| R-08 `audit_app` holds retention grants | the INSERTs are narrowed by insert-shape triggers; the security design records the narrowed grant |
| R-09 a session is active on request | two-step `requested` → `active` approval by the named second approver; agent cannot approve their own request; domain, application, and HTTP tests |
| R-10 unknown-session denial was lost | system-attributed, allow-listed `system.support.check_denied`; application test asserts the event |
| R-11 rollback deletes legally held rows | the down migration refuses while any row carries a legal basis; adapter test |
| R-12 roles are NOLOGIN | both SQL files state that the platform Vault bootstrap owns LOGIN and the passwords |
| R-13 stale exports lack a reason | `audit_exports.failure_reason` stores `replica_lost`; application test |
| R-14 started event lacks the approver | a second `support.session.approved` event names the approver; scope/expiry stay on the row (BR1.4) |
| R-15 `outcome: approved` while active | `outcome` is absent while requested or active; domain test |
| R-16 one failed deletion stops the batch | `execute_pending` logs and continues; application test |
| R-17 `append_idempotency` grows without bound | the sweep removes rows past the cutoff; `created_at` index; application test |
| R-18 `.env.example` lacks audit DSN paths | both audit secret paths are shown per service |

## Things worth knowing for the next unit

- The emitter side owns retry and circuit breaking (the audit service never retries and never
  queues). The response body never says how important an event is; the class is derived from
  `sensitivity`, and a contradicting `eventClass` is a 422.
- The service-token file is the only place emitting services are registered; rotating a token is
  a secret update and a restart, with no code change.
- `infra/sql/audit-schema.sql` documents the four Vault paths and the Docker secret each feeds.
  No credential is written in the repository.
