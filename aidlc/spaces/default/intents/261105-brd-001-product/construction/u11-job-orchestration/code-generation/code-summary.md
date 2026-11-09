# Code Summary — u11-job-orchestration

## What was built

The Job Orchestration service as a five-package Cargo workspace under
`services/job-orchestration/`: four Clean Architecture crates plus the composition package with
`job-orchestration-api` (8105) and `job-orchestration-migrate`.

| Package | Responsibility |
|---|---|
| `job-orchestration-domain` | the schedule grammar and canonical window, the fixed 14-job catalogue, `JobRun` with its six-status state machine, triggers, and the smoke allow-list |
| `job-orchestration-application` | the tick planner (due and missed windows, retries, pending drain), the reaper, target-answer classification, operator reads, manual runs and bounded backfill |
| `job-orchestration-infrastructure` | sqlx repositories (runs, lease, heartbeat), the C11 provider client with the per-window idempotency key, the C10 client, JWKS validation reading `app_metadata.role`, the service-token registry, readiness, clock, ids, migrations and the batched retention prune |
| `job-orchestration-http` | operator routes behind a JWT with the admin/support role, the smoke route behind a service token, RFC 7807 problems and Prometheus metrics |
| `job-orchestration` | composition root: one binary serves the API and runs the tick loop under the lease |

### HTTP surface

| Method and path | Identity | Behaviour |
|---|---|---|
| `GET /v1/jobs` | admin/support JWT | the catalogue with each job's latest attempt |
| `GET /v1/jobs/{job}/runs?from=&to=&limit=` | admin/support JWT | a bounded page (≤500), newest first |
| `POST /v1/jobs/{job}/run` | admin/support JWT | queues the current window as a `manual` run; 409 with the covering run when covered |
| `POST /v1/jobs/{job}/backfill` | admin/support JWT | queues every uncovered window in a ≤7-day range; returns accepted and covered counts |
| `GET /v1/internal/smoke` | service token (`smoke`) | release smoke |
| `GET /healthz`, `/readyz`, `/metrics` | public | readiness checks only Postgres |

### Scheduling model

- A 60 s in-process ticker (allowed 30–90 s) runs only on the instance holding the
  `orchestrator_lease` row (30 s TTL, renewed every 10 s).
- Each tick writes the heartbeat at its start and end, reaps expired `running` leases as `failed`,
  queues the current and (inside a 24 h lookback) the previous window per job, queues due retries
  below `max_attempts`, drains at most 200 pending rows, and returns.
- One row per attempt, unique `(job, scheduled_for, attempt)`; a retry inserts attempt+1; terminal
  rows are `succeeded`, `skipped`, `abandoned` and never change.
- Every call sends `POST {provider}/v1/internal/run/{job}` with `{owner}` and
  `idempotency-key: job:{job}:{window}`. Answers: 200 succeeded/failed/skipped, 202 → skipped,
  409 → failed and retried, other 4xx → abandoned, 5xx/timeout → failed and retried.
- Operator manual and backfill runs record the actor, trigger and reason and are audited.

### Database

`services/job-orchestration/migrations/0001_init.up.sql` creates `job_definitions` (seeded with
the catalogue), `job_runs`, `orchestrator_lease`, `heartbeat`, `backfill_requests`, `prune_state`
and `service_token_registry`, with per-table grants: the runtime role has SELECT-only on the
registry and prune state and no DELETE; the migrator holds DDL and the batched delete. The down
migration refuses to drop a schema holding runs.

### Cross-unit changes

- `infra/sql/job-orchestration-service-tokens.sql` seeds the smoke reader; every provider must
  allow-list `job-orchestration` for its C11 run endpoint (recorded as a C11 amendment).
- `infra/prometheus/job-orchestration-rules.yml` carries the heartbeat (180 s, `absent()`),
  abandonment, still-failing, unrecovered-window and prune alerts.
- `infra/swarm-stack.yml` gains the service (2 replicas, 8105, `/readyz` health check, start-first
  rollout, rollback) and four secrets; the migrate DSN mounts only in the migrate job.
- `docs/architecture/README.md` and `docs/architecture/workspace.dsl` replace Supabase Cron with
  the self-ticking leased scheduler and the Prometheus dead-man rule.

## Tests

`cargo test -p job-orchestration-domain -p job-orchestration-application -p job-orchestration-infrastructure -p job-orchestration-http -p job-orchestration`
runs 54 tests (19 domain, 29 application, 4 infrastructure, 2 HTTP) covering the schedule
grammar, the state machine, the tick, lease loss, retries, answer classification, the operator
role check, manual/backfill behaviour and audit. The workspace suite is 1,349 tests, all green;
`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` are
clean. Database adapter tests that need `TEST_DATABASE_URL` remain `#[ignore]`d.
