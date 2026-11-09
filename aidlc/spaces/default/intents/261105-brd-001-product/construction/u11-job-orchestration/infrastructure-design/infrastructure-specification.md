# Infrastructure Specification — u11-job-orchestration

Inputs: `nfr-design/*`, `functional-design/`, `inception/domain-design/components.md`, C11.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | One `job-orchestration` image, one Swarm service, 2 replicas, 0.25 vCPU / 256 MiB; the OrchestratorLease elects the scheduler | One binary serves operators and schedules (NFR20.1) |
| Networking | Port 8105 on the `internal` overlay (encrypted); no public port; gateway → internal | Operators reach it through the gateway |
| Storage | Supabase Postgres schema `job_orchestration`, roles `job_orchestration_svc` and `job_orchestration_migrator` | One schema; DDL separate |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml` gains `job-orchestration` (2 replicas, 8105, secrets, internal network, health check, start-first rolling update, rollback) in this unit's code generation | Infrastructure is code |
| Sizing | One pool of 10 per process | NFR20.5 |
| Health | Swarm healthcheck probes `/readyz` on 8105 (it includes Postgres), so a task that lost its database cannot pass the rolling-update gate | NFR1.9 |
| Retention scheduler | A systemd timer installed by the runbook on the Swarm manager fires the migrate job at 02:00 UTC; the prune deletes in bounded 50,000-row batches with a 1 s pause, and a weekly `VACUUM (ANALYZE)` follows. The runbook owns installation; `PruneMissed` detects a missed prune | First run ≈ 8.1M rows in ~3 min |
| Update | Rolling `start-first`, `failure_action: rollback` | Brief restart gap; lease failover ≤ 90 s |
| Scaling | Manual `scale job-orchestration=N`; extra replicas serve reads only | Lease elects one scheduler |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `job_orchestration` | Database | `job_definitions`, `job_runs` (unique `(job, scheduled_for, attempt)`), `orchestrator_lease`, `heartbeat`, `backfill_requests`, `prune_state`, `service_token_registry`; indexes `(job, scheduled_for desc)`, `(status, next_attempt_at)`; daily backup, PITR | RPO ≤24 h, RTO ≤4 h |
| `job_orchestration_svc` | Runtime role | SELECT on `service_token_registry` and `prune_state`; SELECT/INSERT/UPDATE on `job_definitions`, `job_runs`, `backfill_requests`, `orchestrator_lease`, `heartbeat`; no DELETE, no DDL (DELETE only for the migrator) | Least privilege, NFR9.7 |
| `job_orchestration_migrator` | Migration role + prune CLI (`job-orchestration-migrate --prune-runs`) | DDL; DELETE for retention | Operator DSN only |
| Docker secrets | Bootstrap | `job_orchestration_database_url` and `job_orchestration_outbound_token` mount in the runtime service only; `job_orchestration_migrate_database_url` mounts only in the migrate/prune job; `job_orchestration_smoke_token` is held by the release job and never mounted | `*_FILE` |
| Service-token registry | Auth | One inbound caller, `smoke-reader`, whose token is `job_orchestration_smoke_token`; seeded by `infra/sql/job-orchestration-service-tokens.sql`. The outbound token is seeded by each provider's C11 seed (account, market-data, portfolio, ledger, analytics, notification, reporting, identity, audit) | NFR9.4 |
| `internal` overlay | Network | Encrypted (`--opt encrypted`); gateway and every provider | NFR9.8 |
| Loki/Prometheus/Tempo/Grafana | Observability | Existing stack; heartbeat and failure alerts in Alertmanager | NFR17 |
| Prometheus alert rules | Alerting | `HeartbeatStale`, `JobRunAbandoned`, `JobStillFailing`, `WindowsUnrecovered`, `PruneMissed` | NFR17.7 |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| API Gateway | u1 | u11 operators | C1 admin routes |
| Providers' C11 run endpoints | their units | u11 | service token + idempotency key |
| Audit (C10) | u12 | u11 | service token |
| Observability + Alertmanager | u1 | all | ingestion; alert routing |

## Cost and recovery

- Incremental: 2 × 0.25 vCPU on existing capacity; run rows ≈ 8.1M at 400 days within the Supabase
  plan (TBD with the operator).
- Cluster rebuild from the stack + Supabase backup; RTO ≤ 4 h.

## Non-resources

- No message queue, no Supabase Cron, no cache, no separate worker service.
