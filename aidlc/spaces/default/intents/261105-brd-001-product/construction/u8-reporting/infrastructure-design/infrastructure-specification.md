# Infrastructure Specification — u8-reporting

Inputs: `nfr-design/*`, `functional-design/`, `inception/domain-design/components.md`, C1/C3/C7/C8/C9/C10.

## Deployment

| Facet | Choice | Rationale |
|---|---|---|
| Compute | `reporting` (2 replicas, 0.2 vCPU / 192 MiB) and `reporting-worker` (1 replica, 0.5 vCPU / 512 MiB); the worker holds a 1,800 s lease renewed every 60 s and runs the 32-way generation concurrency | API stateless; one leased generation worker |
| Networking | No published port; gateway → `internal`; C3/C7/C8/C9/C10 internal; `public` for Postgres only | Reporting is internal |
| Storage | Supabase Postgres schema `reporting`, roles `reporting_svc` and `reporting_migrator` | One schema; migration separation |
| Environments | Production only | Team decision |
| IaC | `infra/swarm-stack.yml` gains `reporting-api` (2 replicas, 8103, secrets, internal network), `reporting-worker` (1 replica, 8104, stop-first) and `reporting-migrate` (one-shot, migrator DSN) in this unit's code generation, each with the health check and update config below | Infrastructure is code; the stack delta ships with the build |
| Sizing | API pool 10; worker 8; ceiling 4 | 4.2M briefs/year |
| Health | API `/healthz` 8103, worker 8104; `/readyz` Postgres only | Source outages must not fail readiness |
| Update | Rolling, `start-first` API, `stop-first` worker, `failure_action: rollback` | One generation executor |
| Scaling | Manual `scale reporting=N` on list p95 > 200 ms or open p95 > 300 ms for 10 min | No autoscaler |

## Infrastructure Services

| Service | Role | Configuration | Notes |
|---|---|---|---|
| Supabase Postgres `reporting` | Database | briefs (sections JSONB), deliveries; index `(user_id, generated_at desc)`; unique `(user_id, period, period_start)`; the retention prune is the release job's `reporting-migrate --prune-briefs` step, run nightly at 02:00 by the operator job (no in-service scheduler); daily backup, PITR | RPO ≤24 h, RTO ≤4 h |
| `reporting_svc` | Runtime role | DML on briefs/deliveries; no DDL | — |
| `reporting_migrator` | Migration role + prune CLI (`reporting-migrate --prune-briefs`) | DDL only via the migrate job | Not available to services |
| Docker secrets | Bootstrap | `reporting_database_url`, `reporting_migrate_database_url`, `reporting_smoke_token`, `reporting_outbound_token` | `*_FILE` |
| Service-token registry | Auth | Identity (privacy export/anonymise), Job Orchestration (C11 `POST /v1/internal/run/brief-generation`), smoke-reader; seeded/rotated by `infra/sql/reporting-service-tokens.sql`. Reporting's outbound token is seeded by the consuming services instead: Portfolio (`infra/sql/portfolio-service-tokens.sql`, `portfolios_read` + `valuation_read`), Ledger (`infra/sql/ledger-service-tokens.sql`, `pnl.read`), Notification (`infra/sql/notification-service-tokens.sql`, `alerts.submit`), and the mounted JSON token maps of Identity (`/v1/internal/users/due`) and Audit (`/v1/events`) | Seeded/rotated by those files and secrets |
| `internal` overlay | Network | Encrypted; gateway, Identity, Ledger, Portfolio, Notification, Audit, Job Orchestration | — |
| `public` overlay | Network | Postgres only | TLS |
| Loki/Prometheus/Tempo/Grafana | Observability | Existing stack | — |

## Shared Infrastructure

| Resource | Owner | Consumers | Boundary |
|---|---|---|---|
| API Gateway | u1 | u2, u8 | C1 briefs |
| Identity (C3 due users), Ledger (C7 P&L), Portfolio (C8 reads), Notification (C9), Audit (C10) | their units | u8 | service tokens |
| Observability | u1 | all | ingestion |

## Cost and recovery

- Incremental: 2 × 0.2 + 1 × 0.5 vCPU on existing capacity; brief rows ≈ 8.4M over two years within
  the Supabase plan (TBD with the operator).
- Cluster rebuild from the stack + Supabase backup, RTO ≤4 h; rehearsal before launch.

## Non-resources

- No email provider (C9), no queue, no cache.
